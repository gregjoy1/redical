use redis_module::{redis_module, Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

mod data_types;

use data_types::{EVENT_DATA_TYPE, Event, EventOccurrenceOverride};

fn args_test<'a>(_: &Context, args: Vec<RedisString>) -> RedisResult {
    let response = args.into_iter().map(|arg| RedisValue::SimpleString(arg.to_string())).collect();

    Ok(RedisValue::Array(response))
}

fn event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    match Event::parse_ical(key_name.try_as_str()?,other.as_str()) {
        Ok(mut event) => {

            match event.rebuild_occurrence_cache(1000) {
                Ok(event) => {
                    key.set_value(&EVENT_DATA_TYPE, event.clone())?;
                },
                Err(error) => {
                    return Err(RedisError::String(error.to_string()));
                }
            }

            Ok(other.into())
        },
        Err(error) => {
            Err(RedisError::String(error))
        }
    }
}

fn event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                    => Ok(RedisValue::Null),
        Some(event) => Ok(RedisValue::BulkString(format!("event: {:#?}", event)))
    }
}

fn event_occurrences_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
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
                                       .map(|(timestamp, _)| RedisValue::Integer(timestamp))
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

fn event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let timestamp = args.next_i64()?;

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    if let Some(event) = key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        match EventOccurrenceOverride::parse_ical(other.as_str()) {
            Ok(event_occurrence_override) => {

                match event.override_occurrence(timestamp, &event_occurrence_override) {
                    Ok(updated_event) => {
                        key.set_value(&EVENT_DATA_TYPE, updated_event)?;
                    },

                    Err(error) => {
                        return Err(RedisError::String(error));
                    }
                }

                Ok(other.into())
            },

            Err(error) => {
                Err(RedisError::String(error))
            }
        }

    } else {
        Err(RedisError::String("No event with UUID: '{key}' found".to_string()))
    }
}

fn event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let timestamp = args.next_i64()?;

    if let Some(event) = key.get_value::<Event>(&EVENT_DATA_TYPE)? {

        match event.remove_occurrence_override(timestamp) {
            Ok(updated_event) => {
                key.set_value(&EVENT_DATA_TYPE, updated_event)?;
            },

            Err(error) => {
                return Err(RedisError::String(error));
            }
        }

        Ok(timestamp.into())

    } else {
        Err(RedisError::String("No event with UUID: '{key}' found".to_string()))
    }
}

pub const MODULE_NAME:    &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

#[cfg(not(test))]
redis_module! {
    name:       MODULE_NAME,
    version:    MODULE_VERSION,
    allocator:  (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [
        EVENT_DATA_TYPE
    ],
    commands:   [
        ["args.test",              args_test, "", 0, 0, 0],
        ["event.set",              event_set, "", 0, 0, 0],
        ["event.get",              event_get, "", 0, 0, 0],
        ["event.occurrences_list", event_occurrences_list, "", 0, 0, 0],
        ["event.override_set",     event_override_set, "", 0, 0, 0],
        ["event.override_del",     event_override_del, "", 0, 0, 0],
    ],
}
