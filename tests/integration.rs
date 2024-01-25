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

    macro_rules! assert_matching_ical {
        ($redis_result:ident, $(($ical_content_line:expr)),+ $(,)*) => {
            assert_eq_sorted!(
                $redis_result,
                vec![
                    $(
                        $ical_content_line,
                    )+
                ],
            );
        }
    }

    macro_rules! assert_matching_ical_vec {
        ($redis_result:expr, $expected_result:expr) => {
            assert_eq_sorted!(
                $redis_result.to_owned().sort(),
                $expected_result.to_owned().sort(),
            );
        }
    }

    lazy_static! {
        static ref EVENT_FIXTURES: HashMap<&'static str, Vec<&'static str>> = {
            HashMap::from([
                (
                    "ONLINE_EVENT_MON_WED",
                    vec![
                        "SUMMARY:Online Event on Mondays and Wednesdays at 4:00PM",
                        "RRULE:BYDAY=MO,WE;FREQ=WEEKLY;INTERVAL=1;UNTIL=20211231T170000Z",
                        "DTSTART:20201231T160000Z",
                        "DTEND:20201231T170000Z",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY TWO,CATEGORY_ONE",
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
                (
                    "NON_RUNNING_EVENT_TO_DELETE",
                    vec![
                        "SUMMARY:Non-running event to delete",
                        "DTSTART:20201231T183000Z",
                        "DURATION:PT1H",
                        "RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                        "GEO:51.454481838260214;-2.588329192623361",
                        "CLASS:PRIVATE",
                    ],
                ),
            ])
        };
    }

    fn get_event_fixture_ical_parts(uid: &str, include_uid: bool) -> Result<Vec<String>> {
        let Some(content_lines) = EVENT_FIXTURES.get(uid) else {
            return Err(
                anyhow::Error::msg("Expected event fixture UID to exist").context(format!(
                    r#"Fixture event with UID "{}" does not exist"#,
                    uid
                )),
            );
        };

        let mut content_lines: Vec<String> = content_lines.to_owned().into_iter().map(String::from).collect();

        if include_uid {
            content_lines.push(format!("UID:{uid}"));
        }

        Ok(content_lines)
    }

    fn get_event_fixture_ical(uid: &str, include_uid: bool) -> Result<String> {
        Ok(get_event_fixture_ical_parts(uid, include_uid)?.join(" "))
    }

    fn test_set_calendar(connection: &mut Connection) -> Result<()> {
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

    fn test_set_get_fixture_event(connection: &mut Connection, fixture_event_uid: &str) -> Result<()> {
        let fixture_event_ical = get_event_fixture_ical(fixture_event_uid, false)?;

        assert_eq!(redis::cmd("rdcl.evt_get").arg("TEST_CALENDAR_UID").arg(fixture_event_uid).query(connection), RedisResult::Ok(Value::Nil));

        let event_set_result: Vec<String> = redis::cmd("rdcl.evt_set")
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

        assert_matching_ical_vec!(event_set_result, get_event_fixture_ical_parts(fixture_event_uid, true)?);

        let event_get_result: Vec<String> = redis::cmd("rdcl.evt_get")
            .arg("TEST_CALENDAR_UID")
            .arg(fixture_event_uid)
            .query(connection)
            .with_context(|| {
                format!(
                    "failed to get set fixture event UID: {} with rdcl.evt_get",
                    fixture_event_uid
                )
            })?;

        assert_matching_ical_vec!(event_get_result, get_event_fixture_ical_parts(fixture_event_uid, true)?);

        Ok(())
    }

    fn test_set_get_events(connection: &mut Connection) -> Result<()> {
        for fixture_event_uid in [
            "ONLINE_EVENT_MON_WED",
            "EVENT_IN_OXFORD_MON_WED",
            "EVENT_IN_READING_TUE_THU",
            "EVENT_IN_LONDON_TUE_THU",
            "EVENT_IN_CHELTENHAM_TUE_THU",
            "EVENT_IN_BRISTOL_TUE_THU",
        ] {
            test_set_get_fixture_event(connection, fixture_event_uid)?;
        }

        assert_eq!(redis::cmd("rdcl.evt_get").arg("TEST_CALENDAR_UID").arg("NON_EXISTENT").query(connection), RedisResult::Ok(Value::Nil));

        Ok(())
    }

    fn test_del_event(connection: &mut Connection) -> Result<()> {
        let fixture_event_uid = "NON_RUNNING_EVENT_TO_DELETE";

        // Test that rdcl.evt_del returns OK => false when calendar event not present.
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg(fixture_event_uid).query(connection), RedisResult::Ok(Value::Int(0)));

        // Create and test presence of "NON_RUNNING_EVENT_TO_DELETE" fixture event about to be deleted.
        test_set_get_fixture_event(connection, fixture_event_uid)?;

        // Test that rdcl.evt_del returns OK => true when calendar event was present and deleted.
        assert_eq!(redis::cmd("rdcl.evt_del").arg("TEST_CALENDAR_UID").arg(fixture_event_uid).query(connection), RedisResult::Ok(Value::Int(1)));

        // Test that "NON_RUNNING_EVENT_TO_DELETE" was actually deleted
        assert_eq!(redis::cmd("rdcl.evt_get").arg("TEST_CALENDAR_UID").arg(fixture_event_uid).query(connection), RedisResult::Ok(Value::Nil));

        Ok(())
    }

    #[test]
    fn test_full_round_robin() -> Result<()> {
        let port: u16 = 6479;
        let _guards = vec![start_redis_server_with_module("redical", port)
            .with_context(|| "failed to start redis server")?];
        let mut connection =
            get_redis_connection(port).with_context(|| "failed to connect to redis server")?;

        test_set_calendar(&mut connection)?;

        test_set_get_events(&mut connection)?;

        test_del_event(&mut connection)?;

        /*
        let res: Result<Vec<i32>, RedisError> = redis::cmd("set").arg(&["key"]).query(&mut connection);

        if res.is_ok() {
            return Err(anyhow::Error::msg("Should return an error"));
        }
        */

        Ok(())
    }
}
