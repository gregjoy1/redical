use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue, ThreadSafeContext};

use std::str::FromStr;

use redical_ical::{ICalendarComponent, RenderingContext};
use crate::core::queries::query::Query;
use crate::utils::{run_with_timeout, TimeoutError};
use crate::CONFIGURATION_ICAL_PARSER_TIMEOUT_MS;
use redical_core::Calendar;
use crate::datatype::CALENDAR_DATA_TYPE;

fn icalendar_component_to_redis_value_array<I: ICalendarComponent>(component: &I, rendering_context: &RenderingContext) -> RedisValue {
    RedisValue::Array(
        component
            .to_rendered_content_lines_with_context(Some(rendering_context))
            .iter()
            .map(|ical_part| RedisValue::SimpleString(ical_part.to_owned()))
            .collect()
    )
}

pub fn redical_calendar_query(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug("rdcl.cal_query: event_set WrongArity: {{args.len()}}");

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?.cloned() else {
        return Err(RedisError::String(format!(
            "rdcl.cal_query: No Calendar found on key: {calendar_uid}"
        )));
    };

    if !calendar.indexes_active {
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

    let ical_parser_timeout_ms = *CONFIGURATION_ICAL_PARSER_TIMEOUT_MS.lock(ctx) as u64;

    let calendar_uid = calendar_uid.to_string();

    let blocked_client = ctx.block_client();

    std::thread::spawn(move || {
        let thread_ctx = ThreadSafeContext::with_blocked_client(blocked_client);

        // Spawn the process of parsing the query into it's own timeout enforced thread to guard
        // against malicious payloads intended to cause hangs.
        let mut parsed_query =
            match run_with_timeout(
                move || Query::from_str(query_string.as_str()).map_err(RedisError::String),
                std::time::Duration::from_millis(ical_parser_timeout_ms),
            ) {
                Ok(parser_result) => {
                    match parser_result {
                        Ok(parser_result) => parser_result,

                        Err(parser_error) => {
                            thread_ctx.reply(Err(parser_error));

                            return;
                        }
                    }
                },

                Err(TimeoutError) => {
                    thread_ctx.lock().log_warning(
                        format!(
                            "rdcl.cal_query: query iCal parser exceeded timeout -- calendar_uid: {calendar_uid}",
                        ).as_str()
                    );

                    thread_ctx.reply(Err(RedisError::String(String::from("rdcl.cal_query: query iCal parser exceeded timeout"))));

                    return;
                },
            };

        thread_ctx.lock().log_debug(
            format!(
                "rdcl.cal_query: calendar_uid: {calendar_uid} parsed query: {:#?}",
                parsed_query
            ).as_str(),
        );

        let query_results = match parsed_query.execute(&calendar) {
            Ok(results) => results,

            Err(error) => {
                thread_ctx.reply(Err(RedisError::String(error)));

                return;
            },
        };

        // TODO: Clean up and properly serialize this griminess
        let query_result_items = query_results
            .results
            .iter()
            .map(|query_result| {
                let rendering_context = RenderingContext {
                    tz: Some(parsed_query.in_timezone.to_owned()),
                    distance_unit: None,
                };

                RedisValue::Array(vec![
                    icalendar_component_to_redis_value_array(&query_result.result_ordering, &rendering_context),
                    icalendar_component_to_redis_value_array(&query_result.event_instance, &rendering_context),
                ])
            })
            .collect();

        thread_ctx.reply(
            Ok(RedisValue::Array(query_result_items))
        );
    });

    // We will reply later, from the thread
    Ok(RedisValue::NoReply)
}
