use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

pub fn redical_event_keys(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.is_empty() {
        ctx.log_debug(format!("rdcl.evt_keys: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evt_keys: calendar_uid: {calendar_uid}").as_str()
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    Ok(
        RedisValue::Array(
            calendar.events
                    .keys()
                    .cloned()
                    .map(|key| RedisValue::SimpleString(key))
                    .collect()
        )
    )
}
