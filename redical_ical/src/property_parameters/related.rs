use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// RELATED = ("START"       ; Trigger off of start
//          / "END")        ; Trigger off of end
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Related {
    Start,
    End,
}

impl ICalendarEntity for Related {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RELATED",
            alt((
                map(tag("START"), |_| Related::Start),
                map(tag("END"), |_| Related::End),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
           Self::Start => String::from("START"),
           Self::End => String::from("END"),
        }
    }
}

impl_icalendar_entity_traits!(Related);

// Alarm Trigger Relationship
//
// Parameter Name:  RELATED
//
// Purpose:  To specify the relationship of the alarm trigger with
//    respect to the start or end of the calendar component.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     trigrelparam       = "RELATED" "="
//                         ("START"       ; Trigger off of start
//                        / "END")        ; Trigger off of end
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RelatedParam(Related);

impl ICalendarEntity for RelatedParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TRIGRELPARAM",
            map(
                pair(
                    tag("RELATED"),
                    preceded(tag("="), cut(Related::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RELATED={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(RelatedParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RelatedParam::parse_ical(r#"RELATED=START TESTING"#.into()),
            (
                " TESTING",
                RelatedParam(Related::Start),
            ),
        );

        assert_parser_output!(
            RelatedParam::parse_ical(r#"RELATED=END TESTING"#.into()),
            (
                " TESTING",
                RelatedParam(Related::End),
            ),
        );

        assert!(RelatedParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RelatedParam(Related::Start).render_ical(),
            String::from("RELATED=START"),
        );

        assert_eq!(
            RelatedParam(Related::End).render_ical(),
            String::from("RELATED=END"),
        );
    }
}
