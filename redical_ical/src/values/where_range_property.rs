use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::tag;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

// prop = "DTSTART" / "DTEND"
//
// ;Default is DTSTART
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WhereRangeProperty {
    DTStart,
    DTEnd,
}

impl ICalendarEntity for WhereRangeProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PROP",
            map_err_message!(
                alt((
                    map(tag("DTSTART"), |_| WhereRangeProperty::DTStart),
                    map(tag("DTEND"), |_| WhereRangeProperty::DTEnd),
                )),
                "expected either \"DTSTART\" or \"DTEND\"",
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::DTStart => String::from("DTSTART"),
           Self::DTEnd => String::from("DTEND"),
        }
    }
}

impl_icalendar_entity_traits!(WhereRangeProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical_error() {
        assert_parser_error!(
            WhereRangeProperty::parse_ical(":::: DESCRIPTION:Description text".into()),
            nom::Err::Error(
                span: ":::: DESCRIPTION:Description text",
                message: "expected either \"DTSTART\" or \"DTEND\"",
                context: ["PROP"],
            ),
        );
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            WhereRangeProperty::parse_ical("DTSTART DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereRangeProperty::DTStart,
            ),
        );

        assert_parser_output!(
            WhereRangeProperty::parse_ical("DTEND DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereRangeProperty::DTEnd,
            ),
        );

        assert!(WhereRangeProperty::parse_ical("RECURRENCE-ID".into()).is_err());
        assert!(WhereRangeProperty::parse_ical("RDATE".into()).is_err());
        assert!(WhereRangeProperty::parse_ical("EXDATE".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            WhereRangeProperty::DTStart.render_ical(),
            String::from("DTSTART"),
        );

        assert_eq!(
            WhereRangeProperty::DTEnd.render_ical(),
            String::from("DTEND"),
        );
    }
}
