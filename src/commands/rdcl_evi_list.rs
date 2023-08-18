use redis_module::{Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

use crate::data_types::{EVENT_DATA_TYPE, Event, OccurrenceIndexValue};

pub fn redical_event_instance_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                    => Ok(RedisValue::Null),
        Some(event) => {

            match event.occurrence_cache {
                Some(ref occurrence_cache) => {
                    Ok(
                        RedisValue::Array(
                            occurrence_cache.iter()
                                            .map(|(timestamp, value)| {
                                                match value {
                                                    OccurrenceIndexValue::Occurrence => RedisValue::SimpleString(format!("{timestamp} - occurrence")),
                                                    OccurrenceIndexValue::Override   => RedisValue::SimpleString(format!("{timestamp} - override")),
                                                }
                                            })
                                            .collect()
                        )
                    )
                },

                None => {
                    Ok(RedisValue::Array(vec![]))
                }
            }
        }
    }
}
