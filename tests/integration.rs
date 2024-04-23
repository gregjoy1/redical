use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{Connection, RedisError, RedisResult};

mod utils;
mod macros;

#[cfg(test)]
mod integration {

    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    // Run with:
    //  cargo build && cargo test --all
    //  cargo build && cargo test --all integration

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
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
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
                    "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                    "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
                    "UID:ONLINE_EVENT_MON_WED",
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

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            0,
            1,
            [
                [
                    "DTSTART:20201231T170000Z",
                    "SUMMARY:Overridden event in Oxford summary text",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                    "CATEGORIES:OVERRIDDEN_CATEGORY",
                ],
            ],
        );

        list_and_assert_matching_event_overrides!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            1,
            20,
            [
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

    fn test_event_instance_list(connection: &mut Connection) -> Result<()> {
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
                "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_ONLINE",
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

        set_and_assert_event_override!(
            connection,
            "TEST_CALENDAR_UID",
            "EVENT_IN_OXFORD_MON_WED",
            "20210104T170000Z",
            [
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
                "X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID_ONLINE",
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
                        "RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Vilnius:20210106T180000",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID_ONLINE",
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

        // Assert bad date
        let bad_query_result: Result<Vec<String>, RedisError> = redis::cmd("rdcl.cal_query").arg("TEST_CALENDAR_UID").arg("X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:20210641T180000Z").query(connection);

        if bad_query_result.is_ok() {
            return Err(anyhow::Error::msg("Should return an error"));
        }

        Ok(())
    }

    fn test_calendar_index_disable_rebuild(connection: &mut Connection) -> Result<()> {
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
            "EVENT_IN_OXFORD_MON_WED",
            [
                "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                "RRULE:BYDAY=MO,WE;COUNT=2;FREQ=WEEKLY;INTERVAL=1",
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
            "20210104T170000Z",
            [
                "SUMMARY:Overridden event in Oxford summary text",
                "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID",
                "CATEGORIES:OVERRIDDEN_CATEGORY",
            ],
        );

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
        disable_calendar_indexes!(connection, "TEST_CALENDAR_UID", 0);

        // Assert error reporting Calendar querying disabled
        let disabled_query_result: Result<Vec<String>, String> = redis::cmd("rdcl.cal_query").arg("TEST_CALENDAR_UID").arg("X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID").query(connection).map_err(|error| error.to_string());

        assert_eq!(
            disabled_query_result,
            Err(
                String::from("rdcl.cal_query:: Queries disabled on Calendar: TEST_CALENDAR_UID because it's indexes have been disabled."),
            ),
        );

        rebuild_calendar_indexes!(connection, "TEST_CALENDAR_UID");

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
                    "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                    "RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1",
                    "DTSTART:20201231T170000Z",
                    "DTEND:20201231T173000Z",
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

        Ok(())
    }

    run_all_integration_tests_sequentially!(
        test_calendar_get_set_del,
        test_event_get_set_del_list,
        test_event_override_get_set_del_list,
        test_event_instance_list,
        test_calendar_query,
        test_calendar_index_disable_rebuild,
        test_rdb_save_load,
    );

}
