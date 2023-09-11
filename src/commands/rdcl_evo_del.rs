use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError};

use crate::data_types::{CALENDAR_DATA_TYPE, Event, Calendar, CalendarIndexUpdater};

pub fn redical_event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
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

    // if let Some(event) = calendar.events.get(&String::from(event_uuid.clone())) {

    //     // TODO: Populate and validate this...
    //     let connected_calendars: Vec<Box<Calendar>> = vec![];
    //     let disconnected_calendars: Vec<Box<Calendar>> = vec![];

    //     let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), calendar);

    //     match event.remove_occurrence_override(timestamp, &mut calendar_index_updater) {
    //         Ok(updated_event) => {
    //             calendar.events.insert(String::from(event_uuid), *updated_event);

    //             calendar_key.set_value(&CALENDAR_DATA_TYPE, updated_event.clone())?;
    //         },

    //         Err(error) => {
    //             return Err(RedisError::String(error));
    //         }
    //     }

    //     Ok(timestamp.into())

    // } else {
    //     Err(RedisError::String("No event with UUID: '{key}' found".to_string()))
    // }

    // TODO: delete...
    Err(RedisError::String(format!("tmp delete me")))
}
