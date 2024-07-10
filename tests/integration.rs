use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{Connection, RedisError, RedisResult};

mod utils;
mod macros;

#[cfg(test)]
mod integration {

    use super::*;

    use pretty_assertions_sorted::{assert_ne, assert_eq, assert_eq_sorted};

    use utils::listen_for_keyspace_events;

    use std::sync::{Mutex, Arc};
    use std::collections::VecDeque;

    // Run with:
    //  cargo build && cargo test --all
    //  cargo build && cargo test --all integration

    fn test_calendar_get_set_del(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            let calendar_uid = "TEST_CALENDAR_UID";

            set_and_assert_calendar!(connection, calendar_uid);

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            assert_eq!(
                redis::cmd("DEL").arg(calendar_uid).query(connection),
                RedisResult::Ok(Value::Int(1)),
            );

            assert_calendar_nil!(connection, calendar_uid);

            assert_keyspace_events_published!(
                message_queue,
                [
                    ("rdcl.cal_del", "TEST_CALENDAR_UID"),
                    ("del",          "TEST_CALENDAR_UID"),
                ],
            );

            Ok(())
        })
    }

    fn test_event_get_set_del_list(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                    "DTSTART:20201231T160000Z",
                    "DTEND:20201231T170000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:ONLINE_EVENT_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:EVENT_IN_OXFORD_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            list_and_assert_matching_events!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    [
                        "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                        "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "DTSTART:20201231T170000Z",
                        "DTEND:20201231T173000Z",
                        "LAST-MODIFIED:20210501T090000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        "GEO:51.751365550307604;-1.2601196837753945",
                        "UID:EVENT_IN_OXFORD_MON_WED",
                    ],
                    [
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                        "DTSTART:20201231T160000Z",
                        "DTEND:20201231T170000Z",
                        "LAST-MODIFIED:20210501T090000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        "UID:ONLINE_EVENT_MON_WED",
                    ],
                ],
            );

            // Test that rdcl.evt_del returns OK => 1 (true) when calendar event was present and deleted.
            del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "ONLINE_EVENT_MON_WED", 1);
            del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", 1);

            assert_keyspace_events_published!(
                message_queue,
                [
                    ("rdcl.evt_del:ONLINE_EVENT_MON_WED",    "TEST_CALENDAR_UID"),
                    ("rdcl.evt_del:EVENT_IN_OXFORD_MON_WED", "TEST_CALENDAR_UID"),
                ],
            );

            // Test that rdcl.evt_del returns OK => 0 (false) when trying to delete calendar events that are not present.
            del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "ONLINE_EVENT_MON_WED", 0);
            del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", 0);

            assert_keyspace_events_published!(message_queue, []);

            list_and_assert_matching_events!(connection, "TEST_CALENDAR_UID", []);

            Ok(())
        })
    }

    fn test_event_set_last_modified(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:ONLINE_EVENT_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            list_and_assert_matching_events!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    [
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "DTSTART:20201231T160000Z",
                        "LAST-MODIFIED:20210501T090000Z",
                        "UID:ONLINE_EVENT_MON_WED",
                    ],
                ],
            );

            // Assert setting event with earlier LAST-MODIFIED property gets ignored.
            set_and_assert_event_not_set!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED)",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210201T090000Z", // <- Earlier LAST-MODIFIED specified!
                ],
            );

            // Assert no key-space event notifications published.
            assert_keyspace_events_published!(message_queue, []);

            // Assert not having changed!
            assert_event_present!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                ],
            );

            // Assert setting event with later LAST-MODIFIED property gets acknowledged.
            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED ONE)",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210501T120000Z", // <- Later LAST-MODIFIED specified!
                ],
            );

            // Assert event being changed!
            assert_event_present!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED ONE)",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210501T120000Z",
                ],
            );

            // Assert event being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:ONLINE_EVENT_MON_WED LAST-MODIFIED:20210501T120000Z", "TEST_CALENDAR_UID");

            // Assert setting event with later LAST-MODIFIED property (by a few milliseconds) gets
            // acknowledged.
            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED TWO)",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", // <- Later LAST-MODIFIED specified (by 123 milliseconds)!
                ],
            );

            // Assert event being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:ONLINE_EVENT_MON_WED LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", "TEST_CALENDAR_UID");

            // Assert event being changed!
            assert_event_present!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED TWO)",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", // <- Later LAST-MODIFIED specified (by 123 milliseconds)!
                ],
            );

            let expected_last_modified = format!("LAST-MODIFIED:{}", chrono::offset::Utc::now().format("%Y%m%dT%H%M%SZ"));

            // Assert setting event with no LAST-MODIFIED property specified (defaults to now -- which
            // is later than the existing).
            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM (UPDATED THREE)",
                    "DTSTART:20201231T160000Z",
                ],
                [
                    &expected_last_modified,
                ],
            );

            let expected_keyspace_event_message = format!("rdcl.evt_set:ONLINE_EVENT_MON_WED {}", &expected_last_modified);

            // Assert event being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, expected_keyspace_event_message, "TEST_CALENDAR_UID");

            Ok(())
        })
    }

    fn test_event_override_get_set_del_list(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE;X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:EVENT_IN_OXFORD_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                "20210102T170000Z",
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",
                    "X-SPACES-BOOKED:12",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:EVENT_IN_OXFORD_MON_WED:20210102T170000Z LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                "20201231T170000Z",
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:EVENT_IN_OXFORD_MON_WED:20201231T170000Z LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                [
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20201231T170000Z",
                        "SUMMARY:Overridden event in Oxford summary text",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                    ],
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20210102T170000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",
                        "X-SPACES-BOOKED:12",
                    ],
                ],
            );

            // TEST offset of 0 with limit of 1 result.
            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                0,
                1,
                [
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20201231T170000Z",
                        "SUMMARY:Overridden event in Oxford summary text",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                    ],
                ],
            );

            // TEST offset of 1 with limit of 20 results.
            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                1,
                20,
                [
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20210102T170000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",
                        "X-SPACES-BOOKED:12",
                    ],
                ],
            );

            // Test that rdcl.evo_del returns OK => 1 (true) when calendar event was present and deleted.
            del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210102T170000Z", 1);
            del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20201231T170000Z", 1);

            assert_keyspace_events_published!(
                message_queue,
                [
                    ("rdcl.evo_del:EVENT_IN_OXFORD_MON_WED:20210102T170000Z", "TEST_CALENDAR_UID"),
                    ("rdcl.evo_del:EVENT_IN_OXFORD_MON_WED:20201231T170000Z", "TEST_CALENDAR_UID"),
                ],
            );

            // Test that rdcl.evo_del returns OK => 0 (false) when trying to delete calendar events that are not present.
            del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210102T170000Z", 0);
            del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20201231T170000Z", 0);

            assert_keyspace_events_published!(message_queue, []);

            list_and_assert_matching_event_overrides!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", []);

            // Assert rdcl.evo_set date string format validation
            assert_error_returned!(
                connection,
                "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
                "rdcl.evo_set",
                "TEST_CALENDAR_UID",
                "EVENT_ONE",
                "BAD_FORMAT",
                "SUMMARY:Some text",
            );

            // Assert rdcl.evo_get date string format validation
            assert_error_returned!(
                connection,
                "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
                "rdcl.evo_get",
                "TEST_CALENDAR_UID",
                "EVENT_ONE",
                "BAD_FORMAT",
            );

            // Assert rdcl.evo_del date string format validation
            assert_error_returned!(
                connection,
                "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
                "rdcl.evo_del",
                "TEST_CALENDAR_UID",
                "EVENT_ONE",
                "BAD_FORMAT",
            );

            Ok(())
        })
    }

    fn test_event_override_set_last_modified(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "DTSTART:20201231T160000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:ONLINE_EVENT_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                "20201231T160000Z",
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "X-SPACES-BOOKED:12",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:ONLINE_EVENT_MON_WED:20201231T160000Z LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20201231T160000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "X-SPACES-BOOKED:12",
                    ],
                ],
            );

            // Assert setting event override with earlier LAST-MODIFIED property gets ignored.
            set_and_assert_event_override_not_set!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                "20201231T160000Z",
                [
                    "LAST-MODIFIED:20210201T090000Z", // <- Earlier LAST-MODIFIED specified!
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "X-SPACES-BOOKED:12",
                ],
            );

            // Assert no key-space event notifications published.
            assert_keyspace_events_published!(message_queue, []);

            // Assert not having changed!
            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20201231T160000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "X-SPACES-BOOKED:12",
                    ],
                ],
            );

            // Assert setting event override with later LAST-MODIFIED property gets acknowledged.
            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                "20201231T160000Z",
                [
                    "LAST-MODIFIED:20210501T120000Z", // <- Later LAST-MODIFIED specified!
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                    "X-SPACES-BOOKED:14",
                ],
            );

            // Assert event override being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:ONLINE_EVENT_MON_WED:20201231T160000Z LAST-MODIFIED:20210501T120000Z", "TEST_CALENDAR_UID");

            // Assert event override being changed!
            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    [
                        "LAST-MODIFIED:20210501T120000Z",
                        "DTSTART:20201231T160000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                        "X-SPACES-BOOKED:14",
                    ],
                ],
            );

            // Assert setting event override with later LAST-MODIFIED property (by a few
            // milliseconds) gets acknowledged.
            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                "20201231T160000Z",
                [
                    "LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", // <- Later LAST-MODIFIED specified (by 123 milliseconds)!
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                    "X-SPACES-BOOKED:15",
                ],
            );

            // Assert event override being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:ONLINE_EVENT_MON_WED:20201231T160000Z LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", "TEST_CALENDAR_UID");

            // Assert event override being changed!
            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                [
                    [
                        "LAST-MODIFIED;X-MILLIS=123:20210501T120000Z", // <- Later LAST-MODIFIED specified (by 123 milliseconds)!
                        "DTSTART:20201231T160000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                        "X-SPACES-BOOKED:15",
                    ],
                ],
            );

            let expected_last_modified = format!("LAST-MODIFIED:{}", chrono::offset::Utc::now().format("%Y%m%dT%H%M%SZ"));

            // Assert setting event override with no LAST-MODIFIED property specified (defaults to
            // now -- which is later than the existing).
            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "ONLINE_EVENT_MON_WED",
                "20201231T160000Z",
                [
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY_THREE",
                    "X-SPACES-BOOKED:16",
                ],
                [
                    &expected_last_modified,
                ],
            );

            let expected_keyspace_event_message = format!("rdcl.evo_set:ONLINE_EVENT_MON_WED:20201231T160000Z {}", expected_last_modified);

            // Assert event override being changed key-space event notification is published.
            assert_keyspace_events_published!(message_queue, expected_keyspace_event_message, "TEST_CALENDAR_UID");

            Ok(())
        })
    }

    fn test_event_override_prune(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        for event_uid in ["EVENT_ONE", "EVENT_TWO"] {
            set_and_assert_event!(connection, "TEST_CALENDAR_UID", event_uid, ["SUMMARY:NOT-OVERRIDDEN", "DTSTART:20200101T160000Z", "RRULE:COUNT=10;FREQ=DAILY;INTERVAL=1", "LAST-MODIFIED:20210501T090000Z"]);

            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200101T120000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200102T160000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200104T160000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200106T160000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200108T160000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200110T160000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"]);
            set_and_assert_event_override!(connection, "TEST_CALENDAR_UID", event_uid, "20200112T120000Z", ["LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"]);

            list_and_assert_matching_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                event_uid,
                [
                    ["DTSTART:20200101T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
                    ["DTSTART:20200102T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                    ["DTSTART:20200104T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                    ["DTSTART:20200106T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                    ["DTSTART:20200108T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                    ["DTSTART:20200110T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                    ["DTSTART:20200112T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
                ],
            );
        }

        // Test pruning a specific event on a calendar
        prune_event_overrides!(connection, "TEST_CALENDAR_UID", "EVENT_ONE", "20200105T000000Z", "20200110T160000Z");

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            [
                ["DTSTART:20200101T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
                ["DTSTART:20200102T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                ["DTSTART:20200104T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                ["DTSTART:20200112T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
            ],
        );

        // Test pruning some overrides on all events on a calendar
        prune_event_overrides!(connection, "TEST_CALENDAR_UID", "20200101T000000Z", "20200108T160000Z");

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            [
                ["DTSTART:20200112T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
            ],
        );

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_TWO",
            [
                ["DTSTART:20200110T160000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN"],
                ["DTSTART:20200112T120000Z", "LAST-MODIFIED:20210501T090000Z", "SUMMARY:OVERRIDDEN - DETACHED"],
            ],
        );

        // Test pruning all remaining overrides on all events on a calendar (and all associated keyspace events).
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            prune_event_overrides!(connection, "TEST_CALENDAR_UID", "20200101T000000Z", "20210101T000000Z");

            list_and_assert_matching_event_overrides!(connection, "TEST_CALENDAR_UID", "EVENT_ONE", []);
            list_and_assert_matching_event_overrides!(connection, "TEST_CALENDAR_UID", "EVENT_TWO", []);

            assert_keyspace_events_published!(
                message_queue,
                [
                    ("rdcl.evo_prune:EVENT_ONE:20200112T120000Z", "TEST_CALENDAR_UID"),
                    ("rdcl.evo_prune:EVENT_TWO:20200110T160000Z", "TEST_CALENDAR_UID"),
                    ("rdcl.evo_prune:EVENT_TWO:20200112T120000Z", "TEST_CALENDAR_UID"),
                ],
            );

            Ok(())
        })?;

        // Assert event presence validation
        assert_error_returned!(connection, "No: event with UID: 'NON_EXISTENT_EVENT' found", "rdcl.evo_prune", "TEST_CALENDAR_UID", "NON_EXISTENT_EVENT", "20200110T140000Z", "20200110T160000Z");

        // Assert min/max date string validation
        assert_error_returned!(connection, "FROM: date: 20200110T160000Z cannot be greater than the UNTIL date: 20200110T140000Z", "rdcl.evo_prune", "TEST_CALENDAR_UID", "EVENT_ONE", "20200110T160000Z", "20200110T140000Z");
        assert_error_returned!(connection, "FROM: date: 20200110T160000Z cannot be greater than the UNTIL date: 20200110T140000Z", "rdcl.evo_prune", "TEST_CALENDAR_UID",              "20200110T160000Z", "20200110T140000Z");

        // Assert date string format validation
        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
            "rdcl.evo_prune",
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            "BAD_FORMAT",
            "20200110T160000Z",
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
            "rdcl.evo_prune",
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            "20200110T160000Z",
            "BAD_FORMAT",
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
            "rdcl.evo_prune",
            "TEST_CALENDAR_UID",
            "BAD_FORMAT",
            "20200110T160000Z",
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"BAD_FORMAT\" -- Context: DATE-TIME -> DATE",
            "rdcl.evo_prune",
            "TEST_CALENDAR_UID",
            "20200110T160000Z",
            "BAD_FORMAT",
        );

        Ok(())
    }

    fn test_event_instance_list(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T170000Z",
                "DTEND:20201231T173000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                "GEO:51.751365550307604;-1.2601196837753945",
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                [
                    "DTEND:20210104T173000Z",
                    "DTSTART:20210104T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210106T173000Z",
                    "DTSTART:20210106T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210111T173000Z",
                    "DTSTART:20210111T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210111T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            1,
            1,
            [
                [
                    "DTEND:20210106T173000Z",
                    "DTSTART:20210106T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210104T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden event in Oxford summary text",
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210111T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                "X-SPACES-BOOKED:12",
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                [
                    "DTEND:20210104T173000Z",
                    "DTSTART:20210104T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Overridden event in Oxford summary text",  // <= Overridden
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID", // <= Overridden
                    "CATEGORIES:OVERRIDDEN_CATEGORY",                   // <= Overridden
                    "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",           // <= Overridden
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210106T173000Z",
                    "DTSTART:20210106T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210111T173000Z",
                    "DTSTART:20210111T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210111T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY", // <= Overridden
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                    "X-SPACES-BOOKED:12",                          // <= Overridden
                ],
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            0,
            1,
            [
                [
                    "DTEND:20210104T173000Z",
                    "DTSTART:20210104T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Overridden event in Oxford summary text",  // <= Overridden
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID", // <= Overridden
                    "CATEGORIES:OVERRIDDEN_CATEGORY",                   // <= Overridden
                    "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",           // <= Overridden
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            ],
        );

        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210104T170000Z", 1);
        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210111T170000Z", 1);

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                [
                    "DTEND:20210104T173000Z",
                    "DTSTART:20210104T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210106T173000Z",
                    "DTSTART:20210106T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210111T173000Z",
                    "DTSTART:20210111T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210111T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            2,
            1,
            [
                [
                    "DTEND:20210111T173000Z",
                    "DTSTART:20210111T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210111T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            20,
            1,
            [],
        );

        Ok(())
    }

    fn test_calendar_query(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        // Assert blank results when no events exist
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [],
            [],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_CHELTENHAM_TUE_THU",
            [
                "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T190000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                "GEO:51.89936851432488;-2.078357552295971",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
            [
                "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T190000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                "GEO:51.454481838260214;-2.588329192623361",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
            "20210105T183000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_OVERRIDE",
                "CATEGORIES:CATEGORY_OVERRIDE",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
            "20210107T183000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Event in Bristol overridden to run in Cheltenham instead",
                "GEO:51.89936851432488;-2.078357552295971",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
            "20210108T183000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Detatched override for Event in Bristol with invalid DTSTART",
            ],
        );

        // Assert default query when mixing a couple of existing event (with overide) extrapolations.
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [],
            [
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20201231T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20201231T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20210105T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND:20210105T190000Z",
                        "DTSTART:20210105T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20210105T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_OVERRIDE",
                        "DTEND:20210105T190000Z",
                        "DTSTART:20210105T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_OVERRIDE",
                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20210107T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND:20210107T190000Z",
                        "DTSTART:20210107T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210107T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20210107T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND:20210107T190000Z",
                        "DTSTART:20210107T183000Z",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210107T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Bristol overridden to run in Cheltenham instead",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "ONLINE_EVENT_MON_WED",
            [
                "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                "DTSTART:20201231T160000Z",
                "DTEND:20201231T170000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "LOCATION-TYPE:DIGITAL,ONLINE",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T170000Z",
                "DTEND:20201231T173000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                "GEO:51.751365550307604;-1.2601196837753945",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210104T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden event in Oxford summary text",
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210111T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                "X-SPACES-BOOKED:12",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_READING_TUE_THU",
            [
                "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T180000Z",
                "DTEND:20201231T183000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY_ONE,CATEGORY_THREE",
                "GEO:51.45442303961853;-0.9792277140273513",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_LONDON_TUE_THU",
            [
                "SUMMARY:Event in London on Tuesdays and Thursdays at 6:30PM",
                "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T190000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                "GEO:51.50740017561507;-0.12698231869919185",
            ],
        );

        // Assert comprehensive query with more events (and overrides) added
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:20210105T180000Z",
                "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210630T180000Z",
                "(",
                "(",
                "X-GEO;DIST=105.5KM:51.55577390;-1.77971760",
                "OR",
                "X-CATEGORIES:CATEGORY_ONE",
                "OR",
                "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                ")",
                "AND",
                "(",
                "X-CATEGORIES:CATEGORY TWO",
                "OR",
                "X-LOCATION-TYPE:ONLINE",
                ")",
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-DISTINCT:UID",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210106T180000",
                    ],
                    [
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210106T190000",
                        "DTSTART;TZID=Europe/Vilnius:20210106T180000",
                        "DURATION:PT1H",
                        "LOCATION-TYPE:DIGITAL,ONLINE",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210106T180000",
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "UID:ONLINE_EVENT_MON_WED",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210106T190000",
                        "X-GEO-DIST:41.927336KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210106T193000",
                        "DTSTART;TZID=Europe/Vilnius:20210106T190000",
                        "DURATION:PT30M",
                        "GEO:51.751365550307604;-1.2601196837753945",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210106T190000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                        "UID:EVENT_IN_OXFORD_MON_WED",
                    ],
                ],
            ],
        );

        // Assert comprehensive query with more events (and overrides) added
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:20210105T180000Z",
                "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210630T180000Z",
                "(",
                "(",
                "X-UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                "OR",
                "X-UID:EVENT_IN_CHELTENHAM_TUE_THU",
                ")",
                "OR",
                "X-UID;OP=AND:ONLINE_EVENT_MON_WED,EVENT_IN_OXFORD_MON_WED", // Impossible condition - returns nothing because an event cannot have multiple UIDs.
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-DISTINCT:UID",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "X-GEO-DIST:43.390803KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210105T210000",
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210105T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "X-GEO-DIST:57.088038KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_OVERRIDE",
                        "DTEND;TZID=Europe/Vilnius:20210105T210000",
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "DURATION:PT30M",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210105T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_OVERRIDE",
                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"41T180000Z\" -- Context: X-UNTIL -> DATE-TIME -> DATE",
            "rdcl.cal_query",
            "TEST_CALENDAR_UID",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210641T180000Z",
        );

        Ok(())
    }

    fn test_calendar_index_disable_rebuild(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            // Assert blank results when no events exist
            query_calendar_and_assert_matching_event_instances!(
                connection,
                "TEST_CALENDAR_UID",
                [],
                [],
            );

            set_and_assert_event!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=2;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evt_set:EVENT_IN_OXFORD_MON_WED LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            set_and_assert_event_override!(
                connection,
                "TEST_CALENDAR_UID",
                "EVENT_IN_OXFORD_MON_WED",
                "20210104T170000Z",
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:EVENT_IN_OXFORD_MON_WED:20210104T170000Z LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            // Assert Calendar indexes working with query to strip out overridden event occurrence.
            query_calendar_and_assert_matching_event_instances!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    "X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                ],
                [
                    [
                        [
                            "DTSTART:20210106T170000Z",
                        ],
                        [
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                            "DTEND:20210106T173000Z",
                            "DTSTART:20210106T170000Z",
                            "DURATION:PT30M",
                            "GEO:51.751365550307604;-1.2601196837753945",
                            "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                            "UID:EVENT_IN_OXFORD_MON_WED",
                        ],
                    ],
                ]
            );

            disable_calendar_indexes!(connection, "TEST_CALENDAR_UID", 1);

            assert_keyspace_events_published!(message_queue, "rdcl.cal_idx_disable", "TEST_CALENDAR_UID");

            disable_calendar_indexes!(connection, "TEST_CALENDAR_UID", 0);

            assert_keyspace_events_published!(message_queue, []);

            // Assert error reporting Calendar querying disabled
            let disabled_query_result: Result<Vec<String>, String> = redis::cmd("rdcl.cal_query").arg("TEST_CALENDAR_UID").arg("X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID").query(connection).map_err(|error| error.to_string());

            assert_eq!(
                disabled_query_result,
                Err(
                    String::from("rdcl.cal_query:: Queries disabled on Calendar: TEST_CALENDAR_UID because it's indexes have been disabled."),
                ),
            );

            rebuild_calendar_indexes!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_idx_rebuild", "TEST_CALENDAR_UID");

            // Test that querying is re-enabled and indexes work again.
            query_calendar_and_assert_matching_event_instances!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    "X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                ],
                [
                    [
                        [
                            "DTSTART:20210106T170000Z",
                        ],
                        [
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                            "DTEND:20210106T173000Z",
                            "DTSTART:20210106T170000Z",
                            "DURATION:PT30M",
                            "GEO:51.751365550307604;-1.2601196837753945",
                            "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                            "UID:EVENT_IN_OXFORD_MON_WED",
                        ],
                    ],
                ]
            );

            Ok(())
        })
    }

    fn test_rdb_save_load(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "ONLINE_EVENT_MON_WED",
            [
                "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                "DTSTART:20201231T160000Z",
                "DTEND:20201231T170000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T170000Z",
                "DTEND:20201231T173000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                "GEO:51.751365550307604;-1.2601196837753945",
            ],
        );

        list_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                ],
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                    "DTSTART:20201231T160000Z",
                    "DTEND:20201231T170000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "UID:ONLINE_EVENT_MON_WED",
                ],
            ],
        );

        list_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            1,
            5,
            [
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                    "DTSTART:20201231T160000Z",
                    "DTEND:20201231T170000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "UID:ONLINE_EVENT_MON_WED",
                ],
            ],
        );

        list_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            5,
            20,
            [],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210102T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                "X-SPACES-BOOKED:12",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20201231T170000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden event in Oxford summary text",
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
            ],
        );

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "DTSTART:20201231T170000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "DTSTART:20210102T170000Z",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "X-SPACES-BOOKED:12",
                ],
            ],
        );

        assert_eq!(redis::cmd("SAVE").query(connection), Ok(String::from("OK")));

        // std::thread::sleep(std::time::Duration::from_secs(5));

        redis::cmd("FLUSHDB")
            .query(connection)
            .with_context(|| {
                format!(
                    "failed to cleanup with FLUSHDB after running integration test function: {}", stringify!($test_function),
                )
            })?;

        assert_calendar_nil!(connection, "TEST_CALENDAR_UID");

        // Start another redis instance on a different port which will restore the test_dump.rdb
        // file and allow us to test save and load.
        let port: u16 = 6481; // Running redis port + 1
        let _guards = vec![utils::start_redis_server_with_module("redical", port)
            .with_context(|| "failed to start rdb dump test redis server")?];

        let mut new_connection =
            utils::get_redis_connection(port).with_context(|| "failed to connect to rdb dump test redis server")?;

        assert_calendar_present!(&mut new_connection, "TEST_CALENDAR_UID");

        list_and_assert_matching_events!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            [
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                ],
                [
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                    "DTSTART:20201231T160000Z",
                    "DTEND:20201231T170000Z",
                    "LAST-MODIFIED:20210501T090000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "UID:ONLINE_EVENT_MON_WED",
                ],
            ],
        );

        list_and_assert_matching_event_overrides!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "DTSTART:20201231T170000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
                [
                    "LAST-MODIFIED:20210501T090000Z",
                    "DTSTART:20210102T170000Z",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "X-SPACES-BOOKED:12",
                ],
            ],
        );

        Ok(())
    }

    fn test_key_expire_eviction_keyspace_events(connection: &mut Connection) -> Result<()> {
        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            redis::cmd("EXPIRE")
                .arg("TEST_CALENDAR_UID")
                .arg(0)
                .execute(connection);

            assert_calendar_nil!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(
                message_queue,
                [
                    ("rdcl.cal_del", "TEST_CALENDAR_UID"),
                    ("del",          "TEST_CALENDAR_UID"),
                ],
            );

            set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_set", "TEST_CALENDAR_UID");

            // Update Redis config to evict key at random if max memory exceeded.
            redis::cmd("CONFIG")
                .arg("SET")
                .arg(b"maxmemory-policy")
                .arg("allkeys-random")
                .execute(connection);

            // Update Redis configured max memory to an absurdly low amount to force key eviction.
            redis::cmd("CONFIG")
                .arg("SET")
                .arg(b"maxmemory")
                .arg("1kb")
                .execute(connection);

            assert_keyspace_events_published!(
                message_queue,
                [
                    (
                        "rdcl.cal_del",
                        "TEST_CALENDAR_UID",
                    ),
                    (
                        "evicted",
                        "TEST_CALENDAR_UID",
                    ),
                ],
            );

            // Revert Redis configured max memory back to unlimited to allow subsequent tests to be
            // uneffected by this test case.
            redis::cmd("CONFIG")
                .arg("SET")
                .arg(b"maxmemory")
                .arg("0")
                .execute(connection);

            Ok(())
        })
    }

    fn test_redical_ical_parser_timeout_ms_config(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        // Setup existing event before updating iCal parser config to unrealistic timeout to be
        // available for testing setting event occurrence override iCal parser timeout.
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T170000Z",
                "DTEND:20201231T173000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                "GEO:51.751365550307604;-1.2601196837753945",
            ],
        );


        // Update Redis RediCal config to specify iCal parser timeout after 1ms.
        redis::cmd("CONFIG")
            .arg("SET")
            .arg(b"REDICAL.ICAL-PARSER-TIMEOUT-MS")
            .arg("1")
            .execute(connection);

        {
            let event_set_result: Result<Vec<String>, String> =
                redis::cmd("rdcl.evt_set")
                    .arg("TEST_CALENDAR_UID")
                    .arg("EVENT_IN_OXFORD_MON_WED")
                    .arg(
                        &[
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                            "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T170000Z",
                            "DTEND:20201231T173000Z",
                            "LAST-MODIFIED:20210501T090000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                            "GEO:51.751365550307604;-1.2601196837753945",
                        ].join(" ").to_string()
                    )
                    .query(connection)
                    .map_err(|redis_error| redis_error.to_string());

            assert_eq!(
                event_set_result,
                Err(String::from("rdcl.evt_set:: event iCal parser exceeded timeout")),
            );
        }

        {
            let event_occurrence_override_set_result: Result<Vec<String>, String> =
                redis::cmd("rdcl.evo_set")
                    .arg("TEST_CALENDAR_UID")
                    .arg("EVENT_IN_OXFORD_MON_WED")
                    .arg("20210102T170000Z")
                    .arg(
                        &[
                            "LAST-MODIFIED:20210501T090000Z",
                            "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                            "X-SPACES-BOOKED:12",
                        ].join(" ").to_string()
                    )
                    .query(connection)
                    .map_err(|redis_error| redis_error.to_string());

            assert_eq!(
                event_occurrence_override_set_result,
                Err(String::from("rdcl.evo_set:: event occurrence override iCal parser exceeded timeout")),
            );
        }

        {
            let calendar_query_result: Result<Vec<String>, String> =
                redis::cmd("rdcl.cal_query")
                    .arg("TEST_CALENDAR_UID")
                    .arg(
                        &[
                            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:20210105T180000Z",
                            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210630T180000Z",
                            "X-GEO;DIST=105.5KM:51.55577390;-1.77971760",
                            "X-CATEGORIES:CATEGORY_ONE",
                            "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        ].join(" ").to_string()
                    )
                    .query(connection)
                    .map_err(|redis_error| redis_error.to_string());

            assert_eq!(
                calendar_query_result,
                Err(String::from("rdcl.cal_query:: query iCal parser exceeded timeout")),
            );
        }

        // Restore Redis RediCal config to specify iCal parser timeout back to 500ms.
        redis::cmd("CONFIG")
            .arg("SET")
            .arg(b"REDICAL.ICAL-PARSER-TIMEOUT-MS")
            .arg("1")
            .execute(connection);

        Ok(())
    }

    run_all_integration_tests_sequentially!(
        test_calendar_get_set_del,
        test_event_get_set_del_list,
        test_event_set_last_modified,
        test_event_override_get_set_del_list,
        test_event_override_set_last_modified,
        test_event_override_prune,
        test_event_instance_list,
        test_calendar_query,
        test_calendar_index_disable_rebuild,
        test_rdb_save_load,
        test_key_expire_eviction_keyspace_events,
        test_redical_ical_parser_timeout_ms_config,
    );

}
