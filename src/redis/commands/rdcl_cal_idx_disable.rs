use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::redis::datatype::CALENDAR_DATA_TYPE;

pub fn redical_calendar_idx_disable(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        ctx.log_debug(format!("rdcl.cal_idx_disable: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    if calendar.indexes_active == false {
        ctx.log_debug(format!("rdcl.cal_idx_disable: key: {calendar_uid} skipped - already disabled").as_str());

        return Ok(RedisValue::Bool(false));
    }

    calendar.disable_indexes();

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    ctx.log_debug(format!("rdcl.cal_idx_disable: key: {calendar_uid}").as_str());

    Ok(RedisValue::Bool(true))
}
