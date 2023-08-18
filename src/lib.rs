use redis_module::{redis_module, Context, NextArg, RedisValue, RedisResult, RedisString, RedisError, NotifyEvent, Status};

mod data_types;

use data_types::{EVENT_DATA_TYPE, CALENDAR_DATA_TYPE, Event, EventOccurrenceOverride, OccurrenceIndexValue, Calendar, CalendarIndexUpdater, UpdatedSetMembers, CalendarCategoryIndexUpdater, CalendarRelatedToIndexUpdater};

fn args_test<'a>(_: &Context, args: Vec<RedisString>) -> RedisResult {
    let response = args.into_iter().map(|arg| RedisValue::SimpleString(arg.to_string())).collect();

    Ok(RedisValue::Array(response))
}

fn event_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    match Event::parse_ical(key_name.try_as_str()?,other.as_str()) {
        Ok(mut event) => {

            let mut connected_calendars: Vec<Box<Calendar>> = vec![];
            let mut disconnected_calendars: Vec<Box<Calendar>> = vec![];

            let existing_event = key.get_value::<Event>(&EVENT_DATA_TYPE)?;

            if existing_event.is_some() {
                let existing_event = existing_event.as_ref().unwrap();

                let updated_connected_calendars = UpdatedSetMembers::new(
                    existing_event.indexed_properties.get_indexed_calendars().as_ref(),
                    event.indexed_properties.get_indexed_calendars().as_ref()
                );

                for disconnected_calendar_uuid in updated_connected_calendars.removed.iter() {
                    let key = ctx.open_key(&RedisString::create(None, disconnected_calendar_uuid.as_bytes()));

                    if let Some(disconnected_calendar) = key.get_value::<Calendar>(&EVENT_DATA_TYPE)? {
                        disconnected_calendars.push(Box::new(disconnected_calendar.clone()));
                    } else {
                        // TODO: log that disconnected calendar no longer exists (which is fine
                        // because nothing needs to be done).
                    }
                }

                for connected_calendar_uuid in updated_connected_calendars.all_present_members().iter() {
                    let key = ctx.open_key(&RedisString::create(None, connected_calendar_uuid.as_bytes()));

                    if let Some(connected_calendar) = key.get_value::<Calendar>(&EVENT_DATA_TYPE)? {
                        connected_calendars.push(Box::new(connected_calendar.clone()));
                    } else {
                        return Err(RedisError::String(format!("Indexed calendar with UUID: {connected_calendar_uuid} is not present.")));
                    }
                }
            } else if let Some(indexed_calendars) = event.indexed_properties.get_indexed_calendars() {
                for connected_calendar_uuid in indexed_calendars.iter() {
                    let key = ctx.open_key(&RedisString::create(None, connected_calendar_uuid.as_bytes()));

                    if let Some(connected_calendar) = key.get_value::<Calendar>(&EVENT_DATA_TYPE)? {
                        connected_calendars.push(Box::new(connected_calendar.clone()));
                    } else {
                        return Err(RedisError::String(format!("Indexed calendar with UUID: {connected_calendar_uuid} is not present.")));
                    }
                }
            }

            let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), connected_calendars, disconnected_calendars);

            if existing_event.is_some() && calendar_index_updater.is_any_disconnected_calendars() {
                let existing_event = existing_event.unwrap();

                CalendarCategoryIndexUpdater::new(&mut calendar_index_updater).remove_event_from_calendar(existing_event).map_err(RedisError::String)?;
                CalendarRelatedToIndexUpdater::new(&mut calendar_index_updater).remove_event_from_calendar(existing_event).map_err(RedisError::String)?;
            }

            event.rebuild_indexed_categories(&mut calendar_index_updater).map_err(RedisError::String)?;
            event.rebuild_indexed_related_to(&mut calendar_index_updater).map_err(RedisError::String)?;

            event.rebuild_occurrence_cache(1000).map_err(|error| RedisError::String(error.to_string()))?;

            key.set_value(&EVENT_DATA_TYPE, event.clone())?;

            Ok(other.into())
        },
        Err(error) => {
            Err(RedisError::String(error))
        }
    }
}

fn event_get(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                    => Ok(RedisValue::Null),
        Some(event) => Ok(RedisValue::BulkString(format!("event: {:?}", event)))
    }
}

fn event_occurrences_list(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 1 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key(&key_name);

    ctx.log_debug(format!("key: {key_name}").as_str());

    match key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        None                    => Ok(RedisValue::Null),
        Some(event) => {

            match event.occurrence_cache {
                Some(ref occurrence_cache) => {
                    Ok(
                        RedisValue::Array(
                            occurrence_cache.iter()
                                            .map(|(timestamp, value)| {
                                                match value {
                                                    OccurrenceIndexValue::Occurrence => RedisValue::SimpleString(format!("{timestamp} - occurrence")),
                                                    OccurrenceIndexValue::Override   => RedisValue::SimpleString(format!("{timestamp} - override")),
                                                }
                                            })
                                            .collect()
                        )
                    )
                },

                None => {
                    Ok(RedisValue::Array(vec![]))
                }
            }
        }
    }
}

fn event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    // TODO: properly parse this into unix timestamp - allow date time strings
    let timestamp = args.next_i64()?;

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {key_name}, other: {other}").as_str());

    if let Some(event) = key.get_value::<Event>(&EVENT_DATA_TYPE)? {
        match EventOccurrenceOverride::parse_ical(other.as_str()) {
            Ok(event_occurrence_override) => {

                // TODO: Populate and validate this...
                let connected_calendars: Vec<Box<Calendar>> = vec![];
                let disconnected_calendars: Vec<Box<Calendar>> = vec![];

                let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), connected_calendars, disconnected_calendars);

                match event.override_occurrence(timestamp, &event_occurrence_override, &mut calendar_index_updater) {
                    Ok(updated_event) => {
                        key.set_value(&EVENT_DATA_TYPE, updated_event.clone())?;

                        let status = ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &key_name);
                        match status {
                            Status::Err => {
                                return Err(RedisError::Str("Generic error"));
                            },
                            _ => {},
                        }
                    },

                    Err(error) => {
                        return Err(RedisError::String(error));
                    }
                }

                Ok(other.into())
            },

            Err(error) => {
                Err(RedisError::String(error))
            }
        }

    } else {
        Err(RedisError::String("No event with UUID: '{key}' found".to_string()))
    }
}

fn event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let key_name = args.next_arg()?;

    let key = ctx.open_key_writable(&key_name);

    let timestamp = args.next_i64()?;

    if let Some(event) = key.get_value::<Event>(&EVENT_DATA_TYPE)? {

        // TODO: Populate and validate this...
        let connected_calendars: Vec<Box<Calendar>> = vec![];
        let disconnected_calendars: Vec<Box<Calendar>> = vec![];

        let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), connected_calendars, disconnected_calendars);

        match event.remove_occurrence_override(timestamp, &mut calendar_index_updater) {
            Ok(updated_event) => {
                key.set_value(&EVENT_DATA_TYPE, updated_event.clone())?;
            },

            Err(error) => {
                return Err(RedisError::String(error));
            }
        }

        Ok(timestamp.into())

    } else {
        Err(RedisError::String("No event with UUID: '{key}' found".to_string()))
    }
}

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    ctx.log_notice(
        format!(
            "Received event: {:?} on key: {} via event: {}",
            event_type,
            std::str::from_utf8(key).unwrap(),
            event
        ).as_str()
    );
}

pub const MODULE_NAME:    &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

#[cfg(not(test))]
redis_module! {
    name:       MODULE_NAME,
    version:    MODULE_VERSION,
    allocator:  (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [
        EVENT_DATA_TYPE,
        CALENDAR_DATA_TYPE
    ],
    commands:   [
        ["args.test",              args_test, "", 0, 0, 0],
        ["event.set",              event_set, "", 0, 0, 0],
        ["event.get",              event_get, "", 0, 0, 0],
        ["event.occurrences_list", event_occurrences_list, "", 0, 0, 0],
        ["event.override_set",     event_override_set, "", 0, 0, 0],
        ["event.override_del",     event_override_del, "", 0, 0, 0],
    ],
    event_handlers: [
        [@STRING: on_event],
    ]
}
