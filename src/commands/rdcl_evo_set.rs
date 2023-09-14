use redis_module::{Context, NextArg, RedisResult, RedisString, RedisError, Status, NotifyEvent};

use crate::data_types::{CALENDAR_DATA_TYPE, EventOccurrenceOverride, Calendar, CalendarIndexUpdater, InvertedEventIndex};

use crate::parsers::datestring_to_date;

pub fn redical_event_override_set(ctx: &Context, args: Vec<RedisString>) -> RedisResult {
    if args.len() < 4 {
        ctx.log_debug(format!("rdcl.evo_set: WrongArity: {{args.len()}}").as_str());

        return Err(RedisError::WrongArity);
    }

    let mut args = args.into_iter().skip(1);

    let calendar_uuid = args.next_arg()?;
    let event_uuid    = args.next_arg()?;

    let timestamp = match datestring_to_date(args.next_arg()?.try_as_str()?, None, "") {
        Ok(datetime) => datetime.timestamp(),
        Err(error)   => return Err(RedisError::String(format!("{:#?}", error))),
    };

    let calendar_key = ctx.open_key_writable(&calendar_uuid);

    let other: String = args.map(|arg| arg.try_as_str().unwrap_or("")).collect::<Vec<&str>>().join(" ").as_str().to_owned();

    ctx.log_debug(format!("rdcl.evo_set: key: {calendar_uuid} event uuid: {event_uuid}, other: {other}").as_str());

    let calendar = calendar_key.get_value::<Calendar>(&CALENDAR_DATA_TYPE)?;

    if calendar.is_none() {
        return Err(RedisError::String(format!("rdcl.evo_set: No Calendar found on key: {calendar_uuid}")));
    }

    let mut calendar = calendar.unwrap();

    let event = calendar.events.get_mut(&String::from(event_uuid.clone()));

    if event.is_none() {
        return Err(RedisError::String("No event with UUID: '{event_uuid}' found".to_string()));
    }

    let mut event = event.unwrap().to_owned();

    let event_occurrence_override = match EventOccurrenceOverride::parse_ical(other.as_str()) {
        Ok(event_occurrence_override) => event_occurrence_override,
        Err(error) => { return Err(RedisError::String(error)) },
    };

    match event.override_occurrence(timestamp, &event_occurrence_override) {
        Err(error) => { return Err(RedisError::String(error)) },
        _ => {},
    }

    let existing_event = calendar.events.insert(String::from(event_uuid.clone()), event.to_owned());

    let updated_event_categories_diff = InvertedEventIndex::diff_indexed_terms(
        existing_event.clone().and_then(|existing_event| existing_event.indexed_categories.clone()).as_ref(),
        event.indexed_categories.as_ref(),
    );

    let updated_event_related_to_diff = InvertedEventIndex::diff_indexed_terms(
        existing_event.clone().and_then(|existing_event| existing_event.indexed_related_to.clone()).as_ref(),
        event.indexed_related_to.as_ref(),
    );

    let mut calendar_index_updater = CalendarIndexUpdater::new(event.uuid.clone(), &mut calendar);

    calendar_index_updater.update_indexed_categories(&updated_event_categories_diff).map_err(|error| RedisError::String(error.to_string()))?;
    calendar_index_updater.update_indexed_related_to(&updated_event_related_to_diff).map_err(|error| RedisError::String(error.to_string()))?;

    calendar_key.set_value(&CALENDAR_DATA_TYPE, calendar.clone())?;

    if ctx.notify_keyspace_event(NotifyEvent::GENERIC, "event.override_set", &calendar_uuid) == Status::Err {
        return Err(RedisError::Str("Generic error"));
    }

    Ok(other.into())
}
