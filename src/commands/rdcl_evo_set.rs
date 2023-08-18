use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError, Status, NotifyEvent};

use crate::data_types::{EVENT_DATA_TYPE, Event, EventOccurrenceOverride, Calendar, CalendarIndexUpdater};

pub fn redical_event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
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
