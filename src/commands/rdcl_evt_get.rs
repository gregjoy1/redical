use redis_module::{Context, NextArg, RedisValue, RedisResult, RedisString, RedisError};

use crate::data_types::{Calendar, CALENDAR_DATA_TYPE};

pub fn redical_event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evi_list: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uuid);

    ctx.log_debug(format!("rdcl.evi_list: calendar_uuid: {calendar_uuid} event_uuid: {event_uuid}").as_str());

    if let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        if let Some(event) = calendar.events.get(&String::from(event_uuid.clone())) {
            return Ok(RedisValue::BulkString(format!("event: {:?}", event)));
        }
    }

    Ok(RedisValue::Null)
}
