use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{x_name, iana_token};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// FBTYPE = ("FREE" / "BUSY"
//          / "BUSY-UNAVAILABLE" / "BUSY-TENTATIVE"
//          / x-name
//          ; Some experimental iCalendar free/busy type.
//          / iana-token)
//          ; Some other IANA-registered iCalendar free/busy type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Fbtype {
    Free,
    Busy,
    BusyUnavailable,
    BusyTentative,
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl ICalendarEntity for Fbtype {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "FBTYPE",
            alt((
                map(tag("FREE"), |_| Fbtype::Free),
                map(tag("BUSY-UNAVAILABLE"), |_| Fbtype::BusyUnavailable),
                map(tag("BUSY-TENTATIVE"), |_| Fbtype::BusyTentative),
                map(tag("BUSY"), |_| Fbtype::Busy),
                map(x_name, |value| Fbtype::XName(value.to_string())),
                map(iana_token, |value| Fbtype::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Free => String::from("FREE"),
            Self::Busy => String::from("BUSY"),
            Self::BusyUnavailable => String::from("BUSY-UNAVAILABLE"),
            Self::BusyTentative => String::from("BUSY-TENTATIVE"),
            Self::XName(name) => name.to_owned(),
            Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(Fbtype);

// Free/Busy Time Type
//
// Parameter Name:  FBTYPE
//
// Purpose:  To specify the free or busy time type.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     fbtypeparam        = "FBTYPE" "=" ("FREE" / "BUSY"
//                        / "BUSY-UNAVAILABLE" / "BUSY-TENTATIVE"
//                        / x-name
//              ; Some experimental iCalendar free/busy type.
//                        / iana-token)
//              ; Some other IANA-registered iCalendar free/busy type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FbtypeParam(pub Fbtype);

impl ICalendarEntity for FbtypeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "FBTYPEPARAM",
            map(
                pair(
                    tag("FBTYPE"),
                    preceded(tag("="), cut(Fbtype::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("FBTYPE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(FbtypeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=FREE TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::Free),
            ),
        );

        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=BUSY TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::Busy),
            ),
        );

        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=BUSY-UNAVAILABLE TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::BusyUnavailable),
            ),
        );

        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=BUSY-TENTATIVE TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::BusyTentative),
            ),
        );

        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::XName(String::from("X-TEST-NAME"))),
            ),
        );

        assert_parser_output!(
            FbtypeParam::parse_ical(r#"FBTYPE=TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                FbtypeParam(Fbtype::IanaToken(String::from("TEST-IANA-NAME"))),
            ),
        );

        assert!(FbtypeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            FbtypeParam(Fbtype::Free).render_ical(),
            String::from("FBTYPE=FREE"),
        );

        assert_eq!(
            FbtypeParam(Fbtype::Busy).render_ical(),
            String::from("FBTYPE=BUSY"),
        );

        assert_eq!(
            FbtypeParam(Fbtype::BusyUnavailable).render_ical(),
            String::from("FBTYPE=BUSY-UNAVAILABLE"),
        );

        assert_eq!(
            FbtypeParam(Fbtype::BusyTentative).render_ical(),
            String::from("FBTYPE=BUSY-TENTATIVE"),
        );

        assert_eq!(
            FbtypeParam(Fbtype::XName(String::from("X-TEST-NAME"))).render_ical(),
            String::from("FBTYPE=X-TEST-NAME"),
        );

        assert_eq!(
            FbtypeParam(Fbtype::IanaToken(String::from("TEST-IANA-NAME"))).render_ical(),
            String::from("FBTYPE=TEST-IANA-NAME"),
        );
    }
}
