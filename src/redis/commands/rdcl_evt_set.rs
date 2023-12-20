use redis_module::{Context, NextArg, RedisError, RedisResult, RedisString};

use crate::core::{
    rebase_overrides, Calendar, CalendarIndexUpdater, Event, EventDiff, InvertedEventIndex,
};
use crate::redis::calendar_data_type::CALENDAR_DATA_TYPE;

pub fn redical_event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // TODO: Add option to "rebase" overrides against changes, i.e. add/remove all
    // base added/removed properties to all overrides.
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evt_set: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uuid);

    let other: String = args
        .map(|arg| arg.try_as_str().unwrap_or(""))
        .collect::<Vec<&str>>()
        .join(" ")
        .as_str()
        .to_owned();

    ctx.log_debug(
        format!("rdcl.evt_set: key: {calendar_uuid} event uuid: {event_uuid}, other: {other}")
            .as_str(),
    );

    let mut event =
        Event::parse_ical(event_uuid.try_as_str()?, other.as_str()).map_err(RedisError::String)?;

    let calendar = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?;

    if calendar.is_none() {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uuid}"
        )));
    }

    let mut calendar = calendar.unwrap().to_owned();

    let existing_event = calendar
        .events
        .get(&String::from(event_uuid.clone()))
        .and_then(|event| Some(event.clone()));

    let event_diff = if existing_event.is_some() {
        EventDiff::new(&existing_event.as_ref().unwrap(), &event)
    } else {
        EventDiff::new(&Event::new(event.uuid.clone()), &event)
    };

    if existing_event.is_some() {
        let existing_event = &existing_event.clone().unwrap();

        event.overrides = existing_event.overrides.clone();

        rebase_overrides(&mut event.overrides, &event_diff)
            .map_err(|error| RedisError::String(error.to_string()))?;
    }

    event
        .schedule_properties
        .build_parsed_rrule_set()
        .map_err(|error| RedisError::String(error.to_string()))?;

    event
        .rebuild_indexed_categories()
        .map_err(RedisError::String)?;
    event
        .rebuild_indexed_related_to()
        .map_err(RedisError::String)?;
    event.rebuild_indexed_geo().map_err(RedisError::String)?;

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

    let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), &mut calendar);

    calendar_index_updater
        .update_indexed_categories(&updated_event_categories_diff)
        .map_err(|error| RedisError::String(error.to_string()))?;
    calendar_index_updater
        .update_indexed_related_to(&updated_event_related_to_diff)
        .map_err(|error| RedisError::String(error.to_string()))?;
    calendar_index_updater
        .update_indexed_geo(&updated_event_geo_diff)
        .map_err(|error| RedisError::String(error.to_string()))?;

    calendar.events.insert(String::from(event_uuid), event);

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    Ok(other.into())
}
