use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue, NotifyEvent, Status};

use redical_core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

pub fn redical_calendar_idx_rebuild(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.is_empty() {
        ctx.log_debug(format!("rdcl.cal_idx_rebuild: WrongArity: {}", args.len()).as_str());

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

    calendar.rebuild_indexes().map_err(RedisError::String)?;

    notify_keyspace_event(ctx, &calendar_uid)?;

    ctx.log_debug(format!("rdcl.cal_idx_rebuild: key: {calendar_uid}").as_str());

    Ok(RedisValue::Bool(true))
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString) -> Result<(), RedisError> {
    let event_message = "rdcl.cal_idx_rebuild";

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message, calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.cal_idx_rebuild\" for calendar: \"{}\"", &calendar_uid)
            )
        );
    }

    Ok(())
}
