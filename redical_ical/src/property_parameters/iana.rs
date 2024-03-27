use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::multi::{separated_list0, separated_list1};
use nom::combinator::{map, cut, recognize};
use nom::bytes::complete::tag;

use std::collections::HashSet;

use crate::grammar::{iana_token, param_value, comma, semicolon};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// x-param     = x-name "=" param-value *("," param-value)
//      ; A non-standard, experimental parameter.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct IanaParam(String, String);

impl ICalendarEntity for IanaParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "IANA-PARAM",
            map(
                pair(
                    iana_token,
                    preceded(tag("="), cut(recognize(separated_list1(comma, param_value)))),
                ),
                |(key, value)| Self(key.to_string(), value.to_string())
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("{}={}", self.0, self.1)
    }
}

impl_icalendar_entity_traits!(IanaParam);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct IanaParams(HashSet<IanaParam>);

impl ICalendarEntity for IanaParams {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(
            separated_list0(semicolon, IanaParam::parse_ical),
            |x_params| Self(HashSet::from_iter(x_params.into_iter()))
        )(input)
    }

    fn render_ical(&self) -> String {
        let mut x_params =
            self.0
                .iter()
                .map(|value| value.render_ical())
                .collect::<Vec<String>>();

        x_params.sort();

        x_params.join(";")
    }
}

impl_icalendar_entity_traits!(IanaParams);

impl Default for IanaParams {
    fn default() -> Self {
        IanaParams(HashSet::new())
    }
}

impl IanaParams {
    pub fn insert(&mut self, value: IanaParam) -> bool {
        self.0.insert(value)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn is_some(&self) -> bool {
        self.0.is_empty() == false
    }
}

impl From<Vec<(&str, &str)>> for IanaParams {
    fn from(values: Vec<(&str, &str)>) -> Self {
        let mut params = IanaParams::default();

        for (key, value) in values {
            params.0.insert(
                IanaParam(
                    String::from(key),
                    String::from(value),
                )
            );
        }

        params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            IanaParam::parse_ical("TEST-NAME=TESTING TEXT".into()),
            (
                "",
                IanaParam(
                    String::from("TEST-NAME"),
                    String::from("TESTING TEXT"),
                ),
            ),
        );

        assert_parser_output!(
            IanaParam::parse_ical("TEST-NAME=TESTING TEXT ONE,TESTING TEXT TWO".into()),
            (
                "",
                IanaParam(
                    String::from("TEST-NAME"),
                    String::from("TESTING TEXT ONE,TESTING TEXT TWO"),
                ),
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            IanaParam(
                String::from("TEST-NAME"),
                String::from("TESTING TEXT ONE,TESTING TEXT TWO"),
            ).render_ical(),
            String::from("TEST-NAME=TESTING TEXT ONE,TESTING TEXT TWO"),
        );
    }
}
