use nom::error::context;
use nom::branch::alt;
use nom::combinator::map;

use crate::grammar::tag;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

// OP = "GT" / "GTE"
//
// ;Default is GT
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WhereFromRangeOperator {
    GreaterThan,
    GreaterEqualThan,
}

impl ICalendarEntity for WhereFromRangeOperator {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "OP",
            map_err_message!(
                alt((
                    map(tag("GTE"), |_| WhereFromRangeOperator::GreaterEqualThan),
                    map(tag("GT"), |_| WhereFromRangeOperator::GreaterThan),
                )),
                "expected either \"GTE\" or \"GT\"",
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::GreaterThan => String::from("GT"),
           Self::GreaterEqualThan => String::from("GTE"),
        }
    }
}

impl_icalendar_entity_traits!(WhereFromRangeOperator);

// OP = "LT" / "LTE"
//
// ;Default is LT
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum WhereUntilRangeOperator {
    LessThan,
    LessEqualThan,
}

impl ICalendarEntity for WhereUntilRangeOperator {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "OP",
            map_err_message!(
                alt((
                    map(tag("LTE"), |_| WhereUntilRangeOperator::LessEqualThan),
                    map(tag("LT"), |_| WhereUntilRangeOperator::LessThan),
                )),
                "expected either \"LTE\" or \"LT\"",
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::LessThan => String::from("LT"),
           Self::LessEqualThan => String::from("LTE"),
        }
    }
}

impl_icalendar_entity_traits!(WhereUntilRangeOperator);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn from_range_operator_parse_ical() {
        assert_parser_output!(
            WhereFromRangeOperator::parse_ical("GT DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereFromRangeOperator::GreaterThan,
            ),
        );

        assert_parser_output!(
            WhereFromRangeOperator::parse_ical("GTE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereFromRangeOperator::GreaterEqualThan,
            ),
        );

        assert!(WhereFromRangeOperator::parse_ical("LT".into()).is_err());
        assert!(WhereFromRangeOperator::parse_ical("LTE".into()).is_err());
    }

    #[test]
    fn from_range_property_render_ical() {
        assert_eq!(
            WhereFromRangeOperator::GreaterThan.render_ical(),
            String::from("GT"),
        );

        assert_eq!(
            WhereFromRangeOperator::GreaterEqualThan.render_ical(),
            String::from("GTE"),
        );
    }

    #[test]
    fn until_range_operator_parse_ical() {
        assert_parser_output!(
            WhereUntilRangeOperator::parse_ical("LT DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereUntilRangeOperator::LessThan,
            ),
        );

        assert_parser_output!(
            WhereUntilRangeOperator::parse_ical("LTE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WhereUntilRangeOperator::LessEqualThan,
            ),
        );

        assert!(WhereUntilRangeOperator::parse_ical("GT".into()).is_err());
        assert!(WhereUntilRangeOperator::parse_ical("GTE".into()).is_err());
    }

    #[test]
    fn until_range_property_render_ical() {
        assert_eq!(
            WhereUntilRangeOperator::LessThan.render_ical(),
            String::from("LT"),
        );

        assert_eq!(
            WhereUntilRangeOperator::LessEqualThan.render_ical(),
            String::from("LTE"),
        );
    }
}
