use std::str::FromStr;

use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::{Calendar, EventOccurrenceOverride};
use crate::datatype::CALENDAR_DATA_TYPE;

use redical_ical::ICalendarComponent;
use redical_ical::values::date_time::DateTime;

fn serialize_event_occurrence_override(event_occurrence_override: &EventOccurrenceOverride) -> RedisValue {
    RedisValue::Array(
        event_occurrence_override
            .to_rendered_content_lines()
            .into_iter()
            .map(RedisValue::SimpleString)
            .collect()
    )
}

pub fn redical_event_override_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evo_get: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();
    let override_date_string = args.next_arg()?.try_as_str()?;

    let override_timestamp =
        DateTime::from_str(override_date_string)
            .map(|datetime| datetime.get_utc_timestamp(None))
            .map_err(RedisError::String)?;

    ctx.log_debug(
        format!("rdcl.evo_get: calendar_uid: {calendar_uid} event_uid: {event_uid} occurrence date string: {override_date_string}").as_str()
    );

    let calendar_key = ctx.open_key(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    let Some(event) = calendar.events.get(&event_uid) else {
        return Err(RedisError::String(format!(
            "No event with UID: '{event_uid}' found",
        )));
    };

    event
        .overrides
        .get(&override_timestamp)
        .map_or(
            Ok(RedisValue::Null),
            |event_occurrence_override| {
                Ok(serialize_event_occurrence_override(event_occurrence_override))
            },
        )
}
