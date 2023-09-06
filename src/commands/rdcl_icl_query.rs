use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError, RedisValue};

use crate::data_types::{CALENDAR_DATA_TYPE, Calendar};

pub fn redical_indexed_calendar_query(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    // Queries either on Event level or Event Instances level.
    let query_subject = args.next_arg()?;

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        Some(calendar) => {
            // TODO: run query...

            Ok(RedisValue::BulkString(format!("calendar already exists with UUID: {:?} - {:?}", calendar.uuid, calendar)))
        },

        None => Ok(RedisValue::Null),
    }

}
