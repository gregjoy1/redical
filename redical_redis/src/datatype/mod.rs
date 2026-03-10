use redical_core::Calendar;

use redis_module::{
    logging,
    native_types::RedisType, raw, RedisModuleIO, RedisModuleString, RedisModuleTypeMethods,
};

use std::{
    ffi::{c_int, c_void},
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::null_mut,
};

mod rdb_data;

#[cfg(test)]
pub(crate) mod test_helpers;

use rdb_data::{RDBCalendar, RDBCalendarDump};

const BUILD_VERSION: Option<&str> = option_env!("GIT_SHA");

/// Thin wrappers around redis_module::logging that are no-ops in test mode.
/// The upstream `cfg!(test)` check only applies when redis-module itself is
/// under test, not when our crate is tested as a dependent.
mod log {
    /// Log at DEBUG level; silently ignored during tests to avoid FFI calls.
    #[allow(unused_variables)]
    pub fn debug(message: &str) {
        if !cfg!(test) {
            super::logging::log_debug(message);
        }
    }

    /// Log at NOTICE level; silently ignored during tests to avoid FFI calls.
    #[allow(unused_variables)]
    pub fn notice(message: &str) {
        if !cfg!(test) {
            super::logging::log_notice(message);
        }
    }

    /// Log at WARNING level; silently ignored during tests to avoid FFI calls.
    #[allow(unused_variables)]
    pub fn warning(message: &str) {
        if !cfg!(test) {
            super::logging::log_warning(message);
        }
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

/// Redis RDB load callback. Deserializes a Calendar from the RDB snapshot,
/// first attempting the current envelope format, then falling back to the
/// legacy iCal-only format for backward compatibility.
pub extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, _encver: c_int) -> *mut c_void {
    let Ok(buffer) =
        raw::load_string_buffer(rdb) else {
            log::warning("RDB load: failed to read string buffer from RDB");

            return null_mut();
        };

    let bytes: &[u8] = buffer.as_ref();

    let calendar =
        match bincode::deserialize::<RDBCalendarDump>(bytes) {
            Ok(envelope) => {
                load_from_dump_envelope(envelope)
            },

            Err(_) => {
                log::notice("RDB calendar load: not current format, trying legacy");

                load_from_legacy_ical_dump(bytes)
            },
        };

    Box::into_raw(
        Box::new(calendar)
    ).cast::<c_void>()
}

/// Restore a Calendar from a versioned dump envelope. When the build version
/// matches the saved version, takes a fast path by deserializing the raw
/// bincode dump directly. On version mismatch, corrupted data, or panic,
/// falls back to rebuilding the Calendar from its portable iCal representation.
pub(crate) fn load_from_dump_envelope(envelope: RDBCalendarDump) -> Calendar {
    let version_match = matches!(
        (&envelope.version, BUILD_VERSION), (Some(saved), Some(current)) if saved == current
    );

    if !version_match {
        let saved   = envelope.version.as_deref().unwrap_or("None");
        let current = BUILD_VERSION.unwrap_or("None");

        log::warning(
            &format!("RDB load: fast path skipped (version build digest mismatch: {saved} vs {current})")
        );
    } else {
        let result =
            catch_unwind(
                AssertUnwindSafe(|| -> Result<Calendar, String> {
                    let mut calendar = bincode::deserialize::<Calendar>(&envelope.raw_dump)
                        .map_err(|error| format!("{error}"))?;

                    calendar.validate_and_rebuild_indexes()
                        .map_err(|error| error.to_string())?;

                    Ok(calendar)
                })
            );

        match result {
            Ok(
                Ok(calendar)
            ) => {
                log::debug("RDB load: fast path OK");

                return calendar;
            },

            Ok(
                Err(error)
            ) => {
                log::warning(
                    &format!("RDB load: fast path failed ({error}), using iCal fallback")
                );
            },

            Err(panic_payload) => {
                let message =
                    if let Some(panic_error_message) = panic_payload.downcast_ref::<&str>() {
                        (*panic_error_message).to_string()
                    } else if let Some(panic_error_message) = panic_payload.downcast_ref::<String>() {
                        panic_error_message.clone()
                    } else {
                        String::from("unknown panic")
                    };

                log::warning(
                    &format!("RDB load: fast path panicked (payload: '{message}'), using iCal fallback")
                );
            },
        }
    }

    Calendar::try_from(&envelope.dump).unwrap_or_else(|error| {
        panic!("RDB load: iCal fallback failed: {error}")
    })
}

/// Restore a Calendar from the pre-envelope legacy format, which stored only
/// the iCal-based RDBCalendar without versioning or a raw bincode dump.
pub(crate) fn load_from_legacy_ical_dump(bytes: &[u8]) -> Calendar {
    let rdb_calendar: RDBCalendar = bincode::deserialize(bytes).unwrap();

    Calendar::try_from(&rdb_calendar).unwrap_or_else(|error| {
        panic!("rdb_load failed for Calendar with error: {error:#?}")
    })
}

/// Redis RDB save callback. Serializes a Calendar into a versioned envelope
/// containing both the raw bincode dump (for fast reload on matching builds)
/// and the portable iCal representation (for cross-version compatibility).
pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let raw_dump = bincode::serialize(calendar).unwrap();

    let rdb_calendar =
        RDBCalendar::try_from(calendar).unwrap_or_else(|error| {
            panic!("rdb_save failed for Calendar with error: {error:#?}");
        });

    let envelope =
        RDBCalendarDump {
            version:  BUILD_VERSION.map(String::from),
            raw_dump,
            dump:     rdb_calendar,
        };

    let bytes = bincode::serialize(&envelope).unwrap();

    raw::save_slice(rdb, &bytes);
}

/// Redis AOF rewrite callback. Currently a no-op because a Calendar is built
/// from multiple commands and there is no single command that can reconstruct it.
unsafe extern "C" fn aof_rewrite(
    _aof: *mut RedisModuleIO,
    _key: *mut RedisModuleString,
    _value: *mut c_void,
) {
    // A Calendar is built from multiple commands (RICAL_SET, property decorators, etc.)
    // so there is no single Redis command that can reconstruct it. AOF rewrite is a no-op
    // until a multi-command emit strategy is designed in a future version.
}

/// Redis memory usage callback. Returns 0 as accurate Calendar memory
/// accounting is not yet implemented.
unsafe extern "C" fn mem_usage(_value: *const c_void) -> usize {
    0
}

/// Redis free callback. Reclaims the heap-allocated Calendar when Redis
/// evicts or deletes a key. Handles the NULL pointer case for Redis 6.0.
unsafe extern "C" fn free(value: *mut c_void) {
    if value.is_null() {
        // on Redis 6.0 we might get a NULL value here, so we need to handle it.
        return;
    }

    let calendar = value as *mut Calendar;

    // println!("Calendar data type - free - calendar : {:#?}", &calendar);

    drop(Box::from_raw(calendar));
}

/// Redis COPY command callback. Produces a deep clone of the Calendar so
/// the source and destination keys are fully independent.
unsafe extern "C" fn copy(
    _fromkey: *mut RedisModuleString,
    _tokey: *mut RedisModuleString,
    value: *const c_void,
) -> *mut c_void {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let calendar_cloned = calendar.clone();

    Box::into_raw(Box::new(calendar_cloned)).cast::<c_void>()
}

#[cfg(test)]
mod load_tests {
    use super::*;

    use super::test_helpers::{build_test_calendar, fixture_path};

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn load_from_dump_envelope_with_none_version_uses_ical_fallback() {
        let calendar     = build_test_calendar();
        let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();
        let raw_dump     = bincode::serialize(&calendar).unwrap();

        let envelope = RDBCalendarDump {
            version:  None,
            raw_dump,
            dump:     rdb_calendar,
        };

        let result = load_from_dump_envelope(envelope);

        assert_eq!(result, calendar);
    }

    #[test]
    fn load_from_legacy_ical_dump_produces_correct_calendar() {
        let calendar     = build_test_calendar();
        let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();
        let bytes        = bincode::serialize(&rdb_calendar).unwrap();

        let result = load_from_legacy_ical_dump(&bytes);

        assert_eq!(result, calendar);
    }

    #[test]
    fn load_from_dump_envelope_with_corrupted_raw_dump_falls_back_to_ical() {
        let calendar     = build_test_calendar();
        let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();

        // BUILD_VERSION is None in tests, so we can't trigger the fast path directly.
        // Instead, test the iCal fallback path: even with garbage raw_dump, the envelope's
        // dump field produces the correct Calendar via iCal fallback.
        let envelope = RDBCalendarDump {
            version:  None,
            raw_dump: vec![0xFF, 0xFF, 0xFF],
            dump:     rdb_calendar,
        };

        let result = load_from_dump_envelope(envelope);

        assert_eq!(result, calendar);
    }

    #[test]
    fn load_from_legacy_ical_dump_fixture_produces_correct_calendar() {
        let bytes = std::fs::read(fixture_path("rdb_calendar_legacy.bin")).unwrap();

        let result = load_from_legacy_ical_dump(&bytes);

        assert_eq!(result, build_test_calendar());
    }

    #[test]
    fn load_mismatch_fixture_falls_back_to_ical() {
        let bytes = std::fs::read(fixture_path("rdb_calendar_dump_mismatch.bin")).unwrap();

        let envelope: RDBCalendarDump = bincode::deserialize(&bytes).unwrap();

        let result = load_from_dump_envelope(envelope);

        assert_eq!(result, build_test_calendar());
    }

    #[test]
    fn envelope_round_trip_produces_correct_calendar() {
        let calendar     = build_test_calendar();
        let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();
        let raw_dump     = bincode::serialize(&calendar).unwrap();

        let envelope = RDBCalendarDump {
            version:  BUILD_VERSION.map(String::from),
            raw_dump,
            dump:     rdb_calendar,
        };

        let bytes        = bincode::serialize(&envelope).unwrap();
        let deserialized = bincode::deserialize::<RDBCalendarDump>(&bytes).unwrap();

        let result = load_from_dump_envelope(deserialized);

        assert_eq!(result, calendar);
    }
}
