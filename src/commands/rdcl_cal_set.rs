use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError, RedisValue};

use crate::data_types::{CALENDAR_DATA_TYPE, Calendar};

pub fn redical_calendar_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        ctx.log_debug(format!("rdcl.cal_set: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    ctx.log_debug(format!("rdcl.cal_set: key: {key_name}").as_str());

    match key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        Some(calendar) => {
            Ok(RedisValue::BulkString(format!("calendar already exists with UUID: {:?} - {:?}", calendar.uuid, calendar)))
        },

        None => {
            let new_calendar = Calendar::new(String::from(key_name));

            key.set_value(&CALENDAR_DATA_TYPE, new_calendar.clone())?;

            Ok(RedisValue::BulkString(format!("calendar added with UUID: {:?} - {:?}", new_calendar.uuid, new_calendar)))
        }
    }

}
