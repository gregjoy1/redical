use nom::error::context;
use nom::sequence::preceded;
use nom::combinator::{map_res, cut};

use crate::grammar::{tag, colon};

use crate::properties::ICalendarProperty;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

use crate::value_data_types::integer::Integer;

/// Query limit property, does what it says on the tin.
///
/// Example:
///
/// X-LIMIT:50
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XLimitProperty {
    pub limit: Integer,
}

impl Into<usize> for XLimitProperty {
    fn into(self) -> usize {
        *self.limit as usize
    }
}

impl ICalendarEntity for XLimitProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-LIMIT",
            preceded(
                tag("X-LIMIT"),
                cut(
                    map_res(
                        preceded(colon, Integer::parse_ical),
                        |limit| {
                            if *limit < 0 {
                                return Err(
                                    ParserError::new(String::from("limit cannot be less than 0"), input)
                                );
                            }

                            Ok(XLimitProperty { limit })
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_with_context(context).render_ical()
    }
}

impl ICalendarProperty for XLimitProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-LIMIT",
            (
                ContentLineParams::default(),
                self.limit.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for XLimitProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XLimitProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XLimitProperty::parse_ical("X-LIMIT:50 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XLimitProperty {
                    limit: Integer(50_i64),
                },
            ),
        );

        assert_parser_output!(
            XLimitProperty::parse_ical("X-LIMIT:0 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XLimitProperty {
                    limit: Integer(0_i64),
                },
            ),
        );

        assert!(XLimitProperty::parse_ical("X-LIMIT:-50".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XLimitProperty {
                limit: Integer(50_i64),
            }.render_ical(),
            String::from("X-LIMIT:50"),
        );
    }
}
