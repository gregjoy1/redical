use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::Calendar;
use crate::redis::datatype::CALENDAR_DATA_TYPE;

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

pub fn redical_calendar_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        ctx.log_debug(format!("rdcl.cal_set: WrongArity: {{args.len()}}").as_str());

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

    let calendar = Calendar::new(calendar_uid.into());

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    Ok(serialize_calendar(&calendar))
}
