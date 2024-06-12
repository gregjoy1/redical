use std::str::FromStr;

use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use crate::core::{Calendar, CalendarIndexUpdater, EventOccurrenceOverride, InvertedEventIndex};
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

pub fn redical_event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 4 {
        ctx.log_debug(format!("rdcl.evo_set: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();
    let override_date_string = args.next_arg()?.try_as_str()?;

    let Ok(parsed_override_datetime) = DateTime::from_str(override_date_string) else {
        return Err(RedisError::String(format!("`{override_date_string}` is not a valid datetime format.")));
    };

    let other: String = args
        .map(|arg| arg.try_as_str().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(" ")
        .as_str()
        .to_owned();

    ctx.log_debug(
        format!("rdcl.evo_set: calendar_uid: {calendar_uid} event_uid: {event_uid} occurrence date string: {override_date_string} ical: {other}").as_str()
    );

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    let Some(mut event) = calendar.get_event(&event_uid).cloned() else {
        return Err(RedisError::String(format!(
            "No event with UID: '{}' found",
            event_uid
        )));
    };

    let event_occurrence_override = EventOccurrenceOverride::parse_ical(override_date_string, other.as_str()).map_err(RedisError::String)?;

    // Validate new event occurrence override's LAST-MODIFIED property (if provided) is more
    // recent than that on the existing event occurrence override (if present).
    //
    // Only proceed with inserting the newly provided event occurrence override if it is found
    // to have a newer LAST-MODIFIED property than the existing event occurrence override, if
    // not then we skip the insert and return false to signal this to the client.
    if let Some(existing_event_occurrence_override) = event.overrides.get(&parsed_override_datetime.get_utc_timestamp(None)) {
        if event_occurrence_override.last_modified < existing_event_occurrence_override.last_modified {
            ctx.log_debug(
                format!(
                    "rdcl.evo_set: key: {calendar_uid} event uid: {event_uid} - DTSTART: {override_date_string} - skipped due to existing superseding LAST-MODIFIED - existing: {} new: {}",
                    existing_event_occurrence_override.last_modified.to_string(),
                    event_occurrence_override.last_modified.to_string(),
                ).as_str()
            );

            return Ok(RedisValue::Bool(false));
        }
    }

    event.override_occurrence(&event_occurrence_override, calendar.indexes_active.to_owned()).map_err(RedisError::String)?;

    // HashMap.insert returns the old value (if present) which we can use in diffing old -> new.
    let existing_event = calendar.insert_event(event.clone()).and_then(|boxed_event| Some(boxed_event));

    if calendar.indexes_active {

        let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_categories.clone())
                .as_ref(),
            event.indexed_categories.as_ref(),
        );

        let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .clone()
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

        let mut calendar_index_updater = CalendarIndexUpdater::new(&event_uid, calendar);

        calendar_index_updater
            .update_indexed_categories(&updated_event_categories_diff)
            .map_err(|error| RedisError::String(error.to_string()))?;

        calendar_index_updater
            .update_indexed_related_to(&updated_event_related_to_diff)
            .map_err(|error| RedisError::String(error.to_string()))?;

        calendar_index_updater
            .update_indexed_geo(&updated_event_geo_diff)
            .map_err(|error| RedisError::String(error.to_string()))?;

        calendar_index_updater
            .update_indexed_class(&updated_event_class_diff)
            .map_err(|error| RedisError::String(error.to_string()))?;
    }

    ctx.log_debug(
        format!(
            "rdcl.evo_set: key: {calendar_uid} event uid: {event_uid} - count: {} - DTSTART: {override_date_string} - count: {}",
            calendar.events.len(),
            event.overrides.len(),
        ).as_str()
    );

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    notify_keyspace_event(ctx, &calendar_uid, &event_uid, &override_date_string, &event_occurrence_override.last_modified.to_string())?;

    Ok(serialize_event_occurrence_override(&event_occurrence_override))
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, event_uid: &String, override_date_string: &str, last_modified_ical_property: &String) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evo_set:{}:{} {}", event_uid, override_date_string, last_modified_ical_property);

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), &calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.evo_set\" for calendar: \"{}\" event: \"{}\" date string: \"{}\"", &calendar_uid, &event_uid, &override_date_string)
            )
        );
    }

    Ok(())
}
