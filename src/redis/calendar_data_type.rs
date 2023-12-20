use crate::core::Calendar;

use redis_module::{
    native_types::RedisType, raw, RedisModuleIO, RedisModuleString, RedisModuleTypeMethods,
};

use std::{
    ffi::{c_int, c_void},
    ptr::null_mut,
};

pub const CALENDAR_DATA_TYPE_NAME:    &str = "RICAL_CAL";
pub const CALENDAR_DATA_TYPE_VERSION: i32  = 1;

pub static CALENDAR_DATA_TYPE: RedisType = RedisType::new(
    CALENDAR_DATA_TYPE_NAME,
    CALENDAR_DATA_TYPE_VERSION,
    RedisModuleTypeMethods {
        version:           redis_module::TYPE_METHOD_VERSION,
        rdb_load:          Some(rdb_load),
        rdb_save:          Some(rdb_save),
        aof_rewrite:       Some(aof_rewrite),
        mem_usage:         Some(mem_usage),
        digest:            None,
        free:              Some(free),
        aux_load:          None,
        aux_save:          None,
        aux_save_triggers: 0,
        free_effort:       None,
        unlink:            None,
        copy:              Some(copy),
        defrag:            None,

        copy2:             None,
        free_effort2:      None,
        mem_usage2:        None,
        unlink2:           None,
    }
);

pub extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, _encver: c_int) -> *mut c_void {
    let Ok(buffer) = raw::load_string_buffer(rdb) else {
        return null_mut();
    };

    let bytes: &[u8] = buffer.as_ref();

    let calendar: Calendar = bincode::deserialize(&bytes).unwrap();

    Box::into_raw(Box::new(calendar)).cast::<libc::c_void>()
}

pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let bytes: Vec<u8> = bincode::serialize(&calendar).unwrap();

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
    todo!();
}

unsafe extern "C" fn free(value: *mut c_void) {
    if value.is_null() {
        println!("Calendar data type - free - is null");
        // on Redis 6.0 we might get a NULL value here, so we need to handle it.
        return;
    }

    let calendar = value as *mut Calendar;

    println!("Calendar data type - free - calendar : {:#?}", calendar);

    drop(Box::from_raw(calendar));
}

unsafe extern "C" fn copy(
    _fromkey: *mut RedisModuleString,
    _tokey:   *mut RedisModuleString,
    value:    *const c_void,
) -> *mut c_void {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let calendar_cloned = calendar.clone();

    Box::into_raw(Box::new(calendar_cloned)).cast::<c_void>()
}
