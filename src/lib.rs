use redis_module::{redis_module, Context, NotifyEvent};

mod core;
mod redis;

#[cfg(test)]
mod testing;

use redis::calendar_data_type::CALENDAR_DATA_TYPE;

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
        ["rdcl.evt_set",   redis::commands::redical_event_set,           "", 0, 0, 0],
        ["rdcl.evt_get",   redis::commands::redical_event_get,           "", 0, 0, 0],
        ["rdcl.evt_del",   redis::commands::redical_event_del,           "", 0, 0, 0],
        ["rdcl.evt_list",  redis::commands::redical_event_list,          "", 0, 0, 0],
        ["rdcl.evi_list",  redis::commands::redical_event_instance_list, "", 0, 0, 0],
        ["rdcl.evo_set",   redis::commands::redical_event_override_set,  "", 0, 0, 0],
        ["rdcl.evo_del",   redis::commands::redical_event_override_del,  "", 0, 0, 0],
        ["rdcl.evo_list",  redis::commands::redical_event_override_list, "", 0, 0, 0],
        ["rdcl.cal_set",   redis::commands::redical_calendar_set,        "", 0, 0, 0],
        ["rdcl.cal_get",   redis::commands::redical_calendar_get,        "", 0, 0, 0],
        ["rdcl.cal_query", redis::commands::redical_calendar_query,      "", 0, 0, 0],
    ],
    event_handlers: [
        [@STRING: on_event],
    ]
}
