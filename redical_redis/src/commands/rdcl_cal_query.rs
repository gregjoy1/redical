use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use std::str::FromStr;

use redical_ical::{ICalendarComponent, RenderingContext};
use crate::core::queries::query::Query;
use crate::utils::{run_with_timeout, TimeoutError};
use redical_core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

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

    if calendar.indexes_active == false {
        return Err(RedisError::String(format!(
            "rdcl.cal_query: Queries disabled on Calendar: {calendar_uid} because it's indexes have been disabled."
        )));
    }

    ctx.log_debug(format!("rdcl.cal_query: calendar_uid: {calendar_uid}").as_str());

    let query_string: String = args
        .map(|arg| arg.try_as_str().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(" ")
        .as_str()
        .to_owned();

    // Spawn the process of parsing the query into it's own timeout enforced thread to guard
    // against malicious payloads intended to cause hangs.
    let mut parsed_query =
        match run_with_timeout(
            move || Query::from_str(query_string.as_str()).map_err(RedisError::String),
            std::time::Duration::from_millis(250),
        ) {
            Ok(parser_result) => {
                parser_result?
            },

            Err(TimeoutError) => {
                ctx.log_warning(
                    format!(
                        "rdcl.cal_query: query iCal parser exceeded timeout -- calendar_uid: {calendar_uid}",
                    ).as_str()
                );

                return Err(RedisError::String(format!(
                    "rdcl.cal_query: query iCal parser exceeded timeout"
                )));
            },
        };

    ctx.log_debug(
        format!(
            "rdcl.cal_query: calendar_uid: {calendar_uid} parsed query: {:#?}",
            parsed_query
        ).as_str(),
    );

    let query_results = parsed_query
        .execute(calendar)
        .map_err(|error| RedisError::String(error))?;

    // TODO: Clean up and properly serialize this grimeyness
    let query_result_items = query_results
        .results
        .iter()
        .map(|query_result| {
            let rendering_context = RenderingContext {
                tz: Some(parsed_query.in_timezone.to_owned()),
                distance_unit: None,
            };

            RedisValue::Array(vec![
                RedisValue::Array(
                    query_result
                        .result_ordering
                        .to_rendered_content_lines_with_context(Some(&rendering_context))
                        .iter()
                        .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                        .collect(),
                ),
                RedisValue::Array(
                    query_result
                        .event_instance
                        .to_rendered_content_lines_with_context(Some(&rendering_context))
                        .iter()
                        .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
                        .collect(),
                ),
            ])
        })
        .collect();

    Ok(RedisValue::Array(query_result_items))
}
