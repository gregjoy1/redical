use redis_module::{Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

use crate::data_types::{CALENDAR_DATA_TYPE, Calendar, EventInstanceIterator};

pub fn redical_event_instance_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evi_list: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uuid);

    ctx.log_debug(format!("rdcl.evi_list: calendar_uuid: {calendar_uuid} event_uuid: {event_uuid}").as_str());

    if let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        if let Some(event) = calendar.events.get(&String::from(event_uuid.clone())) {
            return Ok(
                RedisValue::Array(
                    EventInstanceIterator::new(event, None).map(|event_instance| {
                        RedisValue::Array(
                            event_instance.serialize_to_ical()
                                 .iter()
                                 .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                                 .collect()
                        )
                    })
                    .collect()
                )
            );
        }
    }

    Ok(RedisValue::Null)
}
