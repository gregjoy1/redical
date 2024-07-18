use std::str::FromStr;

use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use redical_core::{Calendar, EventOccurrenceOverride, CalendarIndexUpdater, InvertedEventIndex};
use crate::datatype::CALENDAR_DATA_TYPE;

use redical_ical::values::date_time::DateTime;

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, event_uid: &String, override_date_string: &str) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evo_prune:{}:{}", event_uid, override_date_string);

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.evo_prune\" for calendar: \"{}\" event: \"{}\" date string: \"{}\"", &calendar_uid, &event_uid, &override_date_string)
            )
        );
    }

    Ok(())
}

fn prune_calendar_events_overrides(calendar: &mut Calendar, event_uid: String, from_timestamp: i64, until_timestamp: i64) -> Result<Vec<(i64, EventOccurrenceOverride)>, RedisError> {
    let Some(mut event) = calendar.events.get(&event_uid).cloned() else {
        return Err(RedisError::String(format!(
            "No event with UID: '{}' found",
            event_uid
        )));
    };

    let removed_overrides = event.as_mut().prune_event_overrides(from_timestamp, until_timestamp).map_err(RedisError::String)?;

    let event_uid = event.uid.uid.to_string();

    // HashMap.insert returns the old value (if present) which we can use in diffing old -> new.
    let existing_event =
        calendar.events
                .insert(event_uid.to_owned(), event.to_owned());

    if calendar.indexes_active {
        let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_categories.clone())
                .as_ref(),
            event.indexed_categories.as_ref(),
        );

        let updated_event_location_type_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_location_type.clone())
                .as_ref(),
            event.indexed_location_type.as_ref(),
        );

        let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_related_to.clone())
                .as_ref(),
            event.indexed_related_to.as_ref(),
        );

        let updated_event_geo_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_geo.clone())
                .as_ref(),
            event.indexed_geo.as_ref(),
        );

        let updated_event_class_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event
                .as_ref()
                .and_then(|existing_event| existing_event.indexed_class.clone())
                .as_ref(),
            event.indexed_class.as_ref(),
        );

        let mut calendar_index_updater = CalendarIndexUpdater::new(&event_uid, calendar);

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

    Ok(removed_overrides)
}

fn timestamp_from_date_string(date_string: &str) -> Result<i64, RedisError> {
    DateTime::from_str(date_string)
        .map(|datetime| datetime.get_utc_timestamp(None))
        .map_err(RedisError::String)
}

fn timestamps_from_date_strings(from_date_string: String, until_date_string: String) -> Result<(i64, i64), RedisError> {
    let from_timestamp  = timestamp_from_date_string(&from_date_string)?;
    let until_timestamp = timestamp_from_date_string(&until_date_string)?;

    if from_timestamp > until_timestamp {
        return Err(RedisError::String(format!("FROM date: {} cannot be greater than the UNTIL date: {}", &from_date_string, &until_date_string)));
    }

    Ok((from_timestamp, until_timestamp))
}

pub fn redical_event_override_prune(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 || args.len() > 5 {
        ctx.log_debug(format!("rdcl.evo_prune: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    if args.len() == 3 {
        let event_uid = args.next_arg()?.to_string();

        let (from_timestamp, until_timestamp) =
            timestamps_from_date_strings(
                args.next_arg()?.to_string(),
                args.next_arg()?.to_string(),
            )?;

        ctx.log_debug(
            format!("rdcl.evo_prune: calendar_uid: {calendar_uid} event_uid: {event_uid} from_timestamp: {from_timestamp} until_timestamp: {until_timestamp}").as_str()
        );

        // Use this command when replicating across other Redis instances.
        // We call this here to ensure all replicas begin at and reach the same point if any errors
        // are raised in the following prune process.
        ctx.replicate_verbatim();

        for (override_timestamp, _event_occurrence_override) in prune_calendar_events_overrides(calendar, event_uid.to_owned(), from_timestamp, until_timestamp)? {
            notify_keyspace_event(ctx, &calendar_uid, &event_uid, &DateTime::from(override_timestamp).render_formatted_date_time(None))?;
        }
    } else {
        let (from_timestamp, until_timestamp) =
            timestamps_from_date_strings(
                args.next_arg()?.to_string(),
                args.next_arg()?.to_string(),
            )?;

        ctx.log_debug(
            format!("rdcl.evo_prune: calendar_uid: {calendar_uid} from_timestamp: {from_timestamp} until_timestamp: {until_timestamp}").as_str()
        );

        // TODO: Inefficient - optimise towards copy-less approach.
        let event_uids: Vec<String> = calendar.events.keys().map(String::from).collect();

        // Use this command when replicating across other Redis instances.
        // We call this here to ensure all replicas begin at and reach the same point if any errors
        // are raised in the following prune process.
        ctx.replicate_verbatim();

        for event_uid in event_uids {
            for (override_timestamp, _event_occurrence_override) in prune_calendar_events_overrides(calendar, event_uid.to_owned(), from_timestamp, until_timestamp)? {
                notify_keyspace_event(ctx, &calendar_uid, &event_uid, &DateTime::from(override_timestamp).render_formatted_date_time(None))?;
            }
        }
    }

    Ok(RedisValue::Bool(true))
}
