use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{x_name, iana_token};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// RELTYPE = ("PARENT"    ; Parent relationship - Default
//          / "CHILD"     ; Child relationship
//          / "SIBLING"   ; Sibling relationship
//          / iana-token  ; Some other IANA-registered
//                        ; iCalendar relationship type
//          / x-name)     ; A non-standard, experimental
//                        ; relationship type
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Reltype {
    Parent,            // Parent relationship - Default
    Child,             // Child relationship
    Sibling,           // Sibling relationship
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl ICalendarEntity for Reltype {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RELTYPE",
            alt((
                map(tag("PARENT"), |_| Reltype::Parent),
                map(tag("CHILD"), |_| Reltype::Child),
                map(tag("SIBLING"), |_| Reltype::Sibling),
                map(x_name, |value| Reltype::XName(value.to_string())),
                map(iana_token, |value| Reltype::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
           Self::Parent => String::from("PARENT"),
           Self::Child => String::from("CHILD"),
           Self::Sibling => String::from("SIBLING"),
           Self::XName(name) => name.to_owned(),
           Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(Reltype);

// Relationship Type
//
// Parameter Name:  RELTYPE
//
// Purpose:  To specify the type of hierarchical relationship associated
//    with the calendar component specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     reltypeparam       = "RELTYPE" "="
//                         ("PARENT"    ; Parent relationship - Default
//                        / "CHILD"     ; Child relationship
//                        / "SIBLING"   ; Sibling relationship
//                        / iana-token  ; Some other IANA-registered
//                                      ; iCalendar relationship type
//                        / x-name)     ; A non-standard, experimental
//                                      ; relationship type
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ReltypeParam(pub Reltype);

impl ICalendarEntity for ReltypeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RELTYPEPARAM",
            map(
                pair(
                    tag("RELTYPE"),
                    preceded(tag("="), cut(Reltype::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RELTYPE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(ReltypeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ReltypeParam::parse_ical(r#"RELTYPE=PARENT TESTING"#.into()),
            (
                " TESTING",
                ReltypeParam(Reltype::Parent),
            ),
        );

        assert_parser_output!(
            ReltypeParam::parse_ical(r#"RELTYPE=CHILD TESTING"#.into()),
            (
                " TESTING",
                ReltypeParam(Reltype::Child),
            ),
        );

        assert_parser_output!(
            ReltypeParam::parse_ical(r#"RELTYPE=SIBLING TESTING"#.into()),
            (
                " TESTING",
                ReltypeParam(Reltype::Sibling),
            ),
        );

        assert_parser_output!(
            ReltypeParam::parse_ical(r#"RELTYPE=X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                ReltypeParam(Reltype::XName(String::from("X-TEST-NAME"))),
            ),
        );

        assert_parser_output!(
            ReltypeParam::parse_ical(r#"RELTYPE=TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                ReltypeParam(Reltype::IanaToken(String::from("TEST-IANA-NAME"))),
            ),
        );

        assert!(ReltypeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ReltypeParam(Reltype::Parent).render_ical(),
            String::from("RELTYPE=PARENT"),
        );

        assert_eq!(
            ReltypeParam(Reltype::Child).render_ical(),
            String::from("RELTYPE=CHILD"),
        );

        assert_eq!(
            ReltypeParam(Reltype::Sibling).render_ical(),
            String::from("RELTYPE=SIBLING"),
        );

        assert_eq!(
            ReltypeParam(Reltype::XName(String::from("X-TEST-NAME"))).render_ical(),
            String::from("RELTYPE=X-TEST-NAME"),
        );

        assert_eq!(
            ReltypeParam(Reltype::IanaToken(String::from("TEST-IANA-NAME"))).render_ical(),
            String::from("RELTYPE=TEST-IANA-NAME"),
        );
    }
}
