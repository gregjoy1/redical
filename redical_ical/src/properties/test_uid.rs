use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::{fold_many0, separated_list1};
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, comma, x_name, iana_token, param_value};

use crate::{ICalendarEntity, ParserInput, ParserResult, ParserError};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct UidParams {
    pub x: HashMap<String, String>,
    pub iana: HashMap<String, String>,
}

#[macro_export]
macro_rules! define_property_params_parser {
    ($struct_name:ident, $(($parser_expr:expr, $handler:expr, $renderer:expr $(,)*), $(,)*)+ $(,)*) => {
        impl ICalendarEntity for $struct_name {
            fn parse_ical(input: ParserInput) -> ParserResult<Self> {
                let mut remaining = input;
                let mut params = Self::default();

                loop {
                    let Ok((new_remaining, _)) = semicolon(remaining) else {
                        break;
                    };

                    remaining = new_remaining;

                    $(
                        match $parser_expr(remaining) {
                            Ok((new_remaining, (key, value))) => {
                                remaining = new_remaining;

                                let handler = $handler;

                                handler(&mut params, key, value);

                                continue;
                            },

                            Err(nom::Err::Failure(error)) => {
                                return Err(nom::Err::Failure(error));
                            },

                            _ => {},
                        }
                    )+

                    break;
                }

                Ok((remaining, params))
            }

            fn render_ical(&self) -> String {
                let mut output = String::new();

                $(
                    output.push_str($renderer(self).as_str());
                )+

                output
            }
        }
    }
}

define_property_params_parser!(
    UidParams,
    (
        pair(x_name, cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
        |params: &mut UidParams, key: ParserInput, value: ParserInput| params.x.insert(key.to_string(), value.to_string()),
        |params: &UidParams| params.x.iter().map(|(key, value)| format!(";{key}={value}")).sorted().collect::<Vec<String>>().join(""),
    ),
    (
        pair(iana_token, cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
        |params: &mut UidParams, key: ParserInput, value: ParserInput| params.iana.insert(key.to_string(), value.to_string()),
        |params: &UidParams| params.iana.iter().map(|(key, value)| format!(";{key}={value}")).sorted().collect::<Vec<String>>().join(""),
    ),
);

/*
impl ICalendarEntity for UidParams {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        let mut remaining = input;
        let mut params = Self::default();

        loop {
            let Ok((new_remaining, _)) = semicolon(remaining) else {
                break;
            };

            remaining = new_remaining;

            match pair(x_name, cut(preceded(tag("="), recognize(separated_list1(comma, param_value)))))(remaining) {
                Ok((new_remaining, (key, value))) => {
                    remaining = new_remaining;

                    params.x.insert(key.to_string(), value.to_string());

                    continue;
                },

                Err(nom::Err::Failure(error)) => {
                    return Err(nom::Err::Failure(error));
                },

                _ => {},
            }

            match pair(iana_token, cut(preceded(tag("="), recognize(separated_list1(comma, param_value)))))(remaining) {
                Ok((new_remaining, (key, value))) => {
                    remaining = new_remaining;

                    params.iana.insert(key.to_string(), value.to_string());

                    continue;
                },

                Err(nom::Err::Failure(error)) => {
                    return Err(nom::Err::Failure(error));
                },

                _ => {},
            }

            break;
        }

        Ok((remaining, params))
    }

    fn render_ical(&self) -> String {
        String::new()
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn render_ical() {
        assert_eq!(
            UidParams {
                iana: HashMap::from([(String::from("TEST"), String::from("VALUE"))]),
                x: HashMap::from([(String::from("X-TEST"), String::from("X_VALUE"))]),
            }.render_ical(),
            String::from(";X-TEST=X_VALUE;TEST=VALUE"),
        );

        assert_eq!(
            UidParams {
                iana: HashMap::from([(String::from("TEST"), String::from("VALUE"))]),
                x: HashMap::from([(String::from("X-TEST-ONE"), String::from("X_VALUE_ONE_UPDATED")), (String::from("X-TEST-TWO"), String::from("X_VALUE_TWO"))]),
            }.render_ical(),
            String::from(";X-TEST-ONE=X_VALUE_ONE_UPDATED;X-TEST-TWO=X_VALUE_TWO;TEST=VALUE"),
        );
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            UidParams::parse_ical("".into()),
            (
                "",
                UidParams {
                    iana: HashMap::default(),
                    x: HashMap::default(),
                }
            ),
        );

        assert_parser_output!(
            UidParams::parse_ical(";X-TEST=X_VALUE;TEST=VALUE".into()),
            (
                "",
                UidParams {
                    iana: HashMap::from([(String::from("TEST"), String::from("VALUE"))]),
                    x: HashMap::from([(String::from("X-TEST"), String::from("X_VALUE"))]),
                },
            ),
        );

        assert_parser_output!(
            UidParams::parse_ical(";X-TEST-ONE=X_VALUE_ONE;TEST=VALUE;X-TEST-TWO=X_VALUE_TWO;X-TEST-ONE=X_VALUE_ONE_UPDATED".into()),
            (
                "",
                UidParams {
                    iana: HashMap::from([(String::from("TEST"), String::from("VALUE"))]),
                    x: HashMap::from([(String::from("X-TEST-ONE"), String::from("X_VALUE_ONE_UPDATED")), (String::from("X-TEST-TWO"), String::from("X_VALUE_TWO"))]),
                },
            ),
        );
    }
}
