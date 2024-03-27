use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{x_name, iana_token};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// CUTYPE = ("INDIVIDUAL"   ; An individual
//          / "GROUP"        ; A group of individuals
//          / "RESOURCE"     ; A physical resource
//          / "ROOM"         ; A room resource
//          / "UNKNOWN"      ; Otherwise not known
//          / x-name         ; Experimental type
//          / iana-token)    ; Other IANA-registered
//                           ; type
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cutype {
    Individual,        // An individual
    Group,             // A group of individuals
    Resource,          // A physical resource
    Room,              // A room resource
    Unknown,           // Otherwise not known
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl ICalendarEntity for Cutype {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CUTYPE",
            alt((
                map(tag("INDIVIDUAL"), |_| Cutype::Individual),
                map(tag("GROUP"), |_| Cutype::Group),
                map(tag("RESOURCE"), |_| Cutype::Resource),
                map(tag("ROOM"), |_| Cutype::Room),
                map(tag("UNKNOWN"), |_| Cutype::Unknown),
                map(x_name, |value| Cutype::XName(value.to_string())),
                map(iana_token, |value| Cutype::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
           Self::Individual => String::from("INDIVIDUAL"),
           Self::Group => String::from("GROUP"),
           Self::Resource => String::from("RESOURCE"),
           Self::Room => String::from("ROOM"),
           Self::Unknown => String::from("UNKNOWN"),
           Self::XName(name) => name.to_owned(),
           Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(Cutype);

// Calendar User Type
//
// Parameter Name:  CUTYPE
//
// Purpose:  To identify the type of calendar user specified by the
//    property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     cutypeparam        = "CUTYPE" "="
//                        ("INDIVIDUAL"   ; An individual
//                       / "GROUP"        ; A group of individuals
//                       / "RESOURCE"     ; A physical resource
//                       / "ROOM"         ; A room resource
//                       / "UNKNOWN"      ; Otherwise not known
//                       / x-name         ; Experimental type
//                       / iana-token)    ; Other IANA-registered
//                                        ; type
//     ; Default is INDIVIDUAL
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CutypeParam(Cutype);

impl ICalendarEntity for CutypeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CUTYPEPARAM",
            map(
                pair(
                    tag("CUTYPE"),
                    preceded(tag("="), cut(Cutype::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CUTYPE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(CutypeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=INDIVIDUAL TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::Individual),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=GROUP TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::Group),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=RESOURCE TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::Resource),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=ROOM TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::Room),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=UNKNOWN TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::Unknown),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::XName(String::from("X-TEST-NAME"))),
            ),
        );

        assert_parser_output!(
            CutypeParam::parse_ical(r#"CUTYPE=TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                CutypeParam(Cutype::IanaToken(String::from("TEST-IANA-NAME"))),
            ),
        );

        assert!(CutypeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            CutypeParam(Cutype::Individual).render_ical(),
            String::from("CUTYPE=INDIVIDUAL"),
        );

        assert_eq!(
            CutypeParam(Cutype::Group).render_ical(),
            String::from("CUTYPE=GROUP"),
        );

        assert_eq!(
            CutypeParam(Cutype::Resource).render_ical(),
            String::from("CUTYPE=RESOURCE"),
        );

        assert_eq!(
            CutypeParam(Cutype::Room).render_ical(),
            String::from("CUTYPE=ROOM"),
        );

        assert_eq!(
            CutypeParam(Cutype::Unknown).render_ical(),
            String::from("CUTYPE=UNKNOWN"),
        );

        assert_eq!(
            CutypeParam(Cutype::XName(String::from("X-TEST-NAME"))).render_ical(),
            String::from("CUTYPE=X-TEST-NAME"),
        );

        assert_eq!(
            CutypeParam(Cutype::IanaToken(String::from("TEST-IANA-NAME"))).render_ical(),
            String::from("CUTYPE=TEST-IANA-NAME"),
        );
    }
}
