use nom::error::context;
use nom::sequence::{pair, preceded, tuple};
use nom::combinator::{map_res, cut, opt};

use crate::grammar::{tag, semicolon, colon};

use crate::values::list::List;

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::values::class::ClassValue;
use crate::values::where_operator::WhereOperator;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XClassPropertyParams {
    pub op: WhereOperator,
}

impl ICalendarEntity for XClassPropertyParams {
    define_property_params_ical_parser!(
        XClassPropertyParams,
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereOperator::parse_ical))),
            |params: &mut XClassPropertyParams, (_key, value): (ParserInput, WhereOperator)| params.op = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XClassPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("OP"), self.op.render_ical());

        content_line_params
    }
}

impl From<XClassPropertyParams> for ContentLineParams {
    fn from(classes_params: XClassPropertyParams) -> Self {
        ContentLineParams::from(&classes_params)
    }
}

impl Default for XClassPropertyParams {
    fn default() -> Self {
        XClassPropertyParams {
            op: WhereOperator::And,
        }
    }
}

/// Query CLASS where condition property.
///
/// Example:
///
/// X-CLASS:PUBLIC
/// X-CLASS:PUBLIC,CONFIDENTIAL  => X-CLASS;OP=AND:PUBLIC,CONFIDENTIAL
/// X-CLASS;OP=OR:PUBLIC,CONFIDENTIAL
/// X-CLASS;OP=AND:PUBLIC,CONFIDENTIAL
///
/// Negated:
///
/// X-CLASS-NOT:PUBLIC
/// X-CLASS-NOT:PUBLIC,CONFIDENTIAL  => X-CLASS;OP=AND:PUBLIC,CONFIDENTIAL
/// X-CLASS-NOT;OP=OR:PUBLIC,CONFIDENTIAL
/// X-CLASS-NOT;OP=AND:PUBLIC,CONFIDENTIAL
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XClassProperty {
    pub params: XClassPropertyParams,
    pub classes: List<ClassValue>,
    pub negated: bool,
}

impl ICalendarEntity for XClassProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-CLASS",
            preceded(
                tag("X-CLASS"),
                cut(
                    map_res(
                        tuple(
                            (
                                opt(tag("-NOT")),
                                opt(XClassPropertyParams::parse_ical),
                                preceded(colon, List::parse_ical),
                            )
                        ),
                        |(not, params, classes)| {
                            let property = XClassProperty {
                                params: params.unwrap_or_default(),
                                classes,
                                negated: not.is_some(),
                            };

                            if property.negated && property.params.op != WhereOperator::And {
                                return Err(
                                    ParserError::new(
                                        String::from("incompatible NOT operator"),
                                        input
                                    )
                                );
                            }

                            Ok(property)
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

impl ICalendarProperty for XClassProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        let property = if self.negated { "X-CLASS-NOT" } else { "X-CLASS" };

        ContentLine::from((
            property,
            (
                ContentLineParams::from(&self.params),
                self.classes.to_string(),
            )
        ))
    }
}

impl XClassProperty {
    pub fn get_classifications(&self) -> Vec<String> {
        self.classes
            .iter()
            .map(|text| text.to_string())
            .collect::<Vec<String>>()
    }
}

impl std::hash::Hash for XClassProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XClassProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XClassProperty::parse_ical("X-CLASS:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XClassProperty {
                    params: XClassPropertyParams { op: WhereOperator::And },
                    classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XClassProperty::parse_ical("X-CLASS-NOT:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XClassProperty {
                    params: XClassPropertyParams { op: WhereOperator::And },
                    classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                    negated: true,
                },
            ),
        );

        assert_parser_output!(
            XClassProperty::parse_ical("X-CLASS;OP=AND:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XClassProperty {
                    params: XClassPropertyParams { op: WhereOperator::And },
                    classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XClassProperty::parse_ical("X-CLASS-NOT;OP=AND:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XClassProperty {
                    params: XClassPropertyParams { op: WhereOperator::And },
                    classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                    negated: true,
                },
            ),
        );

        assert_parser_output!(
            XClassProperty::parse_ical("X-CLASS;OP=OR:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XClassProperty {
                    params: XClassPropertyParams { op: WhereOperator::Or },
                    classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                    negated: false,
                },
            ),
        );

        assert!(XClassProperty::parse_ical("X-CLASS-NOT;OP=OR:PUBLIC,PRIVATE".into()).is_err());
        assert!(XClassProperty::parse_ical(":".into()).is_err());
        assert!(XClassProperty::parse_ical("X-CLASS;OP=WRONG:PUBLIC".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XClassProperty {
                params: XClassPropertyParams { op: WhereOperator::And },
                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                negated: false,
            }.render_ical(),
            String::from("X-CLASS;OP=AND:PRIVATE,PUBLIC"),
        );

        assert_eq!(
            XClassProperty {
                params: XClassPropertyParams { op: WhereOperator::And },
                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                negated: true,
            }.render_ical(),
            String::from("X-CLASS-NOT;OP=AND:PRIVATE,PUBLIC"),
        );

        assert_eq!(
            XClassProperty {
                params: XClassPropertyParams { op: WhereOperator::Or },
                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                negated: false,
            }.render_ical(),
            String::from("X-CLASS;OP=OR:PRIVATE,PUBLIC"),
        );

        assert_eq!(
            XClassProperty {
                params: XClassPropertyParams { op: WhereOperator::Or },
                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                negated: true,
            }.render_ical(),
            String::from("X-CLASS-NOT;OP=OR:PRIVATE,PUBLIC"),
        );
    }
}
