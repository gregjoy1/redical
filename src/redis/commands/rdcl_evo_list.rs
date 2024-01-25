use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::{Calendar, Event, EventOccurrenceOverride};
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

fn serialize_event_overrides(event: &Event) -> RedisValue {
    RedisValue::Array(
        event
            .overrides
            .values()
            .map(serialize_event_occurrence_override)
            .collect()
    )
}

fn serialize_event_occurrence_override(event_occurrence_override: &EventOccurrenceOverride) -> RedisValue {
    RedisValue::Array(
        event_occurrence_override
            .serialize_to_ical(None)
            .into_iter()
            .map(RedisValue::SimpleString)
            .collect()
    )
}

pub fn redical_event_override_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evo_list: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evo_list: calendar_uid: {calendar_uid} event_uid: {event_uid}").as_str()
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: '{calendar_uid}'"
        )));
    };

    let Some(event) = calendar.events.get(&event_uid) else {
        return Err(RedisError::String(format!(
            "No event with UID: '{event_uid}' found",
        )));
    };

    Ok(serialize_event_overrides(event))
}
