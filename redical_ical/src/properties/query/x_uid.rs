use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut, opt};

use crate::grammar::{tag, colon};

use crate::values::text::Text;
use crate::values::list::List;

use crate::properties::ICalendarProperty;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

/// Query UID where condition property.
///
/// Example:
///
/// X-UID:UID_ONE
/// X-UID:UID_ONE,UID_TWO (equivalent X-UID:UID_ONE OR X-UID:UID_TWO)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XUIDProperty {
    pub uids: List<Text>,
    pub negated: bool,
}

impl ICalendarEntity for XUIDProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-UID",
            preceded(
                tag("X-UID"),
                cut(
                    map(
                        pair(
                            opt(tag("-NOT")),
                            preceded(colon, List::parse_ical),
                        ),
                        |(not, uids)| {
                            XUIDProperty {
                                uids,
                                negated: not.is_some(),
                            }
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

impl ICalendarProperty for XUIDProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        let property = if self.negated { "X-UID-NOT" } else { "X-UID" };

        ContentLine::from((
            property,
            (
                ContentLineParams::default(),
                self.uids.to_string(),
            )
        ))
    }
}

impl XUIDProperty {
    /// Return all UID Strings (blanks stripped out).
    pub fn get_uids(&self) -> Vec<String> {
        self.uids
            .iter()
            .map(|text| text.to_string())
            .skip_while(|text| text.is_empty())
            .collect::<Vec<String>>()
    }
}

impl std::hash::Hash for XUIDProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XUIDProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID:UID_ONE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    uids: List::from(vec![Text(String::from("UID_ONE"))]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID-NOT:UID_ONE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    uids: List::from(vec![Text(String::from("UID_ONE"))]),
                    negated: true,
                },
            ),
        );

        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID:UID_ONE,UID_TWO DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID-NOT:UID_ONE,UID_TWO DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                    negated: true,
                },
            ),
        );

        assert!(XUIDProperty::parse_ical(":".into()).is_err());
        assert!(XUIDProperty::parse_ical("X-UID;OP=OR:UID_ONE".into()).is_err());
        assert!(XUIDProperty::parse_ical("X-UID;OP=AND:UID_ONE".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XUIDProperty {
                uids: List::from(vec![Text(String::from("UID_ONE"))]),
                negated: false,
            }.render_ical(),
            String::from("X-UID:UID_ONE"),
        );

        assert_eq!(
            XUIDProperty {
                uids: List::from(vec![Text(String::from("UID_ONE"))]),
                negated: true,
            }.render_ical(),
            String::from("X-UID-NOT:UID_ONE"),
        );

        assert_eq!(
            XUIDProperty {
                uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                negated: false,
            }.render_ical(),
            String::from("X-UID:UID_ONE,UID_TWO"),
        );

        assert_eq!(
            XUIDProperty {
                uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                negated: true,
            }.render_ical(),
            String::from("X-UID-NOT:UID_ONE,UID_TWO"),
        );
    }
}
