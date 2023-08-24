use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError};

use crate::data_types::{EVENT_DATA_TYPE, CALENDAR_DATA_TYPE, Event, Calendar, CalendarIndexUpdater, UpdatedSetMembers, CalendarCategoryIndexUpdater, CalendarRelatedToIndexUpdater, EventDiff};

pub fn redical_event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // TODO: Add option to "rebase" overrides against changes, i.e. add/remove all
    // base added/removed properties to all overrides.
    if args.len() < 2 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    let mut event = Event::parse_ical(key_name.try_as_str()?, other.as_str()).map_err(RedisError::String)?;

    let mut connected_calendars: Vec<Box<Calendar>> = vec![];
    let mut disconnected_calendars: Vec<Box<Calendar>> = vec![];

    let existing_event = key.get_value::<Event>(&EVENT_DATA_TYPE)?;

    if existing_event.is_some() {
        let existing_event = existing_event.as_ref().unwrap();

        // TODO: use this.
        let event_diff = EventDiff::new(&existing_event, &event);
        let _rebased_overrides = existing_event.overrides.clone().rebase_overrides(&event_diff);

        let updated_connected_calendars = UpdatedSetMembers::new(
            existing_event.indexed_properties.get_indexed_calendars().as_ref(),
            event.indexed_properties.get_indexed_calendars().as_ref()
        );

        for disconnected_calendar_uuid in updated_connected_calendars.removed.iter() {
            let key = ctx.open_key(&RedisString::create(None, disconnected_calendar_uuid.as_bytes()));

            if let Some(disconnected_calendar) = key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
                // TODO: find a way to not clone Calendar and use the mutable pointer/ref instead
                disconnected_calendars.push(Box::new(disconnected_calendar.clone()));
            } else {
                // TODO: log that disconnected calendar no longer exists (which is fine
                // because nothing needs to be done).
            }
        }

        for connected_calendar_uuid in updated_connected_calendars.all_present_members().iter() {
            let key = ctx.open_key(&RedisString::create(None, connected_calendar_uuid.as_bytes()));

            if let Some(connected_calendar) = key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
                // TODO: find a way to not clone Calendar and use the mutable pointer/ref instead
                connected_calendars.push(Box::new(connected_calendar.clone()));
            } else {
                return Err(RedisError::String(format!("Indexed calendar with UUID: {connected_calendar_uuid} is not present.")));
            }
        }
    } else if let Some(indexed_calendars) = event.indexed_properties.get_indexed_calendars() {
        for connected_calendar_uuid in indexed_calendars.iter() {
            let key = ctx.open_key(&RedisString::create(None, connected_calendar_uuid.as_bytes()));

            if let Some(connected_calendar) = key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? {
                // TODO: find a way to not clone Calendar and use the mutable pointer/ref instead
                connected_calendars.push(Box::new(connected_calendar.clone()));
            } else {
                return Err(RedisError::String(format!("Indexed calendar with UUID: {connected_calendar_uuid} is not present.")));
            }
        }
    }

    let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), connected_calendars, disconnected_calendars);

    if existing_event.is_some() {
        let existing_event = existing_event.unwrap();

        if calendar_index_updater.is_any_disconnected_calendars() {
            CalendarCategoryIndexUpdater::new(&mut calendar_index_updater).remove_event_from_calendar(existing_event).map_err(RedisError::String)?;
            CalendarRelatedToIndexUpdater::new(&mut calendar_index_updater).remove_event_from_calendar(existing_event).map_err(RedisError::String)?;
        }

        // TODO: diff the existing categories and related_to and update those which need to be
        // updated.
    } else {
        event.rebuild_indexed_categories(&mut calendar_index_updater).map_err(RedisError::String)?;
        event.rebuild_indexed_related_to(&mut calendar_index_updater).map_err(RedisError::String)?;
    }

    // TODO: Check if it needs to be updated or not
    event.rebuild_occurrence_cache(1000).map_err(|error| RedisError::String(error.to_string()))?;

    key.set_value(&EVENT_DATA_TYPE, event.clone())?;

    for connected_calendar in calendar_index_updater.connected_calendars.iter() {
        let key = ctx.open_key_writable(&RedisString::create(None, connected_calendar.uuid.as_bytes()));

        // TODO: handle error - with rollback?! + find a way to not clone Calendar and use the mutable pointer/ref instead
        key.set_value(&CALENDAR_DATA_TYPE, *connected_calendar.clone())?;
    }

    for disconnected_calendar in calendar_index_updater.disconnected_calendars.iter() {
        let key = ctx.open_key_writable(&RedisString::create(None, disconnected_calendar.uuid.as_bytes()));

        // TODO: handle error - with rollback?! + find a way to not clone Calendar and use the mutable pointer/ref instead
        key.set_value(&CALENDAR_DATA_TYPE, *disconnected_calendar.clone())?;
    }

    Ok(other.into())
}
