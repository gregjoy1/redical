use nom::branch::alt;
use nom::combinator::map;
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

/// Parse boolean chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::value_data_types::boolean::boolean;
///
/// assert!(boolean("TRUE".into()).is_ok());
/// assert!(boolean("FALSE".into()).is_ok());
///
/// assert!(boolean("0".into()).is_err());
/// assert!(boolean("1".into()).is_err());
/// assert!(boolean("true".into()).is_err());
/// assert!(boolean("false".into()).is_err());
/// assert!(boolean("OTHER".into()).is_err());
/// assert!(boolean(":".into()).is_err());
/// ```
///
/// boolean    = "TRUE" / "FALSE"
pub fn boolean(input: ParserInput) -> ParserResult<ParserInput> {
    alt(
        (
            tag("TRUE"),
            tag("FALSE"),
        )
    )(input)
}

// Value Name:  BOOLEAN
//
// Purpose:  This value type is used to identify properties that contain
//    either a "TRUE" or "FALSE" Boolean value.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     boolean    = "TRUE" / "FALSE"
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Boolean {
    True,
    False,
}

impl ICalendarEntity for Boolean {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        alt(
            (
                map(tag("TRUE"), |_| Boolean::True),
                map(tag("FALSE"), |_| Boolean::False),
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Boolean::True => String::from("TRUE"),
            Boolean::False => String::from("FALSE"),
        }
    }
}

impl_icalendar_entity_traits!(Boolean);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Boolean::parse_ical("TRUE TESTING".into()),
            (
                " TESTING",
                Boolean::True,
            ),
        );

        assert_parser_output!(
            Boolean::parse_ical("FALSE TESTING".into()),
            (
                " TESTING",
                Boolean::False,
            ),
        );

        assert!(Boolean::parse_ical("0".into()).is_err());
        assert!(Boolean::parse_ical("1".into()).is_err());
        assert!(Boolean::parse_ical("true".into()).is_err());
        assert!(Boolean::parse_ical("false".into()).is_err());
        assert!(Boolean::parse_ical("OTHER".into()).is_err());
        assert!(Boolean::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(Boolean::True.render_ical(), String::from("TRUE"));
        assert_eq!(Boolean::False.render_ical(), String::from("FALSE"));
    }
}
