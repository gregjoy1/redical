use nom::error::context;
use nom::sequence::preceded;
use nom::combinator::{map_res, cut};

use crate::grammar::{tag, colon};

use crate::properties::ICalendarProperty;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

use crate::values::integer::Integer;

/// Query offset property, does what it says on the tin.
///
/// Example:
///
/// X-OFFSET:50
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XOffsetProperty {
    pub offset: Integer,
}

impl Into<usize> for XOffsetProperty {
    fn into(self) -> usize {
        *self.offset as usize
    }
}

impl Into<usize> for &XOffsetProperty {
    fn into(self) -> usize {
        self.to_owned().into()
    }
}

impl ICalendarEntity for XOffsetProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-OFFSET",
            preceded(
                tag("X-OFFSET"),
                cut(
                    map_res(
                        preceded(colon, Integer::parse_ical),
                        |offset| {
                            if *offset < 0 {
                                return Err(
                                    ParserError::new(String::from("Offset cannot be less than 0"), input)
                                );
                            }

                            Ok(XOffsetProperty { offset })
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

impl ICalendarProperty for XOffsetProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-OFFSET",
            (
                ContentLineParams::default(),
                self.offset.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for XOffsetProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XOffsetProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XOffsetProperty::parse_ical("X-OFFSET:50 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XOffsetProperty {
                    offset: Integer(50_i64),
                },
            ),
        );

        assert_parser_output!(
            XOffsetProperty::parse_ical("X-OFFSET:0 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XOffsetProperty {
                    offset: Integer(0_i64),
                },
            ),
        );

        assert!(XOffsetProperty::parse_ical("X-OFFSET:-50".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XOffsetProperty {
                offset: Integer(50_i64),
            }.render_ical(),
            String::from("X-OFFSET:50"),
        );
    }
}
