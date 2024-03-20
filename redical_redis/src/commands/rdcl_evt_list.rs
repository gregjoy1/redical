use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::{Calendar, Event};
use crate::redis::datatype::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

fn serialize_calendar_events(calendar: &Calendar, offset: usize, count: usize) -> RedisValue {
    RedisValue::Array(
        calendar.events
                .values()
                .skip(offset)
                .take(count)
                .map(serialize_event)
                .collect()
    )
}

fn serialize_event(event: &Box<Event>) -> RedisValue {
    RedisValue::Array(
        event
            .serialize_to_ical(None)
            .into_iter()
            .map(RedisValue::SimpleString)
            .collect()
    )
}

pub fn redical_event_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        ctx.log_debug(format!("rdcl.evt_list: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let offset = args.next_u64().unwrap_or(0) as usize;
    let count = args.next_u64().unwrap_or(50) as usize;

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evt_list: calendar_uid: {calendar_uid}").as_str()
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    Ok(serialize_calendar_events(calendar, offset, count))
}
