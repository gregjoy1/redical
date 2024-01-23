use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::ical::serializer::{SerializableICalComponent, SerializationPreferences};
use crate::core::queries::query::Query;
use crate::core::Calendar;
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

pub fn redical_calendar_query(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.cal_query: event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "rdcl.cal_query: No Calendar found on key: {calendar_uid}"
        )));
    };

    ctx.log_debug(format!("rdcl.cal_query: calendar_uid: {calendar_uid}").as_str());

    let query_string: String = args
        .map(|arg| arg.try_as_str().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(" ")
        .as_str()
        .to_owned();

    let mut parsed_query =
        Query::try_from(query_string.as_str()).map_err(|error| RedisError::String(error))?;

    ctx.log_debug(
        format!(
            "rdcl.cal_query: calendar_uid: {calendar_uid} parsed query: {:#?}",
            parsed_query
        )
        .as_str(),
    );

    let query_results = parsed_query
        .execute(calendar)
        .map_err(|error| RedisError::String(error))?;

    // TODO: Clean up and properly serialize this grimeyness
    let query_result_items = query_results
        .results
        .iter()
        .map(|query_result| {
            let serialization_preferences = SerializationPreferences::from(&parsed_query);

            RedisValue::Array(vec![
                RedisValue::Array(
                    query_result
                        .result_ordering
                        .serialize_to_ical(Some(&serialization_preferences))
                        .iter()
                        .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                        .collect(),
                ),
                RedisValue::Array(
                    query_result
                        .event_instance
                        .serialize_to_ical(Some(&serialization_preferences))
                        .iter()
                        .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                        .collect(),
                ),
            ])
        })
        .collect();

    Ok(RedisValue::Array(query_result_items))
}
