use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, RedisValue, Status};

use crate::core::{
    Calendar, CalendarIndexUpdater, Event, InvertedEventIndex,
};

use crate::datatype::CALENDAR_DATA_TYPE;

use crate::utils::{run_with_timeout, TimeoutError};
use crate::CONFIGURATION_ICAL_PARSER_TIMEOUT_MS;

use redical_ical::ICalendarComponent;

pub fn redical_event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evt_set: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let other = args
        .map(|arg| arg.try_as_str().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(" ")
        .as_str()
        .to_owned();

    ctx.log_debug(
        format!("rdcl.evt_set: key: {calendar_uid} event uid: {event_uid}, other: {other}")
            .as_str(),
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    // Clone the event_uid for it to moved into the timeout enforced Event iCalendar parser thread
    // below.
    let parsed_event_uid = event_uid.clone();

    // Spawn the process of parsing the query into it's own timeout enforced thread to guard
    // against malicious payloads intended to cause hangs.
    let mut event =
        match run_with_timeout(
            move || Event::parse_ical(parsed_event_uid.as_str(), other.as_str()).map_err(RedisError::String),
            std::time::Duration::from_millis(*CONFIGURATION_ICAL_PARSER_TIMEOUT_MS.lock(ctx) as u64),
        ) {
            Ok(parser_result) => {
                parser_result?
            },

            Err(TimeoutError) => {
                ctx.log_warning(
                    format!(
                        "rdcl.evt_set: event iCal parser exceeded timeout -- calendar uid: {calendar_uid} event uid: {event_uid}",
                    ).as_str()
                );

                return Err(RedisError::String(String::from(
                    "rdcl.evt_set: event iCal parser exceeded timeout"
                )));
            },
        };

    event.validate().map_err(RedisError::String)?;

    let existing_event =
        calendar
            .events
            .get(&event_uid)
            .cloned();

    // Validate new event's LAST-MODIFIED property (if provided) is more recent than that on the
    // existing event.
    if let Some(existing_event) = existing_event.as_ref() {
        if event.last_modified < existing_event.last_modified {
            ctx.log_debug(
                format!(
                    "rdcl.evt_set: key: {calendar_uid} event uid: {event_uid} - skipped due to existing superseding LAST-MODIFIED - existing: {} new: {}",
                    existing_event.last_modified.to_string(),
                    event.last_modified.to_string(),
                ).as_str()
            );

            return Ok(RedisValue::Bool(false));
        }
    }

    if let Some(existing_event) = existing_event.as_ref() {
        event.overrides = existing_event.overrides.clone();
    }

    if calendar.indexes_active {
        event.rebuild_indexes().map_err(RedisError::String)?;

        let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_categories.clone())
                .as_ref(),
            event.indexed_categories.as_ref(),
        );

        let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_related_to.clone())
                .as_ref(),
            event.indexed_related_to.as_ref(),
        );

        let updated_event_geo_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_geo.clone())
                .as_ref(),
            event.indexed_geo.as_ref(),
        );

        let updated_event_class_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_class.clone())
                .as_ref(),
            event.indexed_class.as_ref(),
        );

        let mut calendar_index_updater =
            CalendarIndexUpdater::new(&event_uid, calendar);

        calendar_index_updater
            .update_indexed_categories(&updated_event_categories_diff)
            .map_err(RedisError::String)?;

        calendar_index_updater
            .update_indexed_related_to(&updated_event_related_to_diff)
            .map_err(RedisError::String)?;

        calendar_index_updater
            .update_indexed_geo(&updated_event_geo_diff)
            .map_err(RedisError::String)?;

        calendar_index_updater
            .update_indexed_class(&updated_event_class_diff)
            .map_err(RedisError::String)?;
    }

    let serialized_event_ical = event.to_rendered_content_lines();

    ctx.log_debug(
        format!(
            "rdcl.evt_set: key: {calendar_uid} event uid: {event_uid} - count: {}",
            calendar.events.len(),
        ).as_str()
    );

    let last_modified_ical_property = event.last_modified.to_string();

    calendar.insert_event(event);

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    notify_keyspace_event(ctx, &calendar_uid, &event_uid, &last_modified_ical_property)?;

    Ok(
        RedisValue::Array(
            serialized_event_ical
                .into_iter()
                .map(RedisValue::SimpleString)
                .collect(),
        )
    )
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, event_uid: &String, last_modified_ical_property: &String) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evt_set:{} {}", event_uid, last_modified_ical_property);

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.evt_set\" for calendar: \"{}\" event: \"{}\"", &calendar_uid, &event_uid)
            )
        );
    }

    Ok(())
}
