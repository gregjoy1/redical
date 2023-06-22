use redis_module::{redis_module, Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

mod data_types;

use data_types::{EVENT_DATA_TYPE, Event};

#[macro_use]
extern crate redis_module;

pub const MODULE_NAME:    &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

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
        Ok(event) => {
            key.set_value(&EVENT_DATA_TYPE, event)?;

            Ok(other.into())
        }
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

    let response = match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                    => Ok(RedisValue::Null),
        Some(event) => Ok(RedisValue::BulkString(format!("event: {:?}", event)))
    };

    response
}

redis_module! {
    name:       MODULE_NAME,
    version:    MODULE_VERSION,
    data_types: [
        EVENT_DATA_TYPE
    ],
    commands:   [
        ["args.test", args_test, "", 0, 0, 0],
        ["event.set", event_set, "", 0, 0, 0],
        ["event.get", event_get, "", 0, 0, 0],
    ],
}
