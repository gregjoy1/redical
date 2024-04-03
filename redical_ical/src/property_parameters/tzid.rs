use chrono_tz::Tz;

use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{opt, map_res, map, cut, recognize};
use nom::bytes::complete::{tag, take_while1};

use crate::grammar::{is_safe_char, is_wsp_char, solidus};

use crate::{ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzid(pub String);

impl ICalendarEntity for Tzid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZID",
            map_res(
                recognize(
                    pair(
                        opt(solidus),
                        // Small hack that allows paramtext chars except whitespace.
                        take_while1(|input: char| {
                            is_safe_char(input) && !is_wsp_char(input)
                        }),
                    )
                ),
                |value: ParserInput| {
                    let tzid = Self(value.to_string());

                    if let Err(error) = tzid.validate() {
                        Err(error)
                    } else {
                        Ok(tzid)
                    }
                }
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        self.0.to_owned()
    }

    fn validate(&self) -> Result<(), String> {
        if self.0.parse::<Tz>().is_err() {
            return Err(String::from("Timezone is invalid"))
        }

        Ok(())
    }
}

impl_icalendar_entity_traits!(Tzid);

impl TryFrom<Tzid> for Tz {
    type Error = String;

    fn try_from(tzid: Tzid) -> Result<Self, Self::Error> {
        tzid.0.parse::<Tz>().map_err(|_error| String::from("Timezone is invalid"))
    }
}

// Time Zone Identifier
//
// Parameter Name:  TZID
//
// Purpose:  To specify the identifier for the time zone definition for
//    a time component in the property value.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     tzidparam  = "TZID" "=" [tzidprefix] paramtext
//
//     tzidprefix = "/"
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TzidParam(pub Tzid);

impl ICalendarEntity for TzidParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZIDPARAM",
            map(
                pair(
                    tag("TZID"),
                    preceded(
                        tag("="),
                        cut(Tzid::parse_ical),
                    )
                ),
                |(_key, tzid)| Self(tzid)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZID={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(TzidParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            TzidParam::parse_ical("TZID=America/New_York TESTING".into()),
            (
                " TESTING",
                TzidParam(Tzid(String::from("America/New_York"))),
            )
        );

        assert_parser_output!(
            TzidParam::parse_ical("TZID=Etc/GMT+12 TESTING".into()),
            (
                " TESTING",
                TzidParam(Tzid(String::from("Etc/GMT+12"))),
            )
        );

        assert_parser_output!(
            TzidParam::parse_ical("TZID=UTC TESTING".into()),
            (
                " TESTING",
                TzidParam(Tzid(String::from("UTC"))),
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            TzidParam(Tzid(String::from("America/New_York"))).render_ical(),
            String::from("TZID=America/New_York"),
        );
    }
}
