use redis_module::{Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

use crate::data_types::{Event, EVENT_DATA_TYPE};

pub fn redical_event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                => Ok(RedisValue::Null),
        Some(event) => Ok(RedisValue::BulkString(format!("event: {:?}", event)))
    }
}
