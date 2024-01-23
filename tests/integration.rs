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

    // Run with:
    //  cargo test -- --include-ignored
    //  cargo test --ignored

    lazy_static! {
        static ref EVENT_FIXTURES: HashMap<&'static str, Vec<&'static str>> = {
            HashMap::from([
                (
                    "ONLINE_EVENT_MON_WED",
                    vec![
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "RRULE:FREQ=WEEKLY;UNTIL=20211231T170000Z;INTERVAL=1;BYDAY=MO,WE DTSTART:20201231T160000Z",
                        "DTEND:20201231T170000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY TWO",
                    ],
                ),
                (
                    "EVENT_IN_OXFORD_MON_WED",
                    vec![
                        "SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM",
                        "RRULE:FREQ=WEEKLY;COUNT=3;INTERVAL=1;BYDAY=MO,WE",
                        "DTSTART:20201231T170000Z",
                        "DTEND:20201231T173000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY TWO",
                        "GEO:51.751365550307604;-1.2601196837753945",
                    ],
                ),
                (
                    "EVENT_IN_READING_TUE_THU",
                    vec![
                        "SUMMARY:Event in Reading on Tuesdays and Thursdays at 6:00PM",
                        "RRULE:FREQ=WEEKLY;COUNT=3;INTERVAL=1;BYDAY=TU,TH",
                        "DTSTART:20201231T180000Z",
                        "DTEND:20201231T183000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_THREE",
                        "GEO:51.45442303961853;-0.9792277140273513",
                    ],
                ),
                (
                    "EVENT_IN_LONDON_TUE_THU",
                    vec![
                        "SUMMARY:Event in London on Tuesdays and Thursdays at 6:30PM",
                        "RRULE:FREQ=WEEKLY;COUNT=3;INTERVAL=1;BYDAY=TU,TH",
                        "DTSTART:20201231T183000Z",
                        "DTEND:20201231T190000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                        "GEO:51.50740017561507;-0.12698231869919185",
                    ],
                ),
                (
                    "EVENT_IN_CHELTENHAM_TUE_THU",
                    vec![
                        "SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM",
                        "RRULE:FREQ=WEEKLY;COUNT=3;INTERVAL=1;BYDAY=TU,TH",
                        "DTSTART:20201231T183000Z",
                        "DTEND:20201231T190000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                        "GEO:51.89936851432488;-2.078357552295971",
                    ],
                ),
                (
                    "EVENT_IN_BRISTOL_TUE_THU",
                    vec![
                        "SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM",
                        "RRULE:FREQ=WEEKLY;COUNT=3;INTERVAL=1;BYDAY=TU,TH",
                        "DTSTART:20201231T183000Z",
                        "DTEND:20201231T190000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                        "GEO:51.454481838260214;-2.588329192623361",
                    ],
                ),
            ])
        };
    }

    fn get_event_fixture_ical(uid: &str) -> Result<String> {
        let Some(content_lines) = EVENT_FIXTURES.get(uid) else {
            return Err(
                anyhow::Error::msg("Expected event fixture UID to exist").context(format!(
                    r#"Fixture event with UID "{}" does not exist"#,
                    uid
                )),
            );
        };

        Ok(content_lines.join(" "))
    }

    fn test_set_calendar_uid(connection: &mut Connection) -> Result<()> {
        let result: Vec<String> = redis::cmd("rdcl.cal_set")
            .arg("TEST_CALENDAR_UID")
            .query(connection)
            .with_context(|| "failed to set initial calendar key with rdcl.cal_set")?;

        assert_eq!(result.len(), 1);

        let expected_starts_with = r#"calendar added with UID: "TEST_CALENDAR_UID""#;
        if result[0].starts_with(expected_starts_with) == false {
            return Err(
                anyhow::Error::msg("Expected set calendar to return correctly").context(format!(
                    r#"Expected "{}" to start with "{}""#,
                    result[0], expected_starts_with
                )),
            );
        }

        let result: Vec<String> = redis::cmd("rdcl.cal_set")
            .arg("TEST_CALENDAR_UID")
            .query(connection)
            .with_context(|| "failed to set duplicate initial calendar key with rdcl.cal_set")?;

        assert_eq!(result.len(), 1);

        let expected_starts_with = r#"calendar already exists with UID: "TEST_CALENDAR_UID""#;
        if result[0].starts_with(expected_starts_with) == false {
            return Err(
                anyhow::Error::msg("Expected set calendar to return correctly").context(format!(
                    r#"Expected "{}" to start with "{}""#,
                    result[0], expected_starts_with
                )),
            );
        }

        Ok(())
    }

    fn test_set_get_events(connection: &mut Connection) -> Result<()> {
        let fixture_event_uids = [
            "ONLINE_EVENT_MON_WED",
            "EVENT_IN_OXFORD_MON_WED",
            "EVENT_IN_READING_TUE_THU",
            "EVENT_IN_LONDON_TUE_THU",
            "EVENT_IN_CHELTENHAM_TUE_THU",
            "EVENT_IN_BRISTOL_TUE_THU",
        ];

        for fixture_event_uid in fixture_event_uids {
            if let Some(_) = redis::cmd("rdcl.evt_get")
                .arg("TEST_CALENDAR_UID")
                .arg(fixture_event_uid)
                .query::<Option<String>>(connection)
                .with_context(|| {
                    format!(
                        "failed to get unset fixture event UID: {} with rdcl.evt_get",
                        fixture_event_uid
                    )
                })?
            {
                return Err(anyhow::Error::msg("Expected get event to return None")
                    .context(format!(r#"Expected "{}" not to exist"#, fixture_event_uid)));
            }

            let fixture_event_ical = get_event_fixture_ical(fixture_event_uid)?;

            let result: Vec<String> = redis::cmd("rdcl.evt_set")
                .arg("TEST_CALENDAR_UID")
                .arg(fixture_event_uid)
                .arg(fixture_event_ical)
                .query(connection)
                .with_context(|| {
                    format!(
                        "failed to set fixture event UID: {} with rdcl.evt_set",
                        fixture_event_uid
                    )
                })?;

            // TODO: Test output
            dbg!(result);

            if let Some(event_get) = redis::cmd("rdcl.evt_get")
                .arg("TEST_CALENDAR_UID")
                .arg(fixture_event_uid)
                .query::<Option<String>>(connection)
                .with_context(|| {
                    format!(
                        "failed to get set fixture event UID: {} with rdcl.evt_get",
                        fixture_event_uid
                    )
                })?
            {
                // TODO: Test output
                dbg!(event_get);
            } else {
                return Err(anyhow::Error::msg("Expected get event to return Some")
                    .context(format!(r#"Expected "{}" to exist"#, fixture_event_uid)));
            }
        }

        Ok(())
    }

    #[test]
    fn test_full_round_robin() -> Result<()> {
        let port: u16 = 6479;
        let _guards = vec![start_redis_server_with_module("redical", port)
            .with_context(|| "failed to start redis server")?];
        let mut connection =
            get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

        test_set_calendar_uid(&mut connection)?;

        test_set_get_events(&mut connection)?;

        /*
        let res: Result<Vec<i32>, RedisError> = redis::cmd("set").arg(&["key"]).query(&mut connection);

        if res.is_ok() {
            return Err(anyhow::Error::msg("Should return an error"));
        }
        */

        Ok(())
    }
}
