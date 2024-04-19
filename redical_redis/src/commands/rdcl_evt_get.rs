use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

use redical_ical::ICalendarComponent;

pub fn redical_event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evt_get: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evt_get: calendar_uid: {calendar_uid} event_uid: {event_uid}").as_str()
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    calendar
        .events
        .get(&event_uid)
        .map_or(
            Ok(RedisValue::Null),
            |event| {
                Ok(
                    RedisValue::Array(
                        event
                            .to_rendered_content_lines()
                            .into_iter()
                            .map(RedisValue::SimpleString)
                            .collect(),
                    )
                )
            },
        )
}
