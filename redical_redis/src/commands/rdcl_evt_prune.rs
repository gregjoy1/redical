use std::collections::HashMap;
use std::str::FromStr;

use crate::datatype::CALENDAR_DATA_TYPE;

use redis_module::{Context,
    RedisResult,
    NotifyEvent,
    RedisString,
    RedisValue,
    NextArg,
    RedisError,
    Status,
};

use redical_core::{
    Calendar,
    Event,
    CalendarIndexUpdater,
    InvertedEventIndex
};

use redical_ical::values::date_time::DateTime;

pub fn redical_event_prune(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() != 4 {
        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!("No Calendar found on key: {calendar_uid}")));
    };

    let (from, until) = (
        args.next_arg()?.to_string(),
        args.next_arg()?.to_string(),
    );

    let (from_timestamp, until_timestamp) = timestamps_from_date_strings(
        from.to_string(),
        until.to_string(),
    )?;

    let pruned_events = prune_and_reindex(calendar, from_timestamp, until_timestamp)?;

    for (event_uid, _) in pruned_events.iter() {
        notify_keyspace_event(
            ctx,
            &calendar_uid,
            &from,
            &until,
            event_uid,
        )?;
    }

    Ok(RedisValue::Integer(pruned_events.len() as i64))
}

// TODO: make this a helper
fn timestamp_from_date_string(date_string: &str) -> Result<i64, RedisError> {
    DateTime::from_str(date_string)
        .map(|datetime| datetime.get_utc_timestamp(None))
        .map_err(RedisError::String)
}

fn timestamps_from_date_strings(from: String, until: String) -> Result<(i64, i64), RedisError> {
    let from_timestamp  = timestamp_from_date_string(&from)?;
    let until_timestamp = timestamp_from_date_string(&until)?;

    if from_timestamp > until_timestamp {
        return Err(RedisError::String(
            format!("FROM date: {} cannot be greater than the UNTIL date: {}", &from, &until))
        );
    }

    Ok((from_timestamp, until_timestamp))
}

fn prune_and_reindex(calendar: &mut Calendar, from: i64, until: i64) -> Result<HashMap<String, Box<Event>>, RedisError> {
    let pruned_events = calendar.prune_events(from, until).unwrap();

    if calendar.indexes_active {
        for (event_uid, pruned_event) in pruned_events.iter() {
            let mut calendar_index_updater = CalendarIndexUpdater::new(event_uid, calendar);

            let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
                pruned_event.indexed_categories.as_ref(),
                None,
            );

            let updated_event_location_type_diff = InvertedEventIndex::diff_indexed_terms(
                pruned_event.indexed_location_type.as_ref(),
                None,
            );

            let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
                pruned_event.indexed_related_to.as_ref(),
                None,
            );

            let updated_event_geo_diff = InvertedEventIndex::diff_indexed_terms(
                pruned_event.indexed_geo.as_ref(),
                None,
            );

            let updated_event_class_diff = InvertedEventIndex::diff_indexed_terms(
                pruned_event.indexed_class.as_ref(),
                None,
            );

            calendar_index_updater
                .update_indexed_categories(&updated_event_categories_diff)
                .map_err(|error| RedisError::String(error.to_string()))?;
            
            calendar_index_updater
                .update_indexed_location_type(&updated_event_location_type_diff)
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
        }
    }

    Ok(pruned_events)
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, from: &String, until: &String, event_uid: &String) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evt_prune:{event_uid}:{from}-{until}");

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), calendar_uid) == Status::Err {
        let message = format!(
            "Notify keyspace event \"rdcl.evt_prune\" for calendar: \"{}\", range: {} - {}, event: \"{}\"",
            &calendar_uid,
            &from,
            &until,
            &event_uid,
        );

        return Err(RedisError::String(message));
    }

    Ok(())
}
