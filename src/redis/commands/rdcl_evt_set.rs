use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString, RedisValue};

use crate::core::{
    rebase_overrides, Calendar, CalendarIndexUpdater, Event, EventDiff, InvertedEventIndex,
};
use crate::redis::datatype::CALENDAR_DATA_TYPE;

use crate::core::ical::serializer::SerializableICalComponent;

pub fn redical_event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // TODO: Add option to "rebase" overrides against changes, i.e. add/remove all
    // base added/removed properties to all overrides.
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evt_set: WrongArity: {{args.len()}}").as_str());

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

    let mut event =
        Event::parse_ical(event_uid.as_str(), other.as_str()).map_err(RedisError::String)?;

    event.validate();

    let existing_event =
        calendar
            .events
            .get(&event_uid)
            .cloned();

    let event_diff = if let Some(existing_event) = existing_event.as_ref() {
        EventDiff::new(existing_event, &event)
    } else {
        EventDiff::new(&Event::new(event.uid.clone().into()), &event)
    };

    if let Some(existing_event) = existing_event.as_ref() {
        event.overrides = existing_event.overrides.clone();

        rebase_overrides(&mut event.overrides, &event_diff)
            .map_err(RedisError::String)?;
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

    let serialized_event_ical = event.serialize_to_ical(None);

    calendar.events.insert(String::from(event_uid), event);

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    Ok(
        RedisValue::Array(
            serialized_event_ical
                .into_iter()
                .map(RedisValue::SimpleString)
                .collect(),
        )
    )
}
