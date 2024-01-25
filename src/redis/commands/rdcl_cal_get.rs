use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

fn serialize_calendar(calendar: &Calendar) -> RedisValue {
    RedisValue::Array(
        calendar
            .serialize_to_ical(None)
            .into_iter()
            .map(RedisValue::SimpleString)
            .collect(),
    )
}

pub fn redical_calendar_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        ctx.log_debug(format!("rdcl.cal_get: WrongArity: {{args.len()}}").as_str());

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
