use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError, Status, NotifyEvent};

use crate::data_types::{CALENDAR_DATA_TYPE, Event, EventOccurrenceOverride, Calendar, CalendarIndexUpdater};

pub fn redical_event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 4 {
        ctx.log_debug(format!("event_set WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid    = args.next_arg()?;

    // TODO: properly parse this into unix timestamp - allow date time strings
    let timestamp = args.next_i64()?;

    let calendar_key = ctx.open_key_writable(&calendar_uuid);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("key: {calendar_uuid} event uuid: {event_uuid}, other: {other}").as_str());

    let mut event = Event::parse_ical(event_uuid.try_as_str()?, other.as_str()).map_err(RedisError::String)?;

    let calendar = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?;

    if calendar.is_none() {
        return Err(RedisError::String(format!("No Calendar found on key: {calendar_uuid}")));
    }

    let calendar = calendar.unwrap();

    if let Some(event) = calendar.events.get_mut(&String::from(event_uuid)) {
        match EventOccurrenceOverride::parse_ical(other.as_str()) {
            Ok(event_occurrence_override) => {
                match event.override_occurrence(timestamp, &event_occurrence_override) {
                    Ok(updated_event) => {
                        *event = updated_event.clone();

                        calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

                        let status = ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &calendar_uuid);
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
        Err(RedisError::String("No event with UUID: '{event_uuid}' found".to_string()))
    }
}
