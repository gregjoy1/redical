use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut, opt};

use crate::grammar::{tag, semicolon, colon};

use crate::values::text::Text;
use crate::values::list::List;
use crate::values::where_operator::WhereOperator;

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XUIDPropertyParams {
    pub op: WhereOperator,
}

impl ICalendarEntity for XUIDPropertyParams {
    define_property_params_ical_parser!(
        XUIDPropertyParams,
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereOperator::parse_ical))),
            |params: &mut XUIDPropertyParams, (_key, value): (ParserInput, WhereOperator)| params.op = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XUIDPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("OP"), self.op.render_ical());

        content_line_params
    }
}

impl From<XUIDPropertyParams> for ContentLineParams {
    fn from(uid_params: XUIDPropertyParams) -> Self {
        ContentLineParams::from(&uid_params)
    }
}

impl Default for XUIDPropertyParams {
    fn default() -> Self {
        XUIDPropertyParams {
            op: WhereOperator::And,
        }
    }
}

/// Query UID where condition property.
///
/// Example:
///
/// X-UID:UID_ONE
/// X-UID:UID_ONE,UID_TWO (equivalent X-UID;OP=AND:UID_ONE,UID_TWO)
/// X-UID;OP=OR:UID_ONE,UID_TWO
/// X-UID;OP=AND:UID_ONE,UID_TWO
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XUIDProperty {
    pub params: XUIDPropertyParams,
    pub uids: List<Text>,
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
                            opt(XUIDPropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, uids)| {
                            XUIDProperty {
                                params: params.unwrap_or(XUIDPropertyParams::default()),
                                uids,
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
        ContentLine::from((
            "X-UID",
            (
                ContentLineParams::from(&self.params),
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
            XUIDProperty::parse_ical("X-UID:UID_ONE,UID_TWO DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    params: XUIDPropertyParams { op: WhereOperator::And },
                    uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                },
            ),
        );

        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID;OP=AND:UID_ONE,UID_TWO DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    params: XUIDPropertyParams { op: WhereOperator::And },
                    uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                },
            ),
        );

        assert_parser_output!(
            XUIDProperty::parse_ical("X-UID;OP=OR:UID_ONE,UID_TWO DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUIDProperty {
                    params: XUIDPropertyParams { op: WhereOperator::Or },
                    uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                },
            ),
        );

        assert!(XUIDProperty::parse_ical(":".into()).is_err());
        assert!(XUIDProperty::parse_ical("X-UID;OP=WRONG:UID_ONE".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XUIDProperty {
                params: XUIDPropertyParams { op: WhereOperator::And },
                uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
            }.render_ical(),
            String::from("X-UID;OP=AND:UID_ONE,UID_TWO"),
        );

        assert_eq!(
            XUIDProperty {
                params: XUIDPropertyParams { op: WhereOperator::Or },
                uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
            }.render_ical(),
            String::from("X-UID;OP=OR:UID_ONE,UID_TWO"),
        );
    }
}
