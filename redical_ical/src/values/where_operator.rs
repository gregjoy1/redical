use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::tag;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// OP = "OR" / "AND"
//
// ;Default is AND
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WhereOperator {
    Or,
    And,
}

impl ICalendarEntity for WhereOperator {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "OP",
            alt((
                map(tag("OR"), |_| WhereOperator::Or),
                map(tag("AND"), |_| WhereOperator::And),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::Or => String::from("OR"),
           Self::And => String::from("AND"),
        }
    }
}

impl_icalendar_entity_traits!(WhereOperator);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            WhereOperator::parse_ical("AND DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereOperator::And,
            ),
        );

        assert_parser_output!(
            WhereOperator::parse_ical("OR DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereOperator::Or,
            ),
        );

        assert!(WhereOperator::parse_ical(":".into()).is_err());
        assert!(WhereOperator::parse_ical("ELSE".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            WhereOperator::And.render_ical(),
            String::from("AND"),
        );

        assert_eq!(
            WhereOperator::Or.render_ical(),
            String::from("OR"),
        );
    }
}
