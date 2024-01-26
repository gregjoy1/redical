use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{Connection, RedisError, RedisResult};

mod utils;
mod macros;

#[cfg(test)]
mod integration {

    use super::*;

    use std::collections::HashMap;

    use lazy_static::lazy_static;
    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    // Run with:
    //  cargo build && cargo test -- --include-ignored
    //  cargo build && cargo test --ignored

    struct EventOverrideFixture {
        date_string: &'static str,
        properties: Vec<&'static str>,
    }

    struct EventFixture {
        properties: Vec<&'static str>,
        overrides: HashMap<&'static str, EventOverrideFixture>,
    }

    lazy_static! {
        static ref EVENT_FIXTURES: HashMap<&'static str, EventFixture> = {
            HashMap::from([
                (
                    "ONLINE_EVENT_MON_WED",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                            "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                            "DTSTART:20201231T160000Z",
                            "DTEND:20201231T170000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
                (
                    "EVENT_IN_OXFORD_MON_WED",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                            "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T170000Z",
                            "DTEND:20201231T173000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                            "GEO:51.751365550307604;-1.2601196837753945",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
                (
                    "EVENT_IN_READING_TUE_THU",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                            "RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T180000Z",
                            "DTEND:20201231T183000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY_ONE,CATEGORY_THREE",
                            "GEO:51.45442303961853;-0.9792277140273513",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
                (
                    "EVENT_IN_LONDON_TUE_THU",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Event in London on Tuesdays and Thursdays at 6:30PM",
                            "RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T183000Z",
                            "DTEND:20201231T190000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                            "GEO:51.50740017561507;-0.12698231869919185",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
                (
                    "EVENT_IN_CHELTENHAM_TUE_THU",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                            "RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T183000Z",
                            "DTEND:20201231T190000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                            "GEO:51.89936851432488;-2.078357552295971",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
                (
                    "OVERRIDDEN_EVENT_IN_BRISTOL_TUE_THU",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                            "RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                            "DTSTART:20201231T183000Z",
                            "DTEND:20201231T190000Z",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                            "GEO:51.454481838260214;-2.588329192623361",
                        ],
                        overrides: HashMap::from([
                            (
                                "OVERRIDE_SUMMARY_CATEGORY_AND_RELATED_TO",
                                EventOverrideFixture {
                                    date_string: "20210105T183000Z",
                                    properties: vec![
                                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_OVERRIDE",
                                        "CATEGORIES:CATEGORY_OVERRIDE",
                                        "SUMMARY:Overridden Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                                    ]
                                },
                            ),
                            (
                                "OVERRIDE_SUMMARY_AND_GEO",
                                EventOverrideFixture {
                                    date_string: "20210107T183000Z",
                                    properties: vec![
                                        "SUMMARY:Event in Bristol overridden to run in Cheltenham instead",
                                        "GEO:51.89936851432488;-2.078357552295971",
                                    ]
                                },
                            ),
                            (
                                "DETACHED_SUMMARY_AND_GEO_OVERRIDE",
                                EventOverrideFixture {
                                    date_string: "20210108T183000Z",
                                    properties: vec![
                                        "SUMMARY:Overridden Event in Bristol with invalid DTSTART",
                                    ]
                                },
                            ),
                        ]),
                    },
                ),
                (
                    "NON_RUNNING_EVENT_TO_DELETE",
                    EventFixture {
                        properties: vec![
                            "SUMMARY:Non-running event to delete",
                            "DTSTART:20201231T183000Z",
                            "DURATION:PT1H",
                            "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                            "CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE",
                            "GEO:51.454481838260214;-2.588329192623361",
                            "CLASS:PRIVATE",
                        ],
                        overrides: HashMap::new(),
                    },
                ),
            ])
        };
    }

    fn test_calendar_get_set_del(connection: &mut Connection) -> Result<()> {
        let calendar_uid = "TEST_CALENDAR_UID";

        set_and_assert_calendar!(connection, calendar_uid);

        assert_eq!(
            redis::cmd("DEL").arg(calendar_uid).query(connection),
            RedisResult::Ok(Value::Int(1)),
        );

        assert_calendar_nil!(connection, calendar_uid);

        Ok(())
    }

    fn test_event_get_set_del_list(connection: &mut Connection) -> Result<()> {
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
                    "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                    "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                    "DTSTART:20201231T160000Z",
                    "DTEND:20201231T170000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "UID:ONLINE_EVENT_MON_WED",
                ],
                [
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "GEO:51.751365550307604;-1.2601196837753945",
                    "UID:EVENT_IN_OXFORD_MON_WED",
                ],
            ],
        );

        // Test that rdcl.evt_del returns OK => 1 (true) when calendar event was present and deleted.
        del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "ONLINE_EVENT_MON_WED", 1);
        del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", 1);

        // Test that rdcl.evt_del returns OK => 0 (false) when trying to delete calendar events that are not present.
        del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "ONLINE_EVENT_MON_WED", 0);
        del_and_assert_event_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", 0);

        list_and_assert_matching_events!(connection, "TEST_CALENDAR_UID", []);

        Ok(())
    }

    fn test_event_override_get_set_del_list(connection: &mut Connection) -> Result<()> {
        set_and_assert_calendar!(connection, "TEST_CALENDAR_UID");

        set_and_assert_event!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            [
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                "DTSTART:20201231T170000Z",
                "DTEND:20201231T173000Z",
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                "GEO:51.751365550307604;-1.2601196837753945",
            ],
        );

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210102T170000Z",
            [
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
                    "DTSTART:20201231T170000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
                [
                    "DTSTART:20210102T170000Z",
                    "CATEGORIES:CATEGORY_ONE,OVERRIDDEN_CATEGORY",
                    "X-SPACES-BOOKED:12",
                ],
            ],
        );

        // Test that rdcl.evt_del returns OK => 1 (true) when calendar event was present and deleted.
        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210102T170000Z", 1);
        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20201231T170000Z", 1);

        // Test that rdcl.evt_del returns OK => 0 (false) when trying to delete calendar events that are not present.
        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20210102T170000Z", 0);
        del_and_assert_event_override_deletion!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", "20201231T170000Z", 0);

        list_and_assert_matching_event_overrides!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED", []);

        Ok(())
    }

    run_all_integration_tests_sequentially!(
        test_calendar_get_set_del,
        test_event_get_set_del_list,
        test_event_override_get_set_del_list,
    );

}
