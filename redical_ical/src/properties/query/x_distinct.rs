use nom::error::context;
use nom::sequence::preceded;
use nom::combinator::{map, cut};

use crate::grammar::{tag, colon};

use crate::properties::ICalendarProperty;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DistinctValue {
    UID,
}

impl ICalendarEntity for DistinctValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-DISTINCT VALUE",
            map(tag("UID"), |_| DistinctValue::UID),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::UID => String::from("UID"),
        }
    }
}

impl_icalendar_entity_traits!(DistinctValue);

/// Query distinct property, groups by the specified value.
///
/// Currently only UID value is available.
///
/// Example:
///
/// X-DISTINCT:UID
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XDistinctProperty {
    pub value: DistinctValue,
}

impl ICalendarEntity for XDistinctProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-DISTINCT",
            preceded(
                tag("X-DISTINCT"),
                cut(
                    map(
                        preceded(colon, DistinctValue::parse_ical),
                        |value| {
                            XDistinctProperty { value }
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

impl ICalendarProperty for XDistinctProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-DISTINCT",
            (
                ContentLineParams::default(),
                self.value.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for XDistinctProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XDistinctProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XDistinctProperty::parse_ical("X-DISTINCT:UID DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XDistinctProperty {
                    value: DistinctValue::UID,
                },
            ),
        );

        assert!(XDistinctProperty::parse_ical("X-DISTINCT:SOMETHING".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XDistinctProperty {
                value: DistinctValue::UID,
            }.render_ical(),
            String::from("X-DISTINCT:UID"),
        );
    }
}
