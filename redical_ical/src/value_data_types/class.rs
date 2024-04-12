use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::{tag, x_name, iana_token};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// classvalue = "PUBLIC" / "PRIVATE" / "CONFIDENTIAL" / iana-token
//            / x-name
// ;Default is PUBLIC
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum ClassValue {
    Public,
    Private,
    Confidential,
    XName(String),
    IanaToken(String),
}

impl ICalendarEntity for ClassValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CLASSVALUE",
            alt((
                map(tag("PUBLIC"), |_| ClassValue::Public),
                map(tag("PRIVATE"), |_| ClassValue::Private),
                map(tag("CONFIDENTIAL"), |_| ClassValue::Confidential),
                map(x_name, |value| ClassValue::XName(value.to_string())),
                map(iana_token, |value| ClassValue::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::Public => String::from("PUBLIC"),
           Self::Private => String::from("PRIVATE"),
           Self::Confidential => String::from("CONFIDENTIAL"),
           Self::XName(name) => name.to_owned(),
           Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(ClassValue);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ClassValue::parse_ical(r#"PUBLIC TESTING"#.into()),
            (
                " TESTING",
                ClassValue::Public,
            ),
        );

        assert_parser_output!(
            ClassValue::parse_ical(r#"PRIVATE TESTING"#.into()),
            (
                " TESTING",
                ClassValue::Private,
            ),
        );

        assert_parser_output!(
            ClassValue::parse_ical(r#"CONFIDENTIAL TESTING"#.into()),
            (
                " TESTING",
                ClassValue::Confidential,
            ),
        );

        assert_parser_output!(
            ClassValue::parse_ical(r#"X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                ClassValue::XName(String::from("X-TEST-NAME")),
            ),
        );

        assert_parser_output!(
            ClassValue::parse_ical(r#"TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                ClassValue::IanaToken(String::from("TEST-IANA-NAME")),
            ),
        );

        assert!(ClassValue::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ClassValue::Public.render_ical(),
            String::from("PUBLIC"),
        );

        assert_eq!(
            ClassValue::Private.render_ical(),
            String::from("PRIVATE"),
        );

        assert_eq!(
            ClassValue::Confidential.render_ical(),
            String::from("CONFIDENTIAL"),
        );

        assert_eq!(
            ClassValue::XName(String::from("X-TEST-NAME")).render_ical(),
            String::from("X-TEST-NAME"),
        );

        assert_eq!(
            ClassValue::IanaToken(String::from("TEST-IANA-NAME")).render_ical(),
            String::from("TEST-IANA-NAME"),
        );
    }
}
