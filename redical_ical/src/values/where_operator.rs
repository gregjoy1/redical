use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::tag;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

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
            map_err_message!(
                alt((
                    map(tag("OR"), |_| WhereOperator::Or),
                    map(tag("AND"), |_| WhereOperator::And),
                )),
                "expected either \"OR\" or \"AND\"",
            ),
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

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical_error() {
        assert_parser_error!(
            WhereOperator::parse_ical(":::: DESCRIPTION:Description text".into()),
            nom::Err::Error(
                span: ":::: DESCRIPTION:Description text",
                message: "expected either \"OR\" or \"AND\"",
                context: ["OP"],
            ),
        );
    }

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
