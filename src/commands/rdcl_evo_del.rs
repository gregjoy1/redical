use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError};

use crate::data_types::{EVENT_DATA_TYPE, Event, Calendar, CalendarIndexUpdater};

pub fn redical_event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
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
