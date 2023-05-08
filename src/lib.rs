#[macro_use]
extern crate redis_module;

use redis_module::{redis_module, Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

use data_types::{EVENT_DATA_TYPE, Event};

mod data_types;

pub const MODULE_NAME:    &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

fn event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;
    let other = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    if let Some(event) = key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        event.other = other.to_string();
    } else {
        let event = Event {
            uuid:  key_name.to_string(),
            other: other.to_string()
        };

        key.set_value(&EVENT_DATA_TYPE, event)?;
    }

    Ok(other.into())
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
        Some(event) => Ok(RedisValue::BulkString(event.other.clone()))
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
        ["event.set", event_set, "", 0, 0, 0],
        ["event.get", event_get, "", 0, 0, 0],
    ],
}
