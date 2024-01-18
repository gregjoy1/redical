use std::collections::HashMap;

use nom::{
    bytes::complete::tag,
    character::complete::{char, digit1},
    combinator::{cut, map, opt},
    error::{context, VerboseError, VerboseErrorKind},
    multi::separated_list1,
    sequence::{preceded, separated_pair, terminated, tuple},
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;
use crate::core::ical::parser::macros::*;
use crate::core::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializedValue,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct DurationProperty {
    pub weeks: Option<i64>,
    pub days: Option<i64>,
    pub hours: Option<i64>,
    pub minutes: Option<i64>,
    pub seconds: Option<i64>,

    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(DurationProperty);

impl From<i64> for DurationProperty {
    fn from(duration_in_seconds: i64) -> Self {
        let mut remaining_seconds = duration_in_seconds;

        let mut weeks = None;
        let mut days = None;
        let mut hours = None;
        let mut minutes = None;
        let mut seconds = None;

        if remaining_seconds >= Self::SECONDS_IN_WEEK {
            weeks = Some(remaining_seconds / Self::SECONDS_IN_WEEK);

            remaining_seconds = remaining_seconds % Self::SECONDS_IN_WEEK;
        }

        if remaining_seconds >= Self::SECONDS_IN_DAY {
            days = Some(remaining_seconds / Self::SECONDS_IN_DAY);

            remaining_seconds = remaining_seconds % Self::SECONDS_IN_DAY;
        }

        if remaining_seconds >= Self::SECONDS_IN_HOUR {
            hours = Some(remaining_seconds / Self::SECONDS_IN_HOUR);

            remaining_seconds = remaining_seconds % Self::SECONDS_IN_HOUR;
        }

        if remaining_seconds >= Self::SECONDS_IN_MINUTE {
            minutes = Some(remaining_seconds / Self::SECONDS_IN_MINUTE);

            remaining_seconds = remaining_seconds % Self::SECONDS_IN_MINUTE;
        }

        if remaining_seconds > 0 || duration_in_seconds == 0 {
            seconds = Some(remaining_seconds);
        }

        DurationProperty {
            weeks,
            days,
            hours,
            minutes,
            seconds,

            x_params: None,
        }
    }
}

impl SerializableICalProperty for DurationProperty {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

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

        let value =
            SerializedValue::Single(self.serialize_to_ical_value().unwrap_or(String::from("")));

        (String::from(DurationProperty::NAME), params, value)
    }
}

impl Default for DurationProperty {
    fn default() -> Self {
        DurationProperty {
            weeks: None,
            days: None,
            hours: None,
            minutes: None,
            seconds: None,

            x_params: None,
        }
    }
}

impl DurationProperty {
    const NAME: &'static str = "DURATION";

    const SECONDS_IN_MINUTE: i64 = 60;
    const SECONDS_IN_HOUR: i64 = Self::SECONDS_IN_MINUTE * 60;
    const SECONDS_IN_DAY: i64 = Self::SECONDS_IN_HOUR * 24;
    const SECONDS_IN_WEEK: i64 = Self::SECONDS_IN_DAY * 7;

    pub fn get_duration_in_seconds(&self) -> i64 {
        let mut duration_in_seconds = 0;

        if let Some(weeks) = self.weeks {
            duration_in_seconds += weeks * Self::SECONDS_IN_WEEK;
        }

        if let Some(days) = self.days {
            duration_in_seconds += days * Self::SECONDS_IN_DAY;
        }

        if let Some(hours) = self.hours {
            duration_in_seconds += hours * Self::SECONDS_IN_HOUR;
        }

        if let Some(minutes) = self.minutes {
            duration_in_seconds += minutes * Self::SECONDS_IN_MINUTE;
        }

        if let Some(seconds) = self.seconds {
            duration_in_seconds += seconds
        }

        duration_in_seconds
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn serialize_to_ical_value(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut output = String::from("P");

        if let Some(weeks) = self.weeks {
            output.push_str(&format!("{weeks}W"));
        }

        if let Some(days) = self.days {
            output.push_str(&format!("{days}D"));
        }

        if self.hours.is_some() || self.minutes.is_some() || self.seconds.is_some() {
            output.push_str("T");
        }

        if let Some(hours) = self.hours {
            output.push_str(&format!("{hours}H"));
        }

        if let Some(minutes) = self.minutes {
            output.push_str(&format!("{minutes}M"));
        }

        if let Some(seconds) = self.seconds {
            output.push_str(&format!("{seconds}S"));
        }

        Some(output)
    }

    pub fn parse_ical(input: &str) -> ParserResult<&str, DurationProperty> {
        preceded(
            tag("DURATION"),
            cut(context(
                "DURATION",
                tuple((
                    build_property_params_parser!("DURATION"),
                    common::colon_delimeter,
                    context(
                        "parsed duration",
                        preceded(
                            tag("P"),
                            tuple((
                                opt(terminated(digit1, tag("W"))),
                                opt(terminated(digit1, tag("D"))),
                                opt(tuple((
                                    tag("T"),
                                    opt(terminated(digit1, tag("H"))),
                                    opt(terminated(digit1, tag("M"))),
                                    opt(terminated(digit1, tag("S"))),
                                ))),
                            )),
                        ),
                    ),
                )),
            )),
        )(input)
        .and_then(
            |(remaining, (parsed_params, _colon_delimeter, parsed_duration_string_components))| {
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        let parsed_x_param_value =
                            value.expect_list().into_iter().map(String::from).collect();

                        x_params
                            .get_or_insert(HashMap::new())
                            .insert(String::from(key), parsed_x_param_value);
                    }
                }

                let mut duration_property = DurationProperty::default();

                duration_property.x_params = x_params;

                let (weeks, days, time_component) = parsed_duration_string_components;

                if let Some(weeks) = weeks {
                    let parsed_weeks = match str::parse::<i64>(weeks) {
                        Ok(weeks) => weeks,

                        Err(_error) => {
                            return Err(nom::Err::Error(VerboseError {
                                errors: vec![(
                                    input,
                                    VerboseErrorKind::Context(
                                        "Could not parse numeric duration weeks value",
                                    ),
                                )],
                            }));
                        }
                    };

                    duration_property.weeks = Some(parsed_weeks);
                }

                if let Some(days) = days {
                    let parsed_days = match str::parse::<i64>(days) {
                        Ok(days) => days,

                        Err(_error) => {
                            return Err(nom::Err::Error(VerboseError {
                                errors: vec![(
                                    input,
                                    VerboseErrorKind::Context(
                                        "Could not parse numeric duration days value",
                                    ),
                                )],
                            }));
                        }
                    };

                    duration_property.days = Some(parsed_days);
                }

                if let Some((_time_delim, hours, minutes, seconds)) = time_component {
                    if let Some(hours) = hours {
                        let parsed_hours = match str::parse::<i64>(hours) {
                            Ok(hours) => hours,

                            Err(_error) => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context(
                                            "Could not parse numeric duration hours value",
                                        ),
                                    )],
                                }));
                            }
                        };

                        duration_property.hours = Some(parsed_hours);
                    }

                    if let Some(minutes) = minutes {
                        let parsed_minutes = match str::parse::<i64>(minutes) {
                            Ok(minutes) => minutes,

                            Err(_error) => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context(
                                            "Could not parse numeric duration minutes value",
                                        ),
                                    )],
                                }));
                            }
                        };

                        duration_property.minutes = Some(parsed_minutes);
                    }

                    if let Some(seconds) = seconds {
                        let parsed_seconds = match str::parse::<i64>(seconds) {
                            Ok(seconds) => seconds,

                            Err(_error) => {
                                return Err(nom::Err::Error(VerboseError {
                                    errors: vec![(
                                        input,
                                        VerboseErrorKind::Context(
                                            "Could not parse numeric duration seconds value",
                                        ),
                                    )],
                                }));
                            }
                        };

                        duration_property.seconds = Some(parsed_seconds);
                    }
                }

                Ok((remaining, duration_property))
            },
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use nom::error::ErrorKind;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_from_seconds_int() {
        assert_eq!(
            DurationProperty::from(1483506),
            DurationProperty {
                weeks: Some(2),
                days: Some(3),
                hours: Some(4),
                minutes: Some(5),
                seconds: Some(6),
                x_params: None,
            }
        );

        assert_eq!(
            DurationProperty::from(25),
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
                x_params: None,
            }
        );

        assert_eq!(
            DurationProperty::from(0),
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(0),
                x_params: None,
            }
        );

        assert_eq!(
            DurationProperty::from(-100),
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
                x_params: None,
            }
        );
    }

    #[test]
    fn test_get_duration_in_seconds() {
        assert_eq!(DurationProperty::default().get_duration_in_seconds(), 0);

        assert_eq!(
            DurationProperty {
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
                x_params: None,
            }
            .get_duration_in_seconds(),
            20 + ((60 * 60) * 5) + (((60 * 60) * 24) * 15),
        );

        assert_eq!(
            DurationProperty {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
                x_params: None,
            }
            .get_duration_in_seconds(),
            (((60 * 60) * 24) * 7) * 7,
        );

        assert_eq!(
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
                x_params: None,
            }
            .get_duration_in_seconds(),
            25,
        );
    }

    #[test]
    fn test_to_ical() {
        assert_eq!(DurationProperty::default().serialize_to_ical_value(), None);

        assert_eq!(
            DurationProperty {
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
                x_params: None,
            }
            .serialize_to_ical_value(),
            Some(String::from("P15DT5H0M20S")),
        );

        assert_eq!(
            DurationProperty {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
                x_params: None,
            }
            .serialize_to_ical_value(),
            Some(String::from("P7W")),
        );

        assert_eq!(
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
                x_params: None,
            }
            .serialize_to_ical_value(),
            Some(String::from("PT25S")),
        );
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            DurationProperty::parse_ical("DURATION:--INVALID20S"),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("--INVALID20S", VerboseErrorKind::Nom(ErrorKind::Tag,),),
                    (
                        "--INVALID20S",
                        VerboseErrorKind::Context("parsed duration",),
                    ),
                    (":--INVALID20S", VerboseErrorKind::Context("DURATION",),),
                ],
            },),)
        );

        assert_eq!(
            DurationProperty::parse_ical("DURATION:P15DT5H0M20S"),
            Ok((
                "",
                DurationProperty {
                    weeks: None,
                    days: Some(15),
                    hours: Some(5),
                    minutes: Some(0),
                    seconds: Some(20),
                    x_params: None,
                }
            ))
        );

        assert_eq!(
            DurationProperty::parse_ical("DURATION:P7W"),
            Ok((
                "",
                DurationProperty {
                    weeks: Some(7),
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: None,
                    x_params: None,
                }
            ))
        );

        assert_eq!(
            DurationProperty::parse_ical("DURATION:PT25S"),
            Ok((
                "",
                DurationProperty {
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: Some(25),
                    x_params: None,
                }
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            DurationProperty::parse_ical(
                r#"DURATION;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PT25S SUMMARY:Summary Text."#
            ),
            Ok((
                " SUMMARY:Summary Text.",
                DurationProperty {
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: Some(25),
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
    fn test_serialize_to_ical() {
        let parsed_duration_property = DurationProperty::parse_ical(
            r#"DURATION;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PT25S"#,
        )
        .unwrap()
        .1;

        assert_eq!(
            parsed_duration_property,
            DurationProperty {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
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
        );

        let serialized_ical = parsed_duration_property.serialize_to_ical();

        assert_eq!(
            DurationProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_duration_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"DURATION;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:PT25S"#
            ),
        );
    }
}
