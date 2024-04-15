use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut, opt};

use crate::value_data_types::text::Text;
use crate::value_data_types::list::List;
use crate::value_data_types::reltype::Reltype;
use crate::value_data_types::where_operator::WhereOperator;

use crate::grammar::{tag, semicolon, colon};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XRelatedToPropertyParams {
    pub reltype: Reltype,
    pub op: WhereOperator,
}

impl ICalendarEntity for XRelatedToPropertyParams {
    define_property_params_ical_parser!(
        XRelatedToPropertyParams,
        (
            pair(tag("RELTYPE"), cut(preceded(tag("="), Reltype::parse_ical))),
            |params: &mut XRelatedToPropertyParams, (_key, reltype): (ParserInput, Reltype)| params.reltype = reltype,
        ),
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereOperator::parse_ical))),
            |params: &mut XRelatedToPropertyParams, (_key, value): (ParserInput, WhereOperator)| params.op = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XRelatedToPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("RELTYPE"), self.reltype.render_ical());
        content_line_params.insert(String::from("OP"), self.op.render_ical());

        content_line_params
    }
}

impl From<XRelatedToPropertyParams> for ContentLineParams {
    fn from(related_to_params: XRelatedToPropertyParams) -> Self {
        ContentLineParams::from(&related_to_params)
    }
}

impl Default for XRelatedToPropertyParams {
    fn default() -> Self {
        XRelatedToPropertyParams {
            reltype: Reltype::Parent,
            op: WhereOperator::And,
        }
    }
}

/// Query RELATED-TO where condition property.
///
/// Example:
///
/// X-RELATED-TO;RELTYPE=PARENT:PARENT_UID
/// X-RELATED-TO;RELTYPE=PARENT:PARENT_UID_ONE,PARENT_UID_TWO => X-RELATED-TO;OP=AND;RELTYPE=PARENT:PARENT_UID_ONE,PARENT_UID_TWO
/// X-RELATED-TO;RELTYPE=PARENT;OP=AND:PARENT_UID_ONE,PARENT_UID_TWO
/// X-RELATED-TO;RELTYPE=PARENT;OP=OR:PARENT_UID_ONE,PARENT_UID_TWO
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XRelatedToProperty {
    pub params: XRelatedToPropertyParams,
    pub uids: List<Text>,
}

impl ICalendarEntity for XRelatedToProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-RELATED-TO",
            preceded(
                tag("X-RELATED-TO"),
                cut(
                    map(
                        pair(
                            opt(XRelatedToPropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, uids)| {
                            XRelatedToProperty {
                                params: params.unwrap_or(XRelatedToPropertyParams::default()),
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

impl ICalendarProperty for XRelatedToProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-RELATED-TO",
            (
                ContentLineParams::from(&self.params),
                self.uids.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for XRelatedToProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XRelatedToProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XRelatedToProperty::parse_ical(
                "X-RELATED-TO:parent.uid.one,parent.uid.two DESCRIPTION:Description text".into()
            ),
            (
                " DESCRIPTION:Description text",
                XRelatedToProperty {
                    params: XRelatedToPropertyParams {
                        reltype: Reltype::Parent,
                        op: WhereOperator::And,
                    },
                    uids: List::from(vec![Text(String::from("parent.uid.one")), Text(String::from("parent.uid.two"))]),
                },
            ),
        );

        assert_parser_output!(
            XRelatedToProperty::parse_ical("X-RELATED-TO;RELTYPE=X-RELTYPE;OP=OR:x-reltype.uid.one,x-reltype.uid.two DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XRelatedToProperty {
                    params: XRelatedToPropertyParams {
                        reltype: Reltype::XName(String::from("X-RELTYPE")),
                        op: WhereOperator::Or,
                    },
                    uids: List::from(vec![Text(String::from("x-reltype.uid.one")), Text(String::from("x-reltype.uid.two"))]),
                },
            ),
        );

        assert!(XRelatedToProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XRelatedToProperty {
                params: XRelatedToPropertyParams {
                    reltype: Reltype::Parent,
                    op: WhereOperator::And,
                },
                uids: List::from(vec![Text(String::from("parent.uid.one")), Text(String::from("parent.uid.two"))]),
            }.render_ical(),
            String::from("X-RELATED-TO;RELTYPE=PARENT;OP=AND:parent.uid.one,parent.uid.two"),
        );

        assert_eq!(
            XRelatedToProperty {
                params: XRelatedToPropertyParams {
                    reltype: Reltype::XName(String::from("X-RELTYPE")),
                    op: WhereOperator::Or,
                },
                uids: List::from(vec![Text(String::from("x-reltype.uid.one")), Text(String::from("x-reltype.uid.two"))]),
            }.render_ical(),
            String::from("X-RELATED-TO;RELTYPE=X-RELTYPE;OP=OR:x-reltype.uid.one,x-reltype.uid.two"),
        );
    }
}
