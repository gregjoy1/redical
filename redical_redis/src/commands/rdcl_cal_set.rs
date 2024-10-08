use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, RedisValue, Status};

use redical_core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

use redical_ical::ICalendarComponent;

fn serialize_calendar(calendar: &Calendar) -> RedisValue {
    RedisValue::Array(
        calendar
            .to_rendered_content_lines()
            .into_iter()
            .map(RedisValue::SimpleString)
            .collect(),
    )
}

pub fn redical_calendar_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.is_empty() {
        ctx.log_debug(format!("rdcl.cal_set: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    if let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
        ctx.log_debug(format!("rdcl.cal_set: key: {calendar_uid} -- exists: {:#?}", &calendar).as_str());

        return Ok(serialize_calendar(calendar));
    };

    ctx.log_debug(format!("rdcl.cal_set: key: {calendar_uid}").as_str());

    let calendar = Calendar::new(calendar_uid.clone().into());

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    notify_keyspace_event(ctx, &calendar_uid)?;

    Ok(serialize_calendar(&calendar))
}


fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString) -> Result<(), RedisError> {
    let event_message = "rdcl.cal_set";

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message, calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.cal_set\" for calendar: \"{}\"", &calendar_uid)
            )
        );
    }

    Ok(())
}
