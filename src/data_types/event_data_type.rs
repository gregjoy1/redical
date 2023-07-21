use crate::Event;

use redis_module::{
    native_types::RedisType, raw, RedisModuleIO, RedisModuleString, RedisModuleTypeMethods,
};

use std::{
    ffi::{c_int, c_void},
    ptr::null_mut,
};

pub const EVENT_DATA_TYPE_NAME:    &str = "RICAL_EVT";
pub const EVENT_DATA_TYPE_VERSION: i32  = 1;

pub static EVENT_DATA_TYPE: RedisType = RedisType::new(
    EVENT_DATA_TYPE_NAME,
    EVENT_DATA_TYPE_VERSION,
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

    let event: Event = bincode::deserialize(&bytes).unwrap();

    Box::into_raw(Box::new(event)).cast::<libc::c_void>()
}

pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let event = unsafe { &*(value as *mut Event) };

    let bytes: Vec<u8> = bincode::serialize(&event).unwrap();

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
        println!("Event data type - free - is null");
        // on Redis 6.0 we might get a NULL value here, so we need to handle it.
        return;
    }

    let event = value as *mut Event;

    println!("Event data type - free - event : {:#?}", event);

    drop(Box::from_raw(event));
}

unsafe extern "C" fn copy(
    _fromkey: *mut RedisModuleString,
    _tokey:   *mut RedisModuleString,
    value:    *const c_void,
) -> *mut c_void {
    let event = unsafe { &*(value as *mut Event) };

    let event_cloned = event.clone();

    Box::into_raw(Box::new(event_cloned)).cast::<c_void>()
}
