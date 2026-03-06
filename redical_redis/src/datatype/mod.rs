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

use rdb_data::{RDBCalendar, RDBCalendarDump};

const BUILD_VERSION: Option<&str> = option_env!("GIT_SHA");

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
        logging::log_warning("RDB load: failed to read string buffer from RDB");
        return null_mut();
    };

    let bytes: &[u8] = buffer.as_ref();

    let calendar = match bincode::deserialize::<RDBCalendarDump>(bytes) {
        Ok(envelope) => load_from_envelope(envelope),

        Err(_) => {
            logging::log_notice("RDB calendar load: not current format, trying legacy");
            load_legacy(bytes)
        },
    };

    Box::into_raw(Box::new(calendar)).cast::<c_void>()
}

pub(crate) fn load_from_envelope(envelope: RDBCalendarDump) -> Calendar {
    let version_match = match (&envelope.version, BUILD_VERSION) {
        (Some(saved), Some(current)) if saved == current => true,
        _ => false,
    };

    if !version_match {
        let saved   = envelope.version.as_deref().unwrap_or("None");
        let current = BUILD_VERSION.unwrap_or("None");

        logging::log_warning(
            &format!("RDB load: fast path skipped (version build digest mismatch: {saved} vs {current})")
        );
    } else {
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<Calendar, String> {
            let mut calendar = bincode::deserialize::<Calendar>(&envelope.raw_dump)
                .map_err(|e| format!("{e}"))?;

            calendar.rebuild_indexes()
                .map_err(|e| format!("{e}"))?;

            Ok(calendar)
        }));

        match result {
            Ok(Ok(calendar)) => {
                logging::log_debug("RDB load: fast path OK");
                return calendar;
            },

            Ok(Err(error)) => {
                logging::log_warning(
                    &format!("RDB load: fast path failed ({error}), using iCal fallback")
                );
            },

            Err(panic_payload) => {
                let message = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    String::from("unknown panic")
                };

                logging::log_warning(
                    &format!("RDB load: fast path panicked (payload: '{message}'), using iCal fallback")
                );
            },
        }
    }

    Calendar::try_from(&envelope.dump).unwrap_or_else(|error| {
        panic!("RDB load: iCal fallback failed: {error}")
    })
}

pub(crate) fn load_legacy(bytes: &[u8]) -> Calendar {
    let rdb_calendar: RDBCalendar = bincode::deserialize(bytes).unwrap();

    Calendar::try_from(&rdb_calendar).unwrap_or_else(|error| {
        panic!("rdb_load failed for Calendar with error: {error:#?}")
    })
}

pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let raw_dump = bincode::serialize(calendar).unwrap();

    let rdb_calendar = RDBCalendar::try_from(calendar).unwrap_or_else(|error| {
        panic!("rdb_save failed for Calendar with error: {error:#?}");
    });

    let envelope = RDBCalendarDump {
        version:  BUILD_VERSION.map(String::from),
        raw_dump,
        dump:     rdb_calendar,
    };

    let bytes = bincode::serialize(&envelope).unwrap();

    raw::save_slice(rdb, &bytes);
}

unsafe extern "C" fn aof_rewrite(
    _aof: *mut RedisModuleIO,
    _key: *mut RedisModuleString,
    _value: *mut c_void,
) {
    // A Calendar is built from multiple commands (RICAL_SET, property decorators, etc.)
    // so there is no single Redis command that can reconstruct it. AOF rewrite is a no-op
    // until a multi-command emit strategy is designed in a future version.
}

unsafe extern "C" fn mem_usage(_value: *const c_void) -> usize {
    0
}

unsafe extern "C" fn free(value: *mut c_void) {
    if value.is_null() {
        // on Redis 6.0 we might get a NULL value here, so we need to handle it.
        return;
    }

    let calendar = value as *mut Calendar;

    // println!("Calendar data type - free - calendar : {:#?}", &calendar);

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
