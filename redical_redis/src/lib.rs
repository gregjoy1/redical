use redis_module::{redis_module, Context, NotifyEvent};

use redical_core as core;

mod datatype;
mod commands;

use crate::datatype::CALENDAR_DATA_TYPE;

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    ctx.log_notice(
        format!(
            "Received event: {:?} on key: {} via event: {}",
            event_type,
            std::str::from_utf8(key).unwrap(),
            event
        )
        .as_str(),
    );
}

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
        ["rdcl.cal_set",         commands::redical_calendar_set,         "write pubsub deny-oom", 1, 1, 1],
        ["rdcl.cal_get",         commands::redical_calendar_get,         "readonly",              1, 1, 1],
        ["rdcl.cal_query",       commands::redical_calendar_query,       "readonly",              1, 1, 1],
        ["rdcl.cal_idx_disable", commands::redical_calendar_idx_disable, "write pubsub",          1, 1, 1],
        ["rdcl.cal_idx_rebuild", commands::redical_calendar_idx_rebuild, "write pubsub deny-oom", 1, 1, 1],
    ],
    event_handlers: [
        [@STRING: on_event],
    ]
}
