use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
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

pub fn redical_calendar_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.is_empty() {
        ctx.log_debug(format!("rdcl.cal_get: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    ctx.log_debug(format!("rdcl.cal_get: key: {calendar_uid}").as_str());

    calendar_key
        .get_value::<Calendar>(&CALENDAR_DATA_TYPE)?
        .map_or(
            Ok(RedisValue::Null),
            |calendar| {
                Ok(serialize_calendar(calendar))
            },
        )
}
