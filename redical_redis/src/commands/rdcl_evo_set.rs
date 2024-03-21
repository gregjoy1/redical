use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use crate::core::{Calendar, CalendarIndexUpdater, EventOccurrenceOverride, InvertedEventIndex};
use crate::datatype::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

use crate::core::ical::parser::datetime::datestring_to_date;

fn serialize_event_occurrence_override(event_occurrence_override: &EventOccurrenceOverride) -> RedisValue {
    RedisValue::Array(
        event_occurrence_override
            .serialize_to_ical(None)
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

    if datestring_to_date(override_date_string, None, "").is_err() {
        return Err(RedisError::String(format!("`{override_date_string}` is not a valid datetime format.")));
    }

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

    println!("rdcl.evo_set: key: {calendar_uid} event uid: {event_uid} - count: {} - DTSTART: {override_date_string} - count: {}", calendar.events.len(), event.overrides.len());

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    // TODO: Revisit keyspace events...
    if ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &calendar_uid)
        == Status::Err
    {
        return Err(RedisError::Str("Generic error"));
    }

    Ok(serialize_event_occurrence_override(&event_occurrence_override))
}
