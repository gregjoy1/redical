use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::{tag, x_name, iana_token};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

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
            map_err_message!(
                alt((
                    map(tag("PARENT"), |_| Reltype::Parent),
                    map(tag("CHILD"), |_| Reltype::Child),
                    map(tag("SIBLING"), |_| Reltype::Sibling),
                    map(x_name, |value| Reltype::XName(value.to_string())),
                    map(iana_token, |value| Reltype::IanaToken(value.to_string())),
                )),
                "expected either \"PARENT\", \"CHILD\", \"SIBLING\" or iCalendar RFC-5545 X-NAME or IANA-TOKEN chars",
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Reltype::parse_ical(r#"PARENT TESTING"#.into()),
            (
                " TESTING",
                Reltype::Parent,
            ),
        );

        assert_parser_output!(
            Reltype::parse_ical(r#"CHILD TESTING"#.into()),
            (
                " TESTING",
                Reltype::Child,
            ),
        );

        assert_parser_output!(
            Reltype::parse_ical(r#"SIBLING TESTING"#.into()),
            (
                " TESTING",
                Reltype::Sibling,
            ),
        );

        assert_parser_output!(
            Reltype::parse_ical(r#"X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                Reltype::XName(String::from("X-TEST-NAME")),
            ),
        );

        assert_parser_output!(
            Reltype::parse_ical(r#"TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                Reltype::IanaToken(String::from("TEST-IANA-NAME")),
            ),
        );

        assert!(Reltype::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Reltype::Parent.render_ical(),
            String::from("PARENT"),
        );

        assert_eq!(
            Reltype::Child.render_ical(),
            String::from("CHILD"),
        );

        assert_eq!(
            Reltype::Sibling.render_ical(),
            String::from("SIBLING"),
        );

        assert_eq!(
            Reltype::XName(String::from("X-TEST-NAME")).render_ical(),
            String::from("X-TEST-NAME"),
        );

        assert_eq!(
            Reltype::IanaToken(String::from("TEST-IANA-NAME")).render_ical(),
            String::from("TEST-IANA-NAME"),
        );
    }
}
