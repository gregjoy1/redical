use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

pub fn redical_event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evt_get: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evt_get: calendar_uid: {calendar_uid} event_uid: {event_uid}").as_str(),
    );

    if let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        if let Some(event) = calendar.events.get(&String::from(event_uid.clone())) {
            return Ok(
                RedisValue::Array(
                    event
                        .serialize_to_ical(None)
                        .iter()
                        .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                        .collect(),
                )
            );
        }
    }

    Ok(RedisValue::Null)
}
