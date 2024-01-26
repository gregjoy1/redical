use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use crate::core::{Calendar, CalendarIndexUpdater, InvertedEventIndex};
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::ical::parser::datetime::datestring_to_date;

pub fn redical_event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evo_del: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();
    let override_date_string = args.next_arg()?.try_as_str()?;

    let override_timestamp =
        datestring_to_date(override_date_string, None, "")
        .map(|datetime| datetime.timestamp())
        .map_err(|error| RedisError::String(format!("{:#?}", error)))?;

    ctx.log_debug(
        format!("rdcl.evo_del: calendar_uid: {calendar_uid} event_uid: {event_uid} occurrence date string: {override_date_string}").as_str()
    );

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    let Some(mut event) = calendar.events.get(&event_uid).cloned() else {
        return Err(RedisError::String(format!(
            "No event with UID: '{}' found",
            event_uid
        )));
    };

    // Record whether the override removed actually existed for that timestamp or not.
    let was_override_removed = event.remove_occurrence_override(override_timestamp).map_err(RedisError::String)?;

    // HashMap.insert returns the old value (if present) which we can use in diffing old -> new.
    let existing_event = calendar
        .events
        .insert(event_uid.to_owned(), event.to_owned());

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

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    // TODO: Revisit keyspace events...
    if ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &calendar_uid)
        == Status::Err
    {
        return Err(RedisError::Str("Generic error"));
    }

    Ok(RedisValue::Bool(was_override_removed))
}
