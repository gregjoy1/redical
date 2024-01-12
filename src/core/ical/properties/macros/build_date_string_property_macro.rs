#[macro_export]
macro_rules! build_date_string_property {
    ($property_name:expr, $property_struct:ident) => {
        use std::collections::HashMap;

        use rrule::Tz;

        use nom::{
            branch::alt,
            bytes::complete::tag,
            character::complete::char,
            combinator::{cut, map, opt},
            error::{context, VerboseError, VerboseErrorKind},
            multi::separated_list1,
            sequence::{preceded, separated_pair, tuple},
        };

        use crate::core::ical::parser::common;
        use crate::core::ical::parser::common::ParserResult;
        use crate::core::ical::parser::macros::*;
        use crate::core::ical::serializer::{
            quote_string_if_needed, serialize_timestamp_to_ical_datetime, SerializableICalProperty,
            SerializedValue,
        };

        #[derive(Debug, Eq, PartialEq, Clone)]
        pub enum ValueType {
            DateTime,
            Date,
        }

        impl ToString for ValueType {
            fn to_string(&self) -> String {
                match self {
                    ValueType::DateTime => String::from("DATE-TIME"),
                    ValueType::Date => String::from("DATE"),
                }
            }
        }

        #[derive(Debug, PartialEq, Clone)]
        pub struct $property_struct {
            pub timezone: Option<Tz>,
            pub value_type: Option<ValueType>,
            pub utc_timestamp: i64,
            pub x_params: Option<HashMap<String, Vec<String>>>,
        }

        impl Eq for $property_struct {}

        implement_property_ord_partial_ord_and_hash_traits!($property_struct);

        impl SerializableICalProperty for $property_struct {
            fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
                let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();
                let mut property_timezone = &Tz::UTC;

                if let Some(value_type) = &self.value_type {
                    param_key_value_pairs.push((String::from("VALUE"), value_type.to_string()));
                }

                if let Some(timezone) = &self.timezone {
                    param_key_value_pairs.push((String::from("TZID"), String::from(timezone.name())));
                    property_timezone = timezone;
                }

                if let Some(x_params) = &self.x_params {
                    for (key, values) in x_params {
                        let param_value = values
                            .iter()
                            .map(|value| quote_string_if_needed(value, common::param_value))
                            .collect::<Vec<String>>()
                            .join(",");

                        param_key_value_pairs.push((key.clone(), param_value));
                    }
                }

                param_key_value_pairs.sort();

                let params = if param_key_value_pairs.is_empty() {
                    None
                } else {
                    Some(param_key_value_pairs)
                };

                let value = SerializedValue::Single(serialize_timestamp_to_ical_datetime(
                    &self.utc_timestamp,
                    property_timezone,
                ));

                (String::from(Self::NAME), params, value)
            }
        }

        impl $property_struct {
            const NAME: &'static str = $property_name;

            pub fn parse_ical(input: &str) -> ParserResult<&str, $property_struct> {
                preceded(
                    tag(Self::NAME),
                    cut(context(
                        Self::NAME,
                        tuple((
                            build_property_params_parser!(
                                $property_name,
                                (
                                    "VALUE",
                                    common::ParsedValue::parse_single(alt((tag("DATE-TIME"), tag("DATE"))))
                                ),
                                ("TZID", common::ParsedValue::parse_timezone)
                            ),
                            common::colon_delimeter,
                            common::ParsedValue::parse_date_string,
                        )),
                    )),
                )(input)
                .and_then(
                    |(remaining, (parsed_params, _colon_delimeter, parsed_value)): (
                        &str,
                        (
                            Option<HashMap<&str, common::ParsedValue>>,
                            &str,
                            common::ParsedValue,
                        ),
                    )| {
                        let mut value_type: Option<ValueType> = None;
                        let mut timezone: Option<Tz> = None;
                        let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                        if let Some(parsed_params) = parsed_params.clone() {
                            for (key, value) in parsed_params {
                                match key {
                                    "VALUE" => {
                                        value_type = match value.expect_single() {
                                            "DATE-TIME" => Some(ValueType::DateTime),
                                            "DATE" => Some(ValueType::Date),
                                            _ => None,
                                        };
                                    }

                                    "TZID" => {
                                        let _ = timezone.insert(value.expect_timezone());
                                    }

                                    _ => {
                                        let parsed_x_param_value = value
                                            .expect_list()
                                            .into_iter()
                                            .map(String::from)
                                            .collect();

                                        x_params
                                            .get_or_insert(HashMap::new())
                                            .insert(String::from(key), parsed_x_param_value);
                                    }
                                }
                            }
                        }

                        let parsed_date_string = parsed_value.expect_date_string();

                        let utc_timestamp = match parsed_date_string.to_date(timezone, Self::NAME) {
                            Ok(datetime) => datetime.timestamp(),
                            Err(_error) => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context("parsed datetime value invalid"),
                                    )],
                                }));
                            }
                        };

                        match value_type {
                            Some(ValueType::DateTime) if parsed_date_string.time.is_none() => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context(
                                            "expected parsed DATE-TIME value, received DATE",
                                        ),
                                    )],
                                }));
                            }

                            Some(ValueType::Date) if parsed_date_string.time.is_some() => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context(
                                            "expected parsed DATE value, received DATE-TIME",
                                        ),
                                    )],
                                }));
                            }

                            _ => {}
                        };

                        let parsed_property = $property_struct {
                            value_type,
                            timezone,
                            utc_timestamp,
                            x_params,
                        };

                        Ok((remaining, parsed_property))
                    },
                )
            }
        }

        #[cfg(test)]
        mod test {

            use super::*;
            use pretty_assertions_sorted::assert_eq;

            #[test]
            fn test_parse_ical_with_invalid_date_value_type() {
                let input = concat!($property_name, ";VALUE=DATE:20201231T183000Z");
                let parsed_property = $property_struct::parse_ical(input);

                assert!(parsed_property.is_err());

                let nom::Err::Error(error) = parsed_property.unwrap_err() else {
                    panic!("Expected parse error");
                };

                assert_eq!(
                    nom::error::convert_error(input, error),
                    format!("0: at line 1, in expected parsed DATE value, received DATE-TIME:\n{property_name};VALUE=DATE:20201231T183000Z\n^\n\n", property_name = $property_name),
                );
            }

            #[test]
            fn test_parse_ical_with_invalid_date_time_value_type() {
                let input = concat!($property_name, ";VALUE=DATE-TIME:20201231");
                let parsed_property = $property_struct::parse_ical(input);

                assert!(parsed_property.is_err());

                let nom::Err::Error(error) = parsed_property.unwrap_err() else {
                    panic!("Expected parse error");
                };

                assert_eq!(
                    nom::error::convert_error(input, error),
                    format!("0: at line 1, in expected parsed DATE-TIME value, received DATE:\n{property_name};VALUE=DATE-TIME:20201231\n^\n\n", property_name = $property_name),
                );
            }

            #[test]
            fn test_parse_ical_with_invalid_date_string() {
                let input = concat!($property_name, ":20201231ZZZZ");
                let parsed_property = $property_struct::parse_ical(input);

                assert_eq!(
                    parsed_property,
                    Err(
                        nom::Err::Failure(
                            VerboseError {
                                errors: vec![
                                    (
                                        "20201231ZZZZ",
                                        VerboseErrorKind::Context(
                                            "invalid parsed datetime value",
                                        ),
                                    ),
                                    (
                                        ":20201231ZZZZ",
                                        VerboseErrorKind::Context(
                                            $property_name,
                                        ),
                                    ),
                                ],
                            },
                        ),
                    )
                );
            }

            #[test]
            fn test_parse_ical_minimal() {
                assert_eq!(
                    $property_struct::parse_ical(concat!($property_name, ":20201231T183000Z")),
                    Ok((
                        "",
                        $property_struct {
                            value_type: None,
                            timezone: None,
                            utc_timestamp: 1609439400,
                            x_params: None,
                        }
                    ))
                );
            }

            #[test]
            fn test_parse_ical_full() {
                assert_eq!(
                    $property_struct::parse_ical(
                        concat!($property_name, r#";TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000"#)
                    ),
                    Ok((
                        "",
                        $property_struct {
                            value_type: Some(ValueType::DateTime),
                            timezone: Some(Tz::Europe__London),
                            utc_timestamp: 1609439400,
                            x_params: Some(HashMap::from([
                                (
                                    String::from("X-TEST-KEY-TWO"),
                                    vec![String::from("KEY -🎄- TWO")]
                                ),
                                (
                                    String::from("X-TEST-KEY-ONE"),
                                    vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                                ),
                            ])),
                        }
                    ))
                );
            }

            #[test]
            fn test_parse_ical_full_with_lookahead() {
                assert_eq!(
                    $property_struct::parse_ical(
                        concat!($property_name, r#";TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000 SUMMARY:Summary text."#)
                    ),
                    Ok((
                        " SUMMARY:Summary text.",
                        $property_struct {
                            value_type: Some(ValueType::DateTime),
                            timezone: Some(Tz::Europe__London),
                            utc_timestamp: 1609439400,
                            x_params: Some(HashMap::from([
                                (
                                    String::from("X-TEST-KEY-TWO"),
                                    vec![String::from("KEY -🎄- TWO")]
                                ),
                                (
                                    String::from("X-TEST-KEY-ONE"),
                                    vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                                ),
                            ])),
                        }
                    ))
                );
            }

            #[test]
            fn test_serialize_to_ical_with_timezone() {
                let parsed_property = $property_struct::parse_ical(
                    concat!($property_name, r#";TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000"#)
                ).unwrap().1;

                assert_eq!(
                    parsed_property,
                    $property_struct {
                        value_type: Some(ValueType::DateTime),
                        timezone: Some(Tz::Europe__London),
                        utc_timestamp: 1609439400,
                        x_params: Some(HashMap::from([
                            (
                                String::from("X-TEST-KEY-TWO"),
                                vec![String::from("KEY -🎄- TWO")]
                            ),
                            (
                                String::from("X-TEST-KEY-ONE"),
                                vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                            ),
                        ])),
                    },
                );

                let serialized_ical = parsed_property.serialize_to_ical();

                assert_eq!(
                    $property_struct::parse_ical(serialized_ical.as_str())
                        .unwrap()
                        .1,
                    parsed_property
                );

                assert_eq!(
                    serialized_ical,
                    String::from(
                        concat!($property_name, r#";TZID=Europe/London;VALUE=DATE-TIME;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:20201231T183000"#)
                    ),
                );
            }

            #[test]
            fn test_serialize_to_ical_with_no_timezone() {
                let parsed_property = $property_struct::parse_ical(
                    concat!($property_name, r#";X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000Z"#)
                ).unwrap().1;

                assert_eq!(
                    parsed_property,
                    $property_struct {
                        value_type: Some(ValueType::DateTime),
                        timezone: None,
                        utc_timestamp: 1609439400,
                        x_params: Some(HashMap::from([
                            (
                                String::from("X-TEST-KEY-TWO"),
                                vec![String::from("KEY -🎄- TWO")]
                            ),
                            (
                                String::from("X-TEST-KEY-ONE"),
                                vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                            ),
                        ])),
                    },
                );

                let serialized_ical = parsed_property.serialize_to_ical();

                assert_eq!(
                    $property_struct::parse_ical(serialized_ical.as_str())
                        .unwrap()
                        .1,
                    parsed_property
                );

                assert_eq!(
                    serialized_ical,
                    String::from(
                        concat!($property_name, r#";VALUE=DATE-TIME;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:20201231T183000Z"#)
                    ),
                );
            }
        }
    }
}

pub use build_date_string_property;
