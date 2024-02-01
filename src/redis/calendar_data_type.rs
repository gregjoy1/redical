use crate::core::{Calendar, Event, EventOccurrenceOverride};

use redis_module::{
    native_types::RedisType, raw, RedisModuleIO, RedisModuleString, RedisModuleTypeMethods,
};

use std::{
    ffi::{c_int, c_void},
    ptr::null_mut,
};

use serde::{Deserialize, Serialize};

use std::str::FromStr;

use crate::core::ical::properties::Property;

use crate::core::ical::serializer::SerializableICalComponent;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct StoredCalendar(String, Vec<String>, Vec<StoredEvent>);

impl TryFrom<&Calendar> for StoredCalendar {
    type Error = String;

    fn try_from(calendar: &Calendar) -> Result<Self, Self::Error> {
        let uid = calendar.uid.uid.to_owned();

        let properties: Vec<String> = calendar.serialize_to_ical_set(None).into_iter().collect();

        let mut stored_events: Vec<StoredEvent> = Vec::new();

        for event in calendar.events.values() {
            stored_events.push(
                StoredEvent::try_from(event)?
            );
        }

        Ok(
            StoredCalendar(uid, properties, stored_events)
        )
    }
}

impl TryFrom<&StoredCalendar> for Calendar {
    type Error = String;

    fn try_from(stored_calendar: &StoredCalendar) -> Result<Self, Self::Error> {
        let stored_calendar_uid = stored_calendar.0.to_owned();

        let mut calendar = Calendar::new(stored_calendar_uid.clone());

        for stored_property in &stored_calendar.1 {
            let property = Property::from_str(stored_property.as_str())?;

            calendar.insert(property)?;
        }

        let parsed_calendar_uid = calendar.uid.uid.to_owned();

        if stored_calendar_uid != parsed_calendar_uid {
            return Err(format!("Calendar UID property: {} does not match stored UID key: {}", parsed_calendar_uid, stored_calendar_uid));
        }

        for stored_event in stored_calendar.2.iter() {
            let event = Event::try_from(stored_event)?;
            let event_uid = event.uid.uid.to_owned();

            calendar.events.insert(event_uid, event);
        }

        calendar.rebuild_indexes()?;

        Ok(
            calendar
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct StoredEvent(String, Vec<String>, Vec<StoredEventOccurrenceOverride>);

impl TryFrom<&Event> for StoredEvent {
    type Error = String;

    fn try_from(event: &Event) -> Result<Self, Self::Error> {
        let uid = event.uid.uid.to_owned();

        let properties: Vec<String> = event.serialize_to_ical_set(None).into_iter().collect();

        let mut stored_event_occurrence_overrides: Vec<StoredEventOccurrenceOverride> = Vec::new();

        for event_occurrence_override in event.overrides.values() {
            stored_event_occurrence_overrides.push(
                StoredEventOccurrenceOverride::try_from(event_occurrence_override)?
            );
        }

        Ok(
            StoredEvent(uid, properties, stored_event_occurrence_overrides)
        )
    }
}

impl TryFrom<&StoredEvent> for Event {
    type Error = String;

    fn try_from(stored_event: &StoredEvent) -> Result<Self, Self::Error> {
        let stored_event_uid = stored_event.0.to_owned();

        let mut event = Event::new(stored_event_uid.clone());

        for stored_property in &stored_event.1 {
            let property = Property::from_str(stored_property.as_str())?;

            event.insert(property)?;
        }

        event.rebuild_indexes()?;

        let parsed_event_uid = event.uid.uid.to_owned();

        if stored_event_uid != parsed_event_uid {
            return Err(format!("Event UID property: {} does not match stored UID key: {}", parsed_event_uid, stored_event_uid));
        }

        for stored_event_occurrence_override in stored_event.2.iter() {
            let event_occurrence_override = EventOccurrenceOverride::try_from(stored_event_occurrence_override)?;

            event.override_occurrence(&event_occurrence_override)?;
        }

        Ok(
            event
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
struct StoredEventOccurrenceOverride(String, Vec<String>);

impl TryFrom<&EventOccurrenceOverride> for StoredEventOccurrenceOverride {
    type Error = String;

    fn try_from(event_occurrence_override: &EventOccurrenceOverride) -> Result<Self, Self::Error> {
        let Some(dtstart_property) = event_occurrence_override.dtstart.as_ref() else {
            return Err(String::from("EventOccurrenceOverride is invalid, requires defined DTSTART property"));
        };

        let occurrence_date_string = dtstart_property.serialize_datestring_value(None);

        let properties: Vec<String> = event_occurrence_override.serialize_to_ical_set(None).into_iter().collect();

        Ok(
            StoredEventOccurrenceOverride(occurrence_date_string, properties)
        )
    }
}

impl TryFrom<&StoredEventOccurrenceOverride> for EventOccurrenceOverride {
    type Error = String;

    fn try_from(stored_event_occurrence_override: &StoredEventOccurrenceOverride) -> Result<Self, Self::Error> {
        let mut event_occurrence_override = EventOccurrenceOverride::default();

        for stored_property in &stored_event_occurrence_override.1 {
            let property = Property::from_str(stored_property.as_str())?;

            event_occurrence_override.insert(property)?;
        }

        event_occurrence_override.validate()?;

        if let Some(dtstart) = event_occurrence_override.dtstart.as_ref() {
            let stored_date_time_string = stored_event_occurrence_override.0.to_owned();
            let parsed_date_time_string = dtstart.serialize_datestring_value(None);

            if stored_date_time_string != parsed_date_time_string {
                return Err(format!("EventOccurrenceOverride DTSTART property: {parsed_date_time_string} does not match stored DTSTART key: {stored_date_time_string}"));
            }
        }

        Ok(
            event_occurrence_override
        )
    }
}

pub const CALENDAR_DATA_TYPE_NAME: &str = "RICAL_CAL";
pub const CALENDAR_DATA_TYPE_VERSION: i32 = 1;

pub static CALENDAR_DATA_TYPE: RedisType = RedisType::new(
    CALENDAR_DATA_TYPE_NAME,
    CALENDAR_DATA_TYPE_VERSION,
    RedisModuleTypeMethods {
        version: redis_module::TYPE_METHOD_VERSION,
        rdb_load: Some(rdb_load),
        rdb_save: Some(rdb_save),
        aof_rewrite: Some(aof_rewrite),
        mem_usage: Some(mem_usage),
        digest: None,
        free: Some(free),
        aux_load: None,
        aux_save: None,
        aux_save_triggers: 0,
        free_effort: None,
        unlink: None,
        copy: Some(copy),
        defrag: None,

        copy2: None,
        free_effort2: None,
        mem_usage2: None,
        unlink2: None,
    },
);

pub extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, _encver: c_int) -> *mut c_void {
    let Ok(buffer) = raw::load_string_buffer(rdb) else {
        return null_mut();
    };

    let bytes: &[u8] = buffer.as_ref();

    let stored_calendar: StoredCalendar = bincode::deserialize(&bytes).unwrap();

    let calendar = match Calendar::try_from(&stored_calendar) {
        Ok(calendar) => calendar,

        // TODO: Handle properly - log error and return null etc.
        Err(error) => {
            panic!("rdb_load failed for Calendar with error: {:#?}", error);
        },
    };

    println!("Calendar data type - rdb_load - UID: {:#?}", calendar.uid.uid);

    Box::into_raw(Box::new(calendar)).cast::<libc::c_void>()
}

pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    println!("Calendar data type - rdb_save - UID: {:#?}", calendar.uid.uid);

    let stored_calendar = match StoredCalendar::try_from(calendar) {
        Ok(stored_calendar) => stored_calendar,

        // TODO: Handle properly - log error and return null etc.
        Err(error) => {
            panic!("rdb_save failed for Calendar with error: {:#?}", error);
        },
    };

    let bytes: Vec<u8> = bincode::serialize(&stored_calendar).unwrap();

    let str = std::str::from_utf8_unchecked(&bytes[..]); // no save_string_buffer available in redis-module :(

    raw::save_string(rdb, str);
}

unsafe extern "C" fn aof_rewrite(
    _aof: *mut RedisModuleIO,
    _key: *mut RedisModuleString,
    _value: *mut c_void,
) {
    todo!();
}

unsafe extern "C" fn mem_usage(_value: *const c_void) -> usize {
    // todo!();
    0
}

unsafe extern "C" fn free(value: *mut c_void) {
    if value.is_null() {
        println!("Calendar data type - free - is null");
        // on Redis 6.0 we might get a NULL value here, so we need to handle it.
        return;
    }

    let calendar = value as *mut Calendar;

    println!("Calendar data type - free - calendar : {:#?}", &calendar);

    drop(Box::from_raw(calendar));
}

unsafe extern "C" fn copy(
    _fromkey: *mut RedisModuleString,
    _tokey: *mut RedisModuleString,
    value: *const c_void,
) -> *mut c_void {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let calendar_cloned = calendar.clone();

    Box::into_raw(Box::new(calendar_cloned)).cast::<c_void>()
}
