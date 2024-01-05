use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status};

use crate::core::{Calendar, CalendarIndexUpdater, InvertedEventIndex};
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

use crate::core::parsers::datetime::datestring_to_date;

pub fn redical_event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evo_del: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?;

    let timestamp = match datestring_to_date(args.next_arg()?.try_as_str()?, None, "") {
        Ok(datetime) => datetime.timestamp(),
        Err(error) => return Err(RedisError::String(format!("{:#?}", error))),
    };

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    ctx.log_debug(format!("rdcl.evo_del: key: {calendar_uid} event uid: {event_uid}").as_str());

    let calendar = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?;

    if calendar.is_none() {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    }

    let mut calendar = calendar.unwrap();

    let event = calendar.events.get_mut(&String::from(event_uid.clone()));

    if event.is_none() {
        return Err(RedisError::String(
            "No event with UID: '{event_uid}' found".to_string(),
        ));
    }

    let mut event = event.unwrap().to_owned();

    match event.remove_occurrence_override(timestamp) {
        Err(error) => return Err(RedisError::String(error)),
        _ => {}
    }

    let existing_event = calendar
        .events
        .insert(String::from(event_uid.clone()), event.to_owned());

    let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
        existing_event
            .clone()
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
            .clone()
            .and_then(|existing_event| existing_event.indexed_geo.clone())
            .as_ref(),
        event.indexed_geo.as_ref(),
    );

    let updated_event_class_diff = InvertedEventIndex::diff_indexed_terms(
        existing_event
            .clone()
            .and_then(|existing_event| existing_event.indexed_class.clone())
            .as_ref(),
        event.indexed_class.as_ref(),
    );

    let mut calendar_index_updater = CalendarIndexUpdater::new(event.uid.clone(), &mut calendar);

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

    if ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &calendar_uid)
        == Status::Err
    {
        return Err(RedisError::Str("Generic error"));
    }

    Ok(timestamp.into())
}
