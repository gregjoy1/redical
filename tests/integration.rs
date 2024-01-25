use crate::utils::{get_redis_connection, start_redis_server_with_module};
use anyhow::Context;
use anyhow::Result;
use redis::Value;
use redis::{Connection, RedisError, RedisResult};

mod utils;

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

    macro_rules! assert_matching_ical_properties {
        ($redis_result:expr, $expected_result:expr $(,)*) => {
            let mut actual_result = $redis_result.to_owned();
            let mut expected_result = $expected_result.to_owned();

            assert_eq!(actual_result.len(), expected_result.len());

            actual_result.sort();
            expected_result.sort();

            assert_eq_sorted!(actual_result, expected_result);
        }
    }

    macro_rules! assert_matching_ical_components {
        ($redis_result:expr, $expected_result:expr $(,)*) => {
            // assert_eq!($redis_result.len(), $expected_result.len());

            let mut actual_result: Vec<Vec<String>> = $redis_result;
            let mut expected_result: Vec<Vec<String>> = $expected_result;

            // Crudely sort multi-dimensional vec so assert only cares about presence, not order.
            actual_result.iter_mut().for_each(|properties| properties.sort());
            expected_result.iter_mut().for_each(|properties| properties.sort());

            actual_result.sort();
            expected_result.sort();

            assert_eq_sorted!(actual_result, expected_result);
        }
    }

    macro_rules! assert_calendar_present {
        ($connection:expr, $calendar_uid:expr $(,)*) => {
            let calendar_get_result: Vec<String> = redis::cmd("rdcl.cal_get")
                .arg($calendar_uid)
                .query($connection)
                .with_context(|| {
                    format!(
                        "failed to get calendar with UID: '{}' via rdcl.cal_get", $calendar_uid,
                    )
                })?;

            assert_matching_ical_properties!(
                calendar_get_result,
                vec![
                    format!("UID:{}", $calendar_uid),
                ],
            );
        }
    }

    macro_rules! assert_calendar_nil {
        ($connection:expr, $calendar_uid:expr $(,)*) => {
            assert_eq!(redis::cmd("rdcl.cal_get").arg($calendar_uid).query($connection), RedisResult::Ok(Value::Nil));
        }
    }

    macro_rules! set_and_assert_calendar {
        ($connection:expr, $calendar_uid:expr $(,)*) => {
            assert_calendar_nil!($connection, $calendar_uid);

            let calendar_set_result: Vec<String> = redis::cmd("rdcl.cal_set")
                .arg($calendar_uid)
                .query($connection)
                .with_context(|| {
                    format!(r#"failed to set initial calendar UID: "{}" via rdcl.cal_set"#, $calendar_uid)
                })?;

            assert_matching_ical_properties!(
                calendar_set_result,
                vec![
                    format!("UID:{}", $calendar_uid),
                ],
            );

            assert_calendar_present!($connection, $calendar_uid);
        }
    }

    macro_rules! assert_event_present {
        ($connection:expr, $calendar_uid:expr, $event_uid:expr, [$($ical_property:expr),+ $(,)*] $(,)*) => {
            let event_get_result: Vec<String> = redis::cmd("rdcl.evt_get")
                .arg($calendar_uid)
                .arg($event_uid)
                .query($connection)
                .with_context(|| {
                    format!(
                        "failed to get set fixture event with UID: '{}' via rdcl.evt_get", $event_uid,
                    )
                })?;

            assert_matching_ical_properties!(
                event_get_result,
                vec![
                    format!("UID:{}", $event_uid),
                    $(
                        String::from($ical_property),
                    )+
                ],
            );
        }
    }

    macro_rules! assert_event_nil {
        ($connection:expr, $calendar_uid:expr, $event_uid:expr $(,)*) => {
            assert_eq!(redis::cmd("rdcl.evt_get").arg($calendar_uid).arg($event_uid).query($connection), RedisResult::Ok(Value::Nil));
        }
    }

    macro_rules! set_and_assert_event {
        ($connection:expr, $calendar_uid:expr, $event_uid:expr, [$($ical_property:expr),+ $(,)*] $(,)*) => {
            assert_event_nil!(
                $connection,
                $calendar_uid,
                $event_uid,
            );

            let mut ical_properties: Vec<String> = vec![
                $(
                    String::from($ical_property),
                )+
            ];

            let joined_ical_properties = ical_properties.join(" ");

            let event_set_result: Vec<String> = redis::cmd("rdcl.evt_set")
                .arg("TEST_CALENDAR_UID")
                .arg($event_uid)
                .arg(joined_ical_properties)
                .query($connection)
                .with_context(|| {
                    format!(
                        "failed to set fixture event with UID: '{}' via rdcl.evt_set", $event_uid,
                    )
                })?;

            ical_properties.push(format!("UID:{}", $event_uid));

            assert_matching_ical_properties!(event_set_result, ical_properties);

            assert_event_present!(
                $connection,
                $calendar_uid, 
                $event_uid,
                [
                    $(
                        $ical_property,
                    )+
                ],
            );
        }
    }

    macro_rules! list_and_assert_matching_events {
        ($connection:expr, $calendar_uid:expr, [] $(,)*) => {
            let event_list_result: Vec<Vec<String>> = redis::cmd("rdcl.evt_list")
                .arg($calendar_uid)
                .query($connection)
                .with_context(|| {
                    format!(
                        "failed to list calendar: '{}' events via rdcl.evt_list", $calendar_uid,
                    )
                })?;

            let expected_event_list_result: Vec<Vec<String>> = vec![];

            assert_eq!(event_list_result, expected_event_list_result);
        };

        ($connection:expr, $calendar_uid:expr, [$([$($ical_component_property:expr),+ $(,)*]),+ $(,)*] $(,)*) => {
            let expected_event_list_result: Vec<Vec<String>> = vec![
                $(
                    vec![
                        $(
                            String::from($ical_component_property),
                        )+
                    ],
                )+
            ];

            let event_list_result: Vec<Vec<String>> = redis::cmd("rdcl.evt_list")
                .arg($calendar_uid)
                .query($connection)
                .with_context(|| {
                    format!(
                        "failed to list calendar: '{}' events via rdcl.evt_list", $calendar_uid,
                    )
                })?;

            assert_matching_ical_components!(event_list_result, expected_event_list_result);
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

        // Test that rdcl.evt_del returns OK => true when calendar event was present and deleted.
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg("ONLINE_EVENT_MON_WED").query(connection), RedisResult::Ok(Value::Int(1)));

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
            ],
        );

        // Test that rdcl.evt_del returns OK => true when calendar event was present and deleted.
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg("EVENT_IN_OXFORD_MON_WED").query(connection), RedisResult::Ok(Value::Int(1)));

        list_and_assert_matching_events!(connection, "TEST_CALENDAR_UID", []);

        // Test that rdcl.evt_del returns OK => false when trying to delete calendar events that are not present.
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg("ONLINE_EVENT_MON_WED").query(connection), RedisResult::Ok(Value::Int(0)));
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg("EVENT_IN_OXFORD_MON_WED").query(connection), RedisResult::Ok(Value::Int(0)));

        assert_event_nil!(connection, "TEST_CALENDAR_UID", "ONLINE_EVENT_MON_WED");
        assert_event_nil!(connection, "TEST_CALENDAR_UID", "EVENT_IN_OXFORD_MON_WED");

        Ok(())
    }

    macro_rules! run_all_integration_tests_sequentially {
        ($($test_function:ident),+ $(,)*) => {
            #[test]
            fn test_all_integration_tests_sequentially() -> Result<()> {
                let port: u16 = 6480;
                let _guards = vec![start_redis_server_with_module("redical", port)
                    .with_context(|| "failed to start test redis server")?];

                let mut connection =
                    get_redis_connection(port).with_context(|| "failed to connect to test redis server")?;

                test_calendar_get_set_del(&mut connection)?;

                $(
                    $test_function(&mut connection)?;

                    redis::cmd("FLUSHDB")
                        .query(&mut connection)
                        .with_context(|| {
                            format!(
                                "failed to cleanup with FLUSHDB after running integration test function: {}", stringify!($test_function),
                            )
                        })?;
                )+

                Ok(())
            }
        }
    }

    run_all_integration_tests_sequentially!(
        test_calendar_get_set_del,
        test_event_get_set_del_list,
    );

}
