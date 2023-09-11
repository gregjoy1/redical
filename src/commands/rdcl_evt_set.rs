use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError};

use crate::data_types::{CALENDAR_DATA_TYPE, Event, Calendar, EventDiff, ScheduleRebuildConsensus};

pub fn redical_event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    // TODO: Add option to "rebase" overrides against changes, i.e. add/remove all
    // base added/removed properties to all overrides.
    if args.len() < 3 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid    = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uuid);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {calendar_uuid} event uuid: {event_uuid}, other: {other}").as_str());

    let mut event = Event::parse_ical(event_uuid.try_as_str()?, other.as_str()).map_err(RedisError::String)?;

    let calendar = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?;

    if calendar.is_none() {
        return Err(RedisError::String(format!("No Calendar found on key: {calendar_uuid}")));
    }

    let calendar = calendar.unwrap();

    let existing_event = calendar.events.get_mut(&String::from(event_uuid.clone()));

    let event_diff = if existing_event.is_some() {
        EventDiff::new(&existing_event.as_ref().unwrap(), &event)
    } else {
        EventDiff::new(&Event::new(event.uuid.clone()), &event)
    };

    // let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), calendar);

    // if existing_event.is_some() {
    //     let existing_event = existing_event.unwrap();

    //     event.overrides = existing_event.overrides.clone();

    //     event.overrides.rebase_overrides(&event_diff).map_err(|error| RedisError::String(error.to_string()))?;
    // }

    // event.rebuild_indexed_categories().map_err(RedisError::String)?;
    // event.rebuild_indexed_related_to().map_err(RedisError::String)?;

    // if rebuild_event_occurrence_cache(&event_diff) {
    //     event.rebuild_occurrence_cache(65_535)
    //          .map_err(|error| RedisError::String(error.to_string()))?;
    // }

    // calendar.events.insert(String::from(event_uuid), event);

    // calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    Ok(other.into())
}

fn rebuild_event_occurrence_cache(event_diff: &EventDiff) -> bool {
    if let Some(schedule_properties) = &event_diff.schedule_properties {
        match schedule_properties.get_schedule_rebuild_consensus() {
            ScheduleRebuildConsensus::Full | ScheduleRebuildConsensus::Partial => {
            },

            _ => {}
        }

        true
    } else {
        false
    }
}
