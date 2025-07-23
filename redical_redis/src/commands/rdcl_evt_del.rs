use redis_module::{Context, NextArg, NotifyEvent, RedisError, RedisResult, RedisString, Status, RedisValue};

use crate::core::{Calendar, CalendarIndexUpdater, InvertedEventIndex};
use crate::datatype::CALENDAR_DATA_TYPE;

pub fn redical_event_del(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 2 {
        ctx.log_debug(format!("rdcl.evt_del: WrongArity: {}", args.len()).as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uid = args.next_arg()?;
    let event_uid = args.next_arg()?.to_string();

    let calendar_key = ctx.open_key_writable(&calendar_uid);

    ctx.log_debug(
        format!("rdcl.evt_get: calendar_uid: {calendar_uid} event_uid: {event_uid}").as_str(),
    );

    let Some(calendar) = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)? else {
        return Err(RedisError::String(format!(
            "No Calendar found on key: {calendar_uid}"
        )));
    };

    let Some(existing_event) = calendar.events.get_mut(&event_uid) else {
        return Ok(RedisValue::Bool(false));
    };

    if calendar.indexes_active {
        let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event.indexed_categories.as_ref(),
            None,
        );

        let updated_event_location_type_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event.indexed_location_type.as_ref(),
            None,
        );

        let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event.indexed_related_to.as_ref(),
            None,
        );

        let updated_event_geo_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event.indexed_geo.as_ref(),
            None,
        );

        let updated_event_class_diff = InvertedEventIndex::diff_indexed_terms(
            existing_event.indexed_class.as_ref(),
            None,
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

    calendar.remove_event(&event_uid);

    // Use this command when replicating across other Redis instances.
    ctx.replicate_verbatim();

    notify_keyspace_event(ctx, &calendar_uid, &event_uid)?;

    Ok(RedisValue::Bool(true))
}

fn notify_keyspace_event(ctx: &Context, calendar_uid: &RedisString, event_uid: &String) -> Result<(), RedisError> {
    let event_message = format!("rdcl.evt_del:{event_uid}");

    if ctx.notify_keyspace_event(NotifyEvent::MODULE, event_message.as_str(), calendar_uid) == Status::Err {
        return Err(
            RedisError::String(
                format!("Notify keyspace event \"rdcl.evo_set\" for calendar: \"{}\" event: \"{}\"", &calendar_uid, &event_uid)
            )
        );
    }

    Ok(())
}
