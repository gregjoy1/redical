use std::str::FromStr;

use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use redical_core::{Calendar, CalendarIndexUpdater, InvertedEventIndex};
use crate::datatype::CALENDAR_DATA_TYPE;

use redical_ical::values::date_time::DateTime;

pub fn redical_event_override_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 3 {
        ctx.log_debug(format!("rdcl.evo_del: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();
    let override_date_string = args.next_arg()?.try_as_str()?;

    let override_timestamp =
        DateTime::from_str(override_date_string)
            .map(|datetime| datetime.get_utc_timestamp(None))
            .map_err(RedisError::String)?;

    ctx.log_debug(
        format!("rdcl.evo_del: calendar_uid: {calendar_uid} event_uid: {event_uid} occurrence date string: {override_date_string}").as_str()
    );

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    let Some(mut event) = calendar.events.get(&event_uid).cloned() else {
        return Err(RedisError::String(format!(
            "No event with UID: '{}' found",
            event_uid
        )));
    };

    // Record whether the override removed actually existed for that timestamp or not.
    let was_override_removed = event.remove_occurrence_override(override_timestamp, calendar.indexes_active.to_owned()).map_err(RedisError::String)?.is_some();

    if calendar.indexes_active {
        // HashMap.insert returns the old value (if present) which we can use in diffing old -> new.
        let existing_event = calendar
            .events
            .insert(event_uid.to_owned(), event.to_owned());

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

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    if was_override_removed {
        notify_keyspace_event(ctx, &calendar_uid, &event_uid, override_date_string)?;
    }

    Ok(RedisValue::Bool(was_override_removed))
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, event_uid: &String, override_date_string: &str) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evo_del:{}:{}", event_uid, override_date_string);

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.evo_del\" for calendar: \"{}\" event: \"{}\" date string: \"{}\"", &calendar_uid, &event_uid, &override_date_string)
            )
        );
    }

    Ok(())
}
