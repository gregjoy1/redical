#[macro_export]
macro_rules! assert_matching_ical_properties {
    ($redis_result:expr, $expected_result:expr $(,)*) => {
        let mut actual_result = $redis_result.to_owned();
        let mut expected_result = $expected_result.to_owned();

        actual_result.sort();
        expected_result.sort();

        assert_eq_sorted!(actual_result, expected_result);

        assert_eq!(actual_result.len(), expected_result.len());
    }
}

#[macro_export]
macro_rules! assert_matching_ical_components_sorted {
    ($redis_result:expr, $expected_result:expr $(,)*) => {
        let mut actual_result: Vec<Vec<String>> = $redis_result;
        let mut expected_result: Vec<Vec<String>> = $expected_result;

        // Sort both components and their properties as we dont care about the order of
        // either of these, only their overall presence.
        actual_result.iter_mut().for_each(|properties| properties.sort());
        expected_result.iter_mut().for_each(|properties| properties.sort());

        actual_result.sort();
        expected_result.sort();

        assert_eq_sorted!(actual_result, expected_result);
    }
}

#[macro_export]
macro_rules! assert_matching_ical_components {
    ($redis_result:expr, $expected_result:expr $(,)*) => {
        let mut actual_result: Vec<Vec<String>> = $redis_result;
        let mut expected_result: Vec<Vec<String>> = $expected_result;

        // Only sort component properties as we dont care about the order of these, only
        // their presence and order of the collection of components themselves (e.g. testing
        // chronological lists).
        actual_result.iter_mut().for_each(|properties| properties.sort());
        expected_result.iter_mut().for_each(|properties| properties.sort());

        assert_eq!(actual_result, expected_result);
    }
}

#[macro_export]
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

#[macro_export]
macro_rules! assert_calendar_nil {
    ($connection:expr, $calendar_uid:expr $(,)*) => {
        assert_eq!(redis::cmd("rdcl.cal_get").arg($calendar_uid).query($connection), RedisResult::Ok(Value::Nil));
    }
}

#[macro_export]
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

#[macro_export]
macro_rules! assert_event_present {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr $(,)*) => {
        let event_get_result: Vec<String> = redis::cmd("rdcl.evt_get")
            .arg($calendar_uid)
            .arg($event_uid)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to get event with UID: '{}' via rdcl.evt_get", $event_uid,
                )
            })?;

        assert!(event_get_result.len() > 0);
    };

    ($connection:expr, $calendar_uid:expr, $event_uid:expr, [$($ical_property:expr),+ $(,)*] $(,)*) => {
        let event_get_result: Vec<String> = redis::cmd("rdcl.evt_get")
            .arg($calendar_uid)
            .arg($event_uid)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to get event with UID: '{}' via rdcl.evt_get", $event_uid,
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
    };
}

#[macro_export]
macro_rules! assert_event_nil {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr $(,)*) => {
        assert_eq!(redis::cmd("rdcl.evt_get").arg($calendar_uid).arg($event_uid).query($connection), RedisResult::Ok(Value::Nil));
    }
}

#[macro_export]
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
                    "failed to set event with UID: '{}' via rdcl.evt_set", $event_uid,
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

#[macro_export]
macro_rules! del_and_assert_event_deletion {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $expected_result:expr $(,)*) => {
        let event_del_result =
            redis::cmd("rdcl.evt_del")
            .arg($calendar_uid)
            .arg($event_uid)
            .query($connection);

        assert_eq!(
            event_del_result,
            RedisResult::Ok(
                Value::Int($expected_result)
            )
        );

        assert_event_nil!(
            $connection,
            $calendar_uid,
            $event_uid,
        );
    }
}

#[macro_export]
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

        assert_matching_ical_components_sorted!(event_list_result, expected_event_list_result);
    };
}

#[macro_export]
macro_rules! assert_event_override_present {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $override_date_string:expr $(,)*) => {
        let event_override_get_result: Vec<String> = redis::cmd("rdcl.evo_get")
            .arg($calendar_uid)
            .arg($event_uid)
            .arg($override_date_string)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to get override for event with UID: '{}' at '{}' via rdcl.evt_get", $event_uid, $override_date_string,
                )
            })?;

        assert!(event_override_get_result.len() > 0);
    };

    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $override_date_string:expr, [$($ical_property:expr),+ $(,)*] $(,)*) => {
        let event_override_get_result: Vec<String> = redis::cmd("rdcl.evo_get")
            .arg($calendar_uid)
            .arg($event_uid)
            .arg($override_date_string)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to get override for event with UID: '{}' at '{}' via rdcl.evt_get", $event_uid, $override_date_string,
                )
            })?;

        assert_matching_ical_properties!(
            event_override_get_result,
            vec![
                format!("DTSTART:{}", $override_date_string),
                $(
                    String::from($ical_property),
                )+
            ],
        );
    };
}

#[macro_export]
macro_rules! assert_event_override_nil {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $override_date_string:expr, $(,)*) => {
        assert_eq!(redis::cmd("rdcl.evo_get").arg($calendar_uid).arg($event_uid).arg($override_date_string).query($connection), RedisResult::Ok(Value::Nil));
    }
}

#[macro_export]
macro_rules! set_and_assert_event_override {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $override_date_string:expr, [$($ical_property:expr),+ $(,)*] $(,)*) => {
        assert_event_override_nil!(
            $connection,
            $calendar_uid,
            $event_uid,
            $override_date_string,
        );

        let mut ical_properties: Vec<String> = vec![
            $(
                String::from($ical_property),
            )+
        ];

        let joined_ical_properties = ical_properties.join(" ");

        let event_override_set_result: Vec<String> = redis::cmd("rdcl.evo_set")
            .arg("TEST_CALENDAR_UID")
            .arg($event_uid)
            .arg($override_date_string)
            .arg(joined_ical_properties)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to set override for event with UID: '{}' at '{}' via rdcl.evo_set", $event_uid, $override_date_string,
                )
            })?;

        ical_properties.push(format!("DTSTART:{}", $override_date_string));

        assert_matching_ical_properties!(event_override_set_result, ical_properties);

        assert_event_override_present!(
            $connection,
            $calendar_uid, 
            $event_uid,
            $override_date_string,
            [
                $(
                    $ical_property,
                )+
            ],
        );
    }
}

#[macro_export]
macro_rules! list_and_assert_matching_event_overrides {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, [] $(,)*) => {
        let event_override_list_result: Vec<Vec<String>> = redis::cmd("rdcl.evo_list")
            .arg($calendar_uid)
            .arg($event_uid)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to list overrides for event UID: '{}' events via rdcl.evo_list", $event_uid,
                )
            })?;

        let expected_event_override_list_result: Vec<Vec<String>> = vec![];

        assert_eq!(event_override_list_result, expected_event_override_list_result);
    };

    ($connection:expr, $calendar_uid:expr, $event_uid:expr, [$([$($ical_component_property:expr),+ $(,)*]),+ $(,)*] $(,)*) => {
        let expected_event_override_list_result: Vec<Vec<String>> = vec![
            $(
                vec![
                    $(
                        String::from($ical_component_property),
                    )+
                ],
            )+
        ];

        let event_override_list_result: Vec<Vec<String>> = redis::cmd("rdcl.evo_list")
            .arg($calendar_uid)
            .arg($event_uid)
            .query($connection)
            .with_context(|| {
                format!(
                    "failed to list overrides for event UID: '{}' events via rdcl.evo_list", $event_uid,
                )
            })?;

        assert_matching_ical_components!(event_override_list_result, expected_event_override_list_result);
    };
}

#[macro_export]
macro_rules! del_and_assert_event_override_deletion {
    ($connection:expr, $calendar_uid:expr, $event_uid:expr, $override_date_string:expr, $expected_result:expr $(,)*) => {
        let event_override_del_result =
            redis::cmd("rdcl.evo_del")
            .arg($calendar_uid)
            .arg($event_uid)
            .arg($override_date_string)
            .query($connection);

        assert_eq!(
            event_override_del_result,
            RedisResult::Ok(
                Value::Int($expected_result)
            )
        );

        assert_event_override_nil!(
            $connection,
            $calendar_uid,
            $event_uid,
            $override_date_string,
        );
    }
}

#[macro_export]
macro_rules! run_all_integration_tests_sequentially {
    ($($test_function:ident),+ $(,)*) => {
        #[test]
        fn test_all_integration_tests_sequentially() -> Result<()> {
            use utils::{get_redis_connection, start_redis_server_with_module};

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
