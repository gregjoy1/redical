// NOTE: This is distinct enough from other date-string properties to not use the
//       build_date_string_property! macro (for now until it can be broken up more).
//
//       Differences include:
//       * Range param
//       * serialize_to_split_ical shoudl always output UTC
//
// TODO: Cater to RANGE param:
//       - https://icalendar.org/iCalendar-RFC-5545/3-2-13-recurrence-identifier-range.html
//       - https://icalendar.org/iCalendar-RFC-5545/3-8-4-4-recurrence-id.html

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use chrono::{DateTime, TimeZone};
use chrono_tz::Tz;

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
    quote_string_if_needed, serialize_timestamp_to_ical_date, serialize_timestamp_to_ical_datetime,
    SerializableICalProperty, SerializationPreferences, SerializedValue,
};

use crate::core::ical::properties::DTStartProperty;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RecurrenceIDProperty {
    pub timezone: Option<Tz>,
    pub value_type: Option<String>,
    pub utc_timestamp: i64,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

impl Eq for RecurrenceIDProperty {}

implement_property_ord_partial_ord_and_hash_traits!(RecurrenceIDProperty);

// Copy the contents of the DTStartProperty into RecurrenceIDProperty as it serves
// essentially the same purpose.
//
// TODO: Verify that reckless assertion above:
//       - https://icalendar.org/iCalendar-RFC-5545/3-8-4-4-recurrence-id.html
impl From<&DTStartProperty> for RecurrenceIDProperty {
    fn from(dtstart_property: &DTStartProperty) -> Self {
        let timezone = dtstart_property.timezone.to_owned();
        let utc_timestamp = dtstart_property.utc_timestamp.to_owned();

        let value_type = if dtstart_property.is_date_value_type() {
            Some(String::from("DATE"))
        } else {
            Some(String::from("DATE-TIME"))
        };

        RecurrenceIDProperty {
            timezone,
            value_type,
            utc_timestamp,
            x_params: None,
        }
    }
}

impl From<i64> for RecurrenceIDProperty {
    fn from(utc_timestamp: i64) -> Self {
        RecurrenceIDProperty {
            timezone: None,
            value_type: None,
            utc_timestamp,
            x_params: None,
        }
    }
}

impl SerializableICalProperty for RecurrenceIDProperty {
    fn serialize_to_split_ical(
        &self,
        _preferences: Option<&SerializationPreferences>,
    ) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

        if let Some(value_type) = &self.value_type {
            param_key_value_pairs.push((String::from("VALUE"), String::from(value_type)));
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

        let value = SerializedValue::Single(self.serialize_datestring_value(&Tz::UTC));

        (String::from(Self::NAME), params, value)
    }
}

impl RecurrenceIDProperty {
    const NAME: &'static str = "RECURRENCE-ID";

    pub fn is_date_value_type(&self) -> bool {
        self.value_type
            .as_ref()
            .is_some_and(|value_type| value_type == &String::from("DATE"))
    }

    fn serialize_datestring_value(&self, timezone: &Tz) -> String {
        if self.is_date_value_type() {
            serialize_timestamp_to_ical_date(&self.utc_timestamp, timezone)
        } else {
            serialize_timestamp_to_ical_datetime(&self.utc_timestamp, timezone)
        }
    }

    pub fn parse_ical(input: &str) -> ParserResult<&str, RecurrenceIDProperty> {
        preceded(
            tag(Self::NAME),
            cut(context(
                Self::NAME,
                tuple((
                    build_property_params_parser!(
                        "RECURRENCE-ID",
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
                let mut value_type: Option<String> = None;
                let mut timezone: Option<Tz> = None;
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        match key {
                            "VALUE" => {
                                value_type = Some(String::from(value.expect_single()));
                            }

                            "TZID" => {
                                let _ = timezone.insert(value.expect_timezone());
                            }

                            _ => {
                                let parsed_x_param_value =
                                    value.expect_list().into_iter().map(String::from).collect();

                                x_params
                                    .get_or_insert(HashMap::new())
                                    .insert(String::from(key), parsed_x_param_value);
                            }
                        }
                    }
                }

                let parsed_date_string = parsed_value.expect_date_string();

                // TODO: Clean up use of rrule::Tz over chrono_tz::Tz
                // NOTE: rrule::Tz is just an enum wrapper over chrono_tz::Tz except it
                //       does not have any implemented Serde serialization.
                let parsed_timezone = timezone.and_then(|timezone| Some(rrule::Tz::Tz(timezone)));

                let utc_timestamp = match parsed_date_string.to_date(parsed_timezone, Self::NAME) {
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

                if value_type.as_ref().is_some_and(|value_type| {
                    value_type == &String::from("DATE-TIME") && parsed_date_string.time.is_none()
                }) {
                    return Err(nom::Err::Error(VerboseError {
                        errors: vec![(
                            input,
                            VerboseErrorKind::Context(
                                "expected parsed DATE-TIME value, received DATE",
                            ),
                        )],
                    }));
                }

                if value_type.as_ref().is_some_and(|value_type| {
                    value_type == &String::from("DATE") && parsed_date_string.time.is_some()
                }) {
                    return Err(nom::Err::Error(VerboseError {
                        errors: vec![(
                            input,
                            VerboseErrorKind::Context(
                                "expected parsed DATE value, received DATE-TIME",
                            ),
                        )],
                    }));
                }

                let parsed_property = RecurrenceIDProperty {
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
    use crate::core::ical::parser::error::convert_error;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_with_invalid_date_value_type() {
        let input = "RECURRENCE-ID;VALUE=DATE:20201231T183000Z";
        let parsed_property = RecurrenceIDProperty::parse_ical(input);

        assert!(parsed_property.is_err());

        let nom::Err::Error(error) = parsed_property.unwrap_err() else {
            panic!("Expected parse error");
        };

        assert_eq!(
            convert_error(input, error),
            String::from("[0]: expected parsed DATE value, received DATE-TIME at 'RECURRENCE-ID;VALUE=DATE:20201231T183000Z' "),
        );
    }

    #[test]
    fn test_parse_ical_with_valid_date_value_type() {
        let input = "RECURRENCE-ID;VALUE=DATE:20201231";
        let parsed_property = RecurrenceIDProperty::parse_ical(input);

        assert_eq!(
            parsed_property,
            Ok((
                "",
                RecurrenceIDProperty {
                    value_type: Some(String::from("DATE")),
                    timezone: None,
                    utc_timestamp: 1609372800,
                    x_params: None,
                },
            ))
        );

        assert_eq!(parsed_property.unwrap().1.serialize_to_ical(None), input);
    }

    #[test]
    fn test_parse_ical_with_invalid_date_time_value_type() {
        let input = "RECURRENCE-ID;VALUE=DATE-TIME:20201231";
        let parsed_property = RecurrenceIDProperty::parse_ical(input);

        assert!(parsed_property.is_err());

        let nom::Err::Error(error) = parsed_property.unwrap_err() else {
            panic!("Expected parse error");
        };

        assert_eq!(
            convert_error(input, error),
            String::from("[0]: expected parsed DATE-TIME value, received DATE at 'RECURRENCE-ID;VALUE=DATE-TIME:20201231' "),
        );
    }

    #[test]
    fn test_parse_ical_with_valid_date_time_value_type() {
        let input = "RECURRENCE-ID;VALUE=DATE-TIME:20201231T183000Z";
        let parsed_property = RecurrenceIDProperty::parse_ical(input);

        assert_eq!(
            parsed_property,
            Ok((
                "",
                RecurrenceIDProperty {
                    value_type: Some(String::from("DATE-TIME")),
                    timezone: None,
                    utc_timestamp: 1609439400,
                    x_params: None,
                },
            ))
        );

        assert_eq!(parsed_property.unwrap().1.serialize_to_ical(None), input);
    }

    #[test]
    fn test_parse_ical_with_invalid_date_string() {
        let input = "RECURRENCE-ID:20201231ZZZZ";
        let parsed_property = RecurrenceIDProperty::parse_ical(input);

        assert_eq!(
            parsed_property,
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    (
                        "20201231ZZZZ",
                        VerboseErrorKind::Context("invalid parsed datetime value",),
                    ),
                    (":20201231ZZZZ", VerboseErrorKind::Context("RECURRENCE-ID",),),
                ],
            },),)
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            RecurrenceIDProperty::parse_ical("RECURRENCE-ID:20201231T183000Z"),
            Ok((
                "",
                RecurrenceIDProperty {
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
            RecurrenceIDProperty::parse_ical(
                r#"RECURRENCE-ID;TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000"#
            ),
            Ok((
                "",
                RecurrenceIDProperty {
                    value_type: Some(String::from("DATE-TIME")),
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
            RecurrenceIDProperty::parse_ical(
                r#"RECURRENCE-ID;TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000 SUMMARY:Summary text."#
            ),
            Ok((
                " SUMMARY:Summary text.",
                RecurrenceIDProperty {
                    value_type: Some(String::from("DATE-TIME")),
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
        let parsed_property = RecurrenceIDProperty::parse_ical(
            r#"RECURRENCE-ID;TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000"#
        ).unwrap().1;

        assert_eq!(
            parsed_property,
            RecurrenceIDProperty {
                value_type: Some(String::from("DATE-TIME")),
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

        let serialized_ical = parsed_property.serialize_to_ical(None);

        assert_eq!(
            RecurrenceIDProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            RecurrenceIDProperty {
                value_type: Some(String::from("DATE-TIME")),
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

        // TODO: Test with SerializationPreferences timezone...
        assert_eq!(
            serialized_ical,
            String::from(
                r#"RECURRENCE-ID;VALUE=DATE-TIME;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:20201231T183000Z"#
            ),
        );
    }

    #[test]
    fn test_serialize_to_ical_with_no_timezone() {
        let parsed_property = RecurrenceIDProperty::parse_ical(
            r#"RECURRENCE-ID;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000Z"#
        ).unwrap().1;

        assert_eq!(
            parsed_property,
            RecurrenceIDProperty {
                value_type: Some(String::from("DATE-TIME")),
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

        let serialized_ical = parsed_property.serialize_to_ical(None);

        assert_eq!(
            RecurrenceIDProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_property
        );

        // TODO: Test with SerializationPreferences timezone...
        assert_eq!(
            serialized_ical,
            r#"RECURRENCE-ID;VALUE=DATE-TIME;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:20201231T183000Z"#.to_string()
        );
    }
}
