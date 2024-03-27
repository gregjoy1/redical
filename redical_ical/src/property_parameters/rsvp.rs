use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::boolean::Boolean;

// RSVP Expectation
//
// Parameter Name:  RSVP
//
// Purpose:  To specify whether there is an expectation of a favor of a
//    reply from the calendar user specified by the property value.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     rsvpparam = "RSVP" "=" ("TRUE" / "FALSE")
//     ; Default is FALSE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RsvpParam(Boolean);

impl ICalendarEntity for RsvpParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RSVPPARAM",
            map(
                pair(
                    tag("RSVP"),
                    preceded(tag("="), cut(Boolean::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RSVP={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(RsvpParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RsvpParam::parse_ical(r#"RSVP=TRUE TESTING"#.into()),
            (
                " TESTING",
                RsvpParam(Boolean::True),
            ),
        );

        assert_parser_output!(
            RsvpParam::parse_ical(r#"RSVP=FALSE TESTING"#.into()),
            (
                " TESTING",
                RsvpParam(Boolean::False),
            ),
        );

        assert!(RsvpParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RsvpParam(Boolean::True).render_ical(),
            String::from("RSVP=TRUE"),
        );

        assert_eq!(
            RsvpParam(Boolean::False).render_ical(),
            String::from("RSVP=FALSE"),
        );
    }
}
