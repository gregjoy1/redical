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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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

    fn test_event_prune(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        let from = "20250101T090000Z";
        let until = "20250102T090000Z";

        // Recurring event that does not terminate
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            [
                "LAST-MODIFIED:20241110T110000Z",
                "DTSTART:20241231T163000Z",
                "RRULE:FREQ=DAILY;INTERVAL=1"
            ]
        );

        // Recurring event that terminates after the prune range
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_TWO",
            [
                "LAST-MODIFIED:20241110T110000Z",
                "DTSTART:20241231T163000Z",
                "RRULE:COUNT=10;FREQ=DAILY;INTERVAL=1",
            ]
        );

        // Recurring event that terminates inside prune range
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_THREE",
            [
                "LAST-MODIFIED:20241110T110000Z",
                "DTSTART:20241231T163000Z",
                "RRULE:COUNT=2;FREQ=DAILY;INTERVAL=1",
            ]
        );

        // Single event that terminates before prune range
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_FOUR",
            [
                "LAST-MODIFIED:20241110T110000Z",
                "DTSTART:20241231T163000Z",
                "RDATE:20241231T163000Z",
            ]
        );

        // Single event that terminates inside prune range
        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_FIVE",
            [
                "LAST-MODIFIED:20241110T110000Z",
                "DTSTART:20250101T123000Z",
                "RDATE:20250101T123000Z",
            ]
        );

        listen_for_keyspace_events(6480, |message_queue: &mut Arc<Mutex<VecDeque<redis::Msg>>>| {
            let number_pruned: i64 = prune_events!(connection, "TEST_CALENDAR_UID", from, until);

            assert_eq!(number_pruned, 2);

            assert_keyspace_events_published!(
                message_queue,
                [
                    (
                        format!("rdcl.evt_prune:EVENT_THREE:{}-{}", from, until),
                        "TEST_CALENDAR_UID"
                    ),
                    (
                        format!("rdcl.evt_prune:EVENT_FIVE:{}-{}", from, until),
                        "TEST_CALENDAR_UID"
                    ),
                ]
            );

            // Matching events in the prune range have been removed
            assert_event_nil!(connection, "TEST_CALENDAR_UID", "EVENT_THREE");
            assert_event_nil!(connection, "TEST_CALENDAR_UID", "EVENT_FIVE");

            // Non-matching events are not pruned
            assert_event_present!(connection, "TEST_CALENDAR_UID", "EVENT_ONE");
            assert_event_present!(connection, "TEST_CALENDAR_UID", "EVENT_TWO");
            assert_event_present!(connection, "TEST_CALENDAR_UID", "EVENT_FOUR");

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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "GEO:;",
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
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                    ],
                    [
                        "LAST-MODIFIED:20210501T090000Z",
                        "DTSTART:20210102T170000Z",
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",
                        "X-SPACES-BOOKED:12",
                        "GEO:;",
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
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                        "GEO:;",
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
        let pruned_override_count: i64 = prune_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_ONE",
            "20200105T000000Z",
            "20200110T160000Z",
        );

        assert_eq!(pruned_override_count, 3);

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
        let pruned_override_count: i64 = prune_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "20200101T000000Z",
            "20200108T160000Z",
        );

        assert_eq!(pruned_override_count, 8);

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
            let pruned_override_count: i64 = prune_event_overrides!(
                connection,
                "TEST_CALENDAR_UID",
                "20200101T000000Z",
                "20210101T000000Z",
            );

            assert_eq!(pruned_override_count, 3);

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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                "GEO:;",
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
                    "SUMMARY:Overridden event in Oxford summary text", // <= Overridden
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID", // <= Overridden
                    "CATEGORIES:OVERRIDDEN_CATEGORY",                  // <= Overridden
                    "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",          // <= Overridden
                    "GEO:51.751365550307604;-1.2601196837753945",
                ],
                [
                    "DTEND:20210106T173000Z",
                    "DTSTART:20210106T170000Z",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20210106T170000Z",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                    "DURATION:PT30M",
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY", // <= Overridden
                    "LOCATION-TYPE:X-KEY=VALUE:LOCATION_TYPE",
                    // "GEO:;",                                    // <= Overridden (removed from EventInstance because blank)
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
                    "SUMMARY:Overridden event in Oxford summary text", // <= Overridden
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID", // <= Overridden
                    "CATEGORIES:OVERRIDDEN_CATEGORY",                  // <= Overridden
                    "LOCATION-TYPE:OVERRIDDEN_LOCATION_TYPE",          // <= Overridden
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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

    fn test_event_timezone_handling(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "TIMEZONE_EVENT",
            [
                "LAST-MODIFIED:20241001T100000Z",
                "RRULE:COUNT=3;FREQ=MONTHLY;INTERVAL=4",
                "DTSTART;TZID=Europe/London:20241001T100000",
                "DTEND;TZID=Europe/London:20241001T110000",
            ],
        );

        list_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            "TIMEZONE_EVENT",
            [
                [
                    "DTEND:20241001T100000Z",
                    "DTSTART:20241001T090000Z",
                    "DURATION:PT1H",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20241001T090000Z",
                    "UID:TIMEZONE_EVENT",
                ],
                [
                    "DTEND:20250201T110000Z",
                    "DTSTART:20250201T100000Z",
                    "DURATION:PT1H",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20250201T100000Z",
                    "UID:TIMEZONE_EVENT",
                ],
                [
                    "DTEND:20250601T100000Z",
                    "DTSTART:20250601T090000Z",
                    "DURATION:PT1H",
                    "RECURRENCE-ID;VALUE=DATE-TIME:20250601T090000Z",
                    "UID:TIMEZONE_EVENT",
                ],
            ]
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "TIMEZONE_EVENT",
            "20250201T100000Z",
            [
                "LAST-MODIFIED:20241001T100000Z",
                "SUMMARY:Overridden event",
            ],
        );

        // Assert default query when mixing a couple of existing event (with overide) extrapolations.
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-TZID:Europe/London",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/London:20241001T100000",
                    ],
                    [
                        "DTEND;TZID=Europe/London:20241001T110000",
                        "DTSTART;TZID=Europe/London:20241001T100000",
                        "DURATION:PT1H",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/London:20241001T100000",
                        "UID:TIMEZONE_EVENT",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/London:20250201T100000",
                    ],
                    [
                        "DTEND;TZID=Europe/London:20250201T110000",
                        "DTSTART;TZID=Europe/London:20250201T100000",
                        "DURATION:PT1H",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/London:20250201T100000",
                        "SUMMARY:Overridden event",
                        "UID:TIMEZONE_EVENT",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/London:20250601T100000",
                    ],
                    [
                        "DTEND;TZID=Europe/London:20250601T110000",
                        "DTSTART;TZID=Europe/London:20250601T100000",
                        "DURATION:PT1H",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/London:20250601T100000",
                        "UID:TIMEZONE_EVENT",
                    ],
                ],
            ],
        );

        Ok(())
    }

    fn test_calendar_event_instance_query(connection: &mut Connection) -> Result<()> {
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM (running online)",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID_OVERRIDE",
                "CATEGORIES:CATEGORY_OVERRIDE",
                "GEO:;",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        // "GEO:;", <= Overridden (removed from EventInstance because blank)
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID_OVERRIDE",
                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM (running online)",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Bristol overridden to run in Cheltenham instead",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert simple negative querying
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-CATEGORIES-NOT:CATEGORY_ONE"
            ],
            [
                [
                    [
                        "DTSTART:20210105T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_OVERRIDE",
                        "DTEND:20210105T190000Z",
                        "DTSTART:20210105T183000Z",
                        "DURATION:PT30M",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID_OVERRIDE",
                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM (running online)",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ]
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                "GEO:51.50740017561507;-0.12698231869919185",
            ],
        );

        // Assert negative querying with more events
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(",
                    "X-CATEGORIES-NOT:CATEGORY_FOUR",
                "AND",
                    "X-LOCATION-TYPE-NOT:ONLINE",
                "AND",
                    "X-UID-NOT:EVENT_IN_READING_TUE_THU",
                ")",
            ],
            [
                [
                    [
                        "DTSTART:20210104T170000Z",
                    ],
                    [
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                        "DTEND:20210104T173000Z",
                        "DTSTART:20210104T170000Z",
                        "DURATION:PT30M",
                        "GEO:51.751365550307604;-1.2601196837753945",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
                        "SUMMARY:Overridden event in Oxford summary text",
                        "UID:EVENT_IN_OXFORD_MON_WED",
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
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID_OVERRIDE",
                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM (running online)",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                        "UID:EVENT_IN_OXFORD_MON_WED",
                    ],
                ],
                [
                    [
                        "DTSTART:20210111T170000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                        "DTEND:20210111T173000Z",
                        "DTSTART:20210111T170000Z",
                        "DURATION:PT30M",
                        "GEO:51.751365550307604;-1.2601196837753945",
                        "RECURRENCE-ID;VALUE=DATE-TIME:20210111T170000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                        "UID:EVENT_IN_OXFORD_MON_WED",
                        "X-SPACES-BOOKED:12",
                    ],
                ],
            ]
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
                    "AND",
                    "(",
                        "X-LOCATION-TYPE-NOT:DIGITAL",
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
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "X-GEO;DIST=100MI:51.454481838260214;-2.588329192623361",
                "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:20210105T180000Z",
                "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210715T180000Z",
                "(",
                    "X-UID:EVENT_IN_CHELTENHAM_TUE_THU,OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU,EVENT_IN_READING_TUE_THU,EVENT_IN_LONDON_TUE_THU",
                    "OR",
                    "(X-UID:ONLINE_EVENT_MON_WED AND X-UID:EVENT_IN_OXFORD_MON_WED)", // Impossible condition - returns nothing because an event cannot have multiple UIDs.
                ")",
                "X-LIMIT:4",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.454481838260214;-2.588329192623361",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "X-GEO-DIST:60.692838KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210105T210000",
                        "DTSTART;TZID=Europe/Vilnius:20210105T203000",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210105T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210107T200000",
                        "X-GEO-DIST:111.491952KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_THREE",
                        "DTEND;TZID=Europe/Vilnius:20210107T203000",
                        "DTSTART;TZID=Europe/Vilnius:20210107T200000",
                        "DURATION:PT30M",
                        "GEO:51.45442303961853;-0.9792277140273513",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210107T200000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "UID:EVENT_IN_READING_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210107T203000",
                        "X-GEO-DIST:60.692838KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210107T210000",
                        "DTSTART;TZID=Europe/Vilnius:20210107T203000",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210107T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210107T203000",
                        "X-GEO-DIST:60.692838KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210107T210000",
                        "DTSTART;TZID=Europe/Vilnius:20210107T203000",
                        "DURATION:PT30M",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210107T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Bristol overridden to run in Cheltenham instead",
                        "UID:OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ]
        );

        // Assert negative querying by UID
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(X-UID-NOT:EVENT_IN_CHELTENHAM_TUE_THU,OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU)",
                "X-LIMIT:4",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.454481838260214;-2.588329192623361",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "X-GEO-DIST:111.491952KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_THREE",
                        "DTEND;TZID=Europe/Vilnius:20201231T203000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "DURATION:PT30M",
                        "GEO:51.45442303961853;-0.9792277140273513",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20201231T200000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "UID:EVENT_IN_READING_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "X-GEO-DIST:170.540546KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20201231T210000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "DURATION:PT30M",
                        "GEO:51.50740017561507;-0.12698231869919185",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20201231T203000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "SUMMARY:Event in London on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_LONDON_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210104T180000",
                    ],
                    [
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        "DTEND;TZID=Europe/Vilnius:20210104T190000",
                        "DTSTART;TZID=Europe/Vilnius:20210104T180000",
                        "DURATION:PT1H",
                        "LOCATION-TYPE:DIGITAL,ONLINE",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210104T180000",
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "UID:ONLINE_EVENT_MON_WED",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210104T190000",
                        "X-GEO-DIST:97.489206KM",
                    ],
                    [
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                        "DTEND;TZID=Europe/Vilnius:20210104T193000",
                        "DTSTART;TZID=Europe/Vilnius:20210104T190000",
                        "DURATION:PT30M",
                        "GEO:51.751365550307604;-1.2601196837753945",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210104T190000",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
                        "SUMMARY:Overridden event in Oxford summary text",
                        "UID:EVENT_IN_OXFORD_MON_WED",
                    ],
                ],
            ]
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"41T180000Z\" -- Context: X-UNTIL -> DATE-TIME -> DATE",
            "rdcl.evi_query",
            "TEST_CALENDAR_UID",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210641T180000Z",
        );

        Ok(())
    }

    fn test_calendar_event_query(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        // Assert blank results when no events exist
        query_calendar_and_assert_matching_events!(
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                "LOCATION-TYPE:HALL",
                "CLASS:PUBLIC",
                "GEO:51.89936851432488;-2.078357552295971",
            ],
        );

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_BRISTOL_TUE_THU",
            [
                "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 8:30PM",
                "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T203000Z",
                "DTEND:20201231T210000Z",
                "LAST-MODIFIED:20210501T090000Z",
                "RELATED-TO;RELTYPE=SIBLING:SIBLING_UID",
                "CATEGORIES:CATEGORY_THREE",
                "LOCATION-TYPE:HOTEL",
                "CLASS:CONFIDENTIAL",
                "GEO:51.454481838260214;-2.588329192623361",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_CHELTENHAM_TUE_THU",
            "20201231T183000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden Event in Cheltenham running in London",
                "RELATED-TO;RELTYPE=PARENT:OVERIDDEN_UID",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
                "LOCATION-TYPE:OVERRIDDEN_TYPE",
                "CLASS:OVERRIDDEN",
                "GEO:51.50740017561507;-0.12698231869919185", // London
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_BRISTOL_TUE_THU",
            "20201231T203000Z",
            [
                "LAST-MODIFIED:20210501T090000Z",
                "SUMMARY:Overridden Event in Bristol running in London",
                "RELATED-TO;RELTYPE=PARENT:OVERIDDEN_UID",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
                "LOCATION-TYPE:OVERRIDDEN_TYPE",
                "CLASS:OVERRIDDEN",
                "GEO:51.50740017561507;-0.12698231869919185", // London
            ],
        );

        // Assert default empty query when mixing a couple of existing event (with overide) extrapolations.
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [],
            [
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART:20201231T203000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_THREE",
                        "CLASS:CONFIDENTIAL",
                        "DTEND:20201231T210000Z",
                        "DTSTART:20201231T203000Z",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HOTEL",
                        "RELATED-TO;RELTYPE=SIBLING:SIBLING_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 8:30PM",
                        "UID:EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );


        // Assert assert that overrides are ignored and query is only concerned with base event
        // properties.
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(",
                    "X-RELATED-TO;RELTYPE=PARENT:OVERIDDEN_UID",
                    "OR",
                    "X-CATEGORIES:OVERRIDDEN_CATEGORY",
                    "OR",
                    "X-LOCATION-TYPE:OVERRIDDEN_TYPE",
                    "OR",
                    "X-CLASS:OVERRIDDEN",
                    "OR",
                    "X-GEO;DIST=10MI:51.50740017561507;-0.12698231869919185", // London
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [],
        );

        // Assert negative querying
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-CATEGORIES-NOT:CATEGORY_ONE",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "X-GEO-DIST:57.088038KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_THREE",
                        "CLASS:CONFIDENTIAL",
                        "DTEND;TZID=Europe/Vilnius:20201231T230000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HOTEL",
                        "RELATED-TO;RELTYPE=SIBLING:SIBLING_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 8:30PM",
                        "UID:EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );


        // Assert comprehensive query
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(",
                    "(",
                        "X-GEO;DIST=10.5KM:51.454481838260214;-2.588329192623361",
                        "OR",
                        "X-CATEGORIES:CATEGORY_THREE",
                        "OR",
                        "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                    ")",
                    "AND",
                    "(",
                        "X-CLASS:CONFIDENTIAL",
                        "OR",
                         "X-LOCATION-TYPE:HALL",
                    ")",
                    "AND",
                    "(",
                        "X-UID-NOT:EVENT_IN_CHELTENHAM_TUE_THU",
                    ")",
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "X-GEO-DIST:57.088038KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_THREE",
                        "CLASS:CONFIDENTIAL",
                        "DTEND;TZID=Europe/Vilnius:20201231T230000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HOTEL",
                        "RELATED-TO;RELTYPE=SIBLING:SIBLING_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 8:30PM",
                        "UID:EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert comprehensive query focusing on the until time boundry.
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=Europe/London:20201231T193000",
                "(",
                    "(",
                        "X-GEO;DIST=10.5KM:51.454481838260214;-2.588329192623361",
                        "OR",
                        "X-CATEGORIES:CATEGORY_THREE",
                        "OR",
                        "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                    ")",
                    "AND",
                    "(",
                        "X-CLASS:CONFIDENTIAL",
                        "OR",
                         "X-LOCATION-TYPE:HALL",
                    ")",
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "X-GEO-DIST:43.390803KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND;TZID=Europe/Vilnius:20201231T210000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
        );

        // Throw another event into the mix
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
                "GEO:51.45442303961853;-0.9792277140273513",
            ],
        );

        // Assert negative querying by UID
        query_calendar_and_assert_matching_event_instances!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(X-UID-NOT:ONLINE_EVENT_MON_WED,OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU)",
                "X-LIMIT:4",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.454481838260214;-2.588329192623361",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "X-GEO-DIST:111.491952KM",
                    ],
                    [
                        "DTEND;TZID=Europe/Vilnius:20201231T203000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "DURATION:PT30M",
                        "GEO:51.45442303961853;-0.9792277140273513",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20201231T200000",
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "UID:EVENT_IN_READING_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "X-GEO-DIST:170.540546KM",
                    ],
                    [
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                        "CLASS:OVERRIDDEN",
                        "DTEND;TZID=Europe/Vilnius:20201231T210000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "DURATION:PT30M",
                        "GEO:51.50740017561507;-0.12698231869919185",
                        "LOCATION-TYPE:OVERRIDDEN_TYPE",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20201231T203000",
                        "RELATED-TO;RELTYPE=PARENT:OVERIDDEN_UID",
                        "SUMMARY:Overridden Event in Cheltenham running in London",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "X-GEO-DIST:170.540546KM",
                    ],
                    [
                        "CATEGORIES:OVERRIDDEN_CATEGORY",
                        "CLASS:OVERRIDDEN",
                        "DTEND;TZID=Europe/Vilnius:20201231T230000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T223000",
                        "DURATION:PT30M",
                        "GEO:51.50740017561507;-0.12698231869919185",
                        "LOCATION-TYPE:OVERRIDDEN_TYPE",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20201231T223000",
                        "RELATED-TO;RELTYPE=PARENT:OVERIDDEN_UID",
                        "SUMMARY:Overridden Event in Bristol running in London",
                        "UID:EVENT_IN_BRISTOL_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20210105T200000",
                        "X-GEO-DIST:111.491952KM",
                    ],
                    [
                        "DTEND;TZID=Europe/Vilnius:20210105T203000",
                        "DTSTART;TZID=Europe/Vilnius:20210105T200000",
                        "DURATION:PT30M",
                        "GEO:51.45442303961853;-0.9792277140273513",
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210105T200000",
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "UID:EVENT_IN_READING_TUE_THU",
                    ],
                ]
            ]
        );

        // Assert comprehensive impossible query
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(",
                    "X-UID:EVENT_IN_CHELTENHAM_TUE_THU,EVENT_IN_READING_TUE_THU",
                ")",
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "X-GEO-DIST:56.538417KM",
                    ],
                    [
                        "DTEND;TZID=Europe/Vilnius:20201231T203000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T200000",
                        "GEO:51.45442303961853;-0.9792277140273513",
                        "LAST-MODIFIED:20210501T090000Z",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "UID:EVENT_IN_READING_TUE_THU",
                    ],
                ],
                [
                    [
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "X-GEO-DIST:43.390803KM",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND;TZID=Europe/Vilnius:20201231T210000",
                        "DTSTART;TZID=Europe/Vilnius:20201231T203000",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert comprehensive impossible query
        query_calendar_and_assert_matching_events!(
            connection,
            "TEST_CALENDAR_UID",
            [
                "(",
                    "X-UID:ONLINE_EVENT_MON_WED",
                    "AND",
                    "X-UID:EVENT_IN_OXFORD_MON_WED",
                ")", // Impossible condition - returns nothing because an event cannot have multiple UIDs.
                "X-LIMIT:50",
                "X-OFFSET:0",
                "X-TZID:Europe/Vilnius",
                "X-ORDER-BY:DTSTART-GEO-DIST;51.55577390;-1.77971760",
            ],
            [],
        );

        assert_error_returned!(
            connection,
            "Error: - expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY) at \"41T180000Z\" -- Context: X-UNTIL -> DATE-TIME -> DATE",
            "rdcl.evi_query",
            "TEST_CALENDAR_UID",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210641T180000Z",
        );

        // Reload to ensure indexes are rebuild correctly
        assert_eq!(redis::cmd("SAVE").query(connection), Ok(String::from("OK")));

        redis::cmd("FLUSHDB")
            .query::<()>(connection)
            .with_context(|| {
                format!(
                    "failed to cleanup with FLUSHDB after running integration test function: {}", stringify!($test_function),
                )
            })?;

        assert_calendar_nil!(connection, "TEST_CALENDAR_UID");

        // Start another redis instance on a different port which will restore the test_dump.rdb
        // file and allow us to test save and load.
        let port: u16 = 6481; // Running redis port + 1
        let _guards = utils::start_redis_server_with_module("redical", port).with_context(|| "failed to start rdb dump test redis server")?;

        let mut new_connection =
            utils::get_redis_connection(port).with_context(|| "failed to connect to rdb dump test redis server")?;

        assert_calendar_present!(&mut new_connection, "TEST_CALENDAR_UID");

        // Assert category indexes are rebuilt:
        query_calendar_and_assert_matching_events!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            [
                "X-CATEGORIES:CATEGORY_ONE"
            ],
            [
                [
                    // NOTE: the original timezones are stripped out here
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert related-to indexes are rebuilt:
        query_calendar_and_assert_matching_events!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            [
                "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
            ],
            [
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert location-type indexes are rebuilt:
        query_calendar_and_assert_matching_events!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            [
                "X-LOCATION-TYPE:HALL",
            ],
            [
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
        );

        // Assert class indexes are rebuilt:
        query_calendar_and_assert_matching_events!(
            &mut new_connection,
            "TEST_CALENDAR_UID",
            [
                "X-CLASS:PUBLIC",
            ],
            [
                [
                    [
                        "DTSTART:20201231T183000Z",
                    ],
                    [
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO",
                        "CLASS:PUBLIC",
                        "DTEND:20201231T190000Z",
                        "DTSTART:20201231T183000Z",
                        "GEO:51.89936851432488;-2.078357552295971",
                        "LAST-MODIFIED:20210501T090000Z",
                        "LOCATION-TYPE:HALL",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                        "RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "UID:EVENT_IN_CHELTENHAM_TUE_THU",
                    ],
                ],
            ],
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
            );

            assert_keyspace_events_published!(message_queue, "rdcl.evo_set:EVENT_IN_OXFORD_MON_WED:20210104T170000Z LAST-MODIFIED:20210501T090000Z", "TEST_CALENDAR_UID");

            // Assert Calendar indexes working with query to strip out overridden event occurrence.
            query_calendar_and_assert_matching_event_instances!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
            let disabled_query_result: Result<Vec<String>, String> =
                redis::cmd("rdcl.evi_query")
                    .arg("TEST_CALENDAR_UID")
                    .arg("X-RELATED-TO;RELTYPE=PARENT:PARENT_UID")
                    .query(connection)
                    .map_err(|error| error.to_string());

            assert_eq!(
                disabled_query_result,
                Err(
                    String::from("rdcl.evi_query:: Queries disabled on Calendar: TEST_CALENDAR_UID because it's indexes have been disabled."),
                ),
            );

            rebuild_calendar_indexes!(connection, "TEST_CALENDAR_UID");

            assert_keyspace_events_published!(message_queue, "rdcl.cal_idx_rebuild", "TEST_CALENDAR_UID");

            // Test that querying is re-enabled and indexes work again.
            query_calendar_and_assert_matching_event_instances!(
                connection,
                "TEST_CALENDAR_UID",
                [
                    "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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

        assert_eq!(
            redis::cmd("SAVE").query(connection),
            Ok(String::from("OK")),
        );

        // std::thread::sleep(std::time::Duration::from_secs(5));

        redis::cmd("FLUSHDB")
            .query::<()>(connection)
            .with_context(|| {
                format!(
                    "failed to cleanup with FLUSHDB after running integration test function: {}", stringify!($test_function),
                )
            })?;

        assert_calendar_nil!(connection, "TEST_CALENDAR_UID");

        // Start another redis instance on a different port which will restore the test_dump.rdb
        // file and allow us to test save and load.
        let port: u16 = 6481; // Running redis port + 1
        let _guards = utils::start_redis_server_with_module("redical", port).with_context(|| "failed to start rdb dump test redis server")?;

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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
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
            // Nonsense iCal added to this to add to the burden of the iCal parser.
            // This ensures the parsing takes longer than 1ms even on speedy machines.
            let event_set_result: Result<Vec<String>, String> =
                redis::cmd("rdcl.evt_set")
                    .arg("TEST_CALENDAR_UID")
                    .arg("EVENT_IN_OXFORD_MON_WED")
                    .arg(
                        [
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                            "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T170000Z",
                            "DTEND:20201231T173000Z",
                            "LAST-MODIFIED:20210501T090000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                            "GEO:51.751365550307604;-1.2601196837753945",
                            "X-SPACES-BOOKED:12",
                            "X-FOO:FOO",
                            "X-BAR:BAR",
                            "X-BAZ:BAZ",
                            "X-BOO:BOO",
                            "X-FAR:FAR",
                            "X-FAZ:FAZ",
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
                        [
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM (OVERRIDDEN)",
                            "DESCRIPTION:Overridden event description - this should be not be present in the base event.",
                            "LAST-MODIFIED:20210501T090000Z",
                            "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                            "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UID",
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
                redis::cmd("rdcl.evi_query")
                    .arg("TEST_CALENDAR_UID")
                    .arg(
                        [
                            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:20210105T180000Z",
                            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210630T180000Z",
                            "X-GEO;DIST=105.5KM:51.55577390;-1.77971760",
                            "X-CATEGORIES:CATEGORY_ONE",
                            "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
                            "(",
                            "X-CATEGORIES:CATEGORY_TWO",
                            "X-RELATED-TO;RELTYPE=CHILD:CHILD_UID",
                            ")",
                        ].join(" ").to_string()
                    )
                    .query(connection)
                    .map_err(|redis_error| redis_error.to_string());

            assert_eq!(
                calendar_query_result,
                Err(String::from("rdcl.evi_query:: query iCal parser exceeded timeout")),
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
        test_event_prune,
        test_event_override_get_set_del_list,
        test_event_override_set_last_modified,
        test_event_override_prune,
        test_event_instance_list,
        test_event_timezone_handling,
        test_calendar_event_instance_query,
        test_calendar_event_query,
        test_calendar_index_disable_rebuild,
        test_rdb_save_load,
        test_key_expire_eviction_keyspace_events,
        test_redical_ical_parser_timeout_ms_config,
    );
}
