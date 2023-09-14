use redis_module::{redis_module, Context, NotifyEvent};

mod commands;
mod data_types;
mod parsers;

use data_types::{EVENT_DATA_TYPE, CALENDAR_DATA_TYPE};

fn on_event(ctx: &Context, event_type: NotifyEvent, event: &str, key: &[u8]) {
    ctx.log_notice(
        format!(
            "Received event: {:?} on key: {} via event: {}",
            event_type,
            std::str::from_utf8(key).unwrap(),
            event
        ).as_str()
    );
}

pub const MODULE_NAME:    &str = "RediCal";
pub const MODULE_VERSION: u32 = 1;

#[cfg(not(test))]
redis_module! {
    name:       MODULE_NAME,
    version:    MODULE_VERSION,
    allocator:  (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [
        EVENT_DATA_TYPE,
        CALENDAR_DATA_TYPE
    ],
    commands:   [
        ["rdcl.evt_set",   commands::redical_event_set, "", 0, 0, 0],
        ["rdcl.evt_get",   commands::redical_event_get, "", 0, 0, 0],
        ["rdcl.evi_list",  commands::redical_event_instance_list, "", 0, 0, 0],
        ["rdcl.evo_set",   commands::redical_event_override_set, "", 0, 0, 0],
        ["rdcl.evo_del",   commands::redical_event_override_del, "", 0, 0, 0],
        ["rdcl.cal_set",   commands::redical_calendar_set, "", 0, 0, 0],
        ["rdcl.cal_query", commands::redical_calendar_query, "", 0, 0, 0],
    ],
    event_handlers: [
        [@STRING: on_event],
    ]
}
