use lazy_static::lazy_static;
use redis_module::{redis_module, Context, NotifyEvent, Status, RedisString, RedisGILGuard, configuration::ConfigurationFlags};

use redical_core as core;

mod datatype;
mod commands;
mod utils;

use crate::datatype::CALENDAR_DATA_TYPE;

pub const MODULE_NAME: &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

// Wrap the allocator used to that it can be replaced for testing.
//
// This is because redis_module::alloc::RedisAlloc is not available in the test environment, so we
// stub its usage out and replace it with std::alloc::System.
pub struct RedicalAlloc;

unsafe impl std::alloc::GlobalAlloc for RedicalAlloc {
    #[cfg(not(test))]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        redis_module::alloc::RedisAlloc.alloc(layout)
    }

    #[cfg(test)]
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        std::alloc::System.alloc(layout)
    }

    #[cfg(not(test))]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        redis_module::alloc::RedisAlloc.dealloc(ptr, layout)
    }

    #[cfg(test)]
    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        std::alloc::System.dealloc(ptr, layout);
    }
}

fn notify_rdcl_cal_del_keyspace_event(ctx: &Context, calendar_uid: &RedisString) {
    if ctx.notify_keyspace_event(NotifyEvent::MODULE, "rdcl.cal_del", &calendar_uid) == Status::Err {
        ctx.log_warning(
            format!("Notify keyspace event \"rdcl.cal_set\" for calendar: \"{}\" failed", &calendar_uid).as_str()
        );
    }
}

// If key space event is either GENERIC "del" or EVICTED "evicted" and the key stores a RediCal
// Calendar datatype, notify the "rdcl.cal_del" event. This ensures keyspace notification
// subscribers are notified when a RediCal Calendar key is deleted or evicted so that they can
// respond appropriately (e.g. quickly re-import all Calendar data).
//
// This requires at least the "Kge" notification configuration to be enabled to receive these
// notifications.
fn on_keyspace_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    if matches!((event_type, event), (NotifyEvent::GENERIC, "del") | (NotifyEvent::EVICTED, "evicted")) {
        let calendar_uid = RedisString::create_from_slice(ctx.ctx, key);

        if ctx.open_key(&calendar_uid).get_value::<core::Calendar>(&CALENDAR_DATA_TYPE).is_ok() {
            notify_rdcl_cal_del_keyspace_event(ctx, &calendar_uid);
        }
    }
}

lazy_static! {
    static ref CONFIGURATION_ICAL_PARSER_TIMEOUT_MS: RedisGILGuard<i64> = RedisGILGuard::default();
}

redis_module! {
    name:       MODULE_NAME,
    version:    MODULE_VERSION,
    allocator:  (RedicalAlloc, RedicalAlloc),
    data_types: [
        CALENDAR_DATA_TYPE
    ],
    commands:   [
        ["rdcl.evt_set",         commands::redical_event_set,            "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.evt_get",         commands::redical_event_get,            "readonly",              1, 1, 1],
        ["rdcl.evt_del",         commands::redical_event_del,            "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.evt_list",        commands::redical_event_list,           "readonly",              1, 1, 1],
        ["rdcl.evi_list",        commands::redical_event_instance_list,  "readonly",              1, 1, 1],
        ["rdcl.evo_get",         commands::redical_event_override_get,   "readonly",              1, 1, 1],
        ["rdcl.evo_set",         commands::redical_event_override_set,   "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.evo_del",         commands::redical_event_override_del,   "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.evo_list",        commands::redical_event_override_list,  "readonly",              1, 1, 1],
        ["rdcl.evo_prune",       commands::redical_event_override_prune, "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.cal_set",         commands::redical_calendar_set,         "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.cal_get",         commands::redical_calendar_get,         "readonly",              1, 1, 1],
        ["rdcl.cal_query",       commands::redical_calendar_query,       "readonly",              1, 1, 1],
        ["rdcl.cal_idx_disable", commands::redical_calendar_idx_disable, "write pubsub",          1, 1, 1],
        ["rdcl.cal_idx_rebuild", commands::redical_calendar_idx_rebuild, "write pubsub deny-oom", 1, 1, 1],
    ],
    event_handlers: [
        [@GENERIC: on_keyspace_event],
        [@EVICTED: on_keyspace_event],
    ],
    configurations: [
        i64: [
            ["ical-parser-timeout-ms", &*CONFIGURATION_ICAL_PARSER_TIMEOUT_MS, 500, 1, 60000, ConfigurationFlags::DEFAULT, None],
        ],
        string: [],
        bool: [],
        enum: [],
        module_args_as_configuration: true,
    ]
}
