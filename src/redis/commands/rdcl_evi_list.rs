use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::serializers::ical_serializer::ICalSerializer;
use crate::core::{Calendar, EventInstanceIterator};

pub fn redical_event_instance_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evi_list: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evi_list: calendar_uid: {calendar_uid} event_uid: {event_uid}").as_str(),
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Ok(RedisValue::Null);
    };

    let Some(event) = calendar.events.get(&String::from(event_uid.clone())) else {
        return Ok(RedisValue::Null);
    };

    let event_instance_iterator = EventInstanceIterator::new(event, None, None, None, None);

    match event_instance_iterator {
        Ok(event_instance_iterator) => {
            let event_instances = event_instance_iterator
                .map(|event_instance| {
                    RedisValue::Array(
                        event_instance
                            .serialize_to_ical(&rrule::Tz::UTC)
                            .iter()
                            .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                            .collect(),
                    )
                })
                .collect();

            Ok(RedisValue::Array(event_instances))
        }

        Err(error) => Err(RedisError::String(error)),
    }
}
