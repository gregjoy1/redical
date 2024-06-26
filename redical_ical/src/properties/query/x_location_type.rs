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
pub struct XLocationTypePropertyParams {
    pub op: WhereOperator,
}

impl ICalendarEntity for XLocationTypePropertyParams {
    define_property_params_ical_parser!(
        XLocationTypePropertyParams,
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereOperator::parse_ical))),
            |params: &mut XLocationTypePropertyParams, (_key, value): (ParserInput, WhereOperator)| params.op = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XLocationTypePropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("OP"), self.op.render_ical());

        content_line_params
    }
}

impl From<XLocationTypePropertyParams> for ContentLineParams {
    fn from(types_params: XLocationTypePropertyParams) -> Self {
        ContentLineParams::from(&types_params)
    }
}

impl Default for XLocationTypePropertyParams {
    fn default() -> Self {
        XLocationTypePropertyParams {
            op: WhereOperator::And,
        }
    }
}

/// Query LOCATION-TYPE where condition property.
///
/// Example:
///
/// X-LOCATION-TYPE:ONLINE
/// X-LOCATION-TYPE:HOTEL,RESTAURANT (equivalent X-LOCATION-TYPE;OP=AND:HOTEL,RESTAURANT)
/// X-LOCATION-TYPE;OP=OR:HOTEL,RESTAURANT
/// X-LOCATION-TYPE;OP=AND:HOTEL,RESTAURANT
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XLocationTypeProperty {
    pub params: XLocationTypePropertyParams,
    pub types: List<Text>,
}

impl ICalendarEntity for XLocationTypeProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-LOCATION-TYPE",
            preceded(
                tag("X-LOCATION-TYPE"),
                cut(
                    map(
                        pair(
                            opt(XLocationTypePropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, types)| {
                            XLocationTypeProperty {
                                params: params.unwrap_or(XLocationTypePropertyParams::default()),
                                types,
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

impl ICalendarProperty for XLocationTypeProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-LOCATION-TYPE",
            (
                ContentLineParams::from(&self.params),
                self.types.to_string(),
            )
        ))
    }
}

impl XLocationTypeProperty {
    /// Return all type Strings (blanks stripped out).
    pub fn get_location_types(&self) -> Vec<String> {
        self.types
            .iter()
            .map(|text| text.to_string())
            .skip_while(|text| text.is_empty())
            .collect::<Vec<String>>()
    }
}

impl std::hash::Hash for XLocationTypeProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XLocationTypeProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XLocationTypeProperty::parse_ical("X-LOCATION-TYPE:RESTAURANT,HOTEL DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XLocationTypeProperty {
                    params: XLocationTypePropertyParams { op: WhereOperator::And },
                    types: List::from(vec![Text(String::from("RESTAURANT")), Text(String::from("HOTEL"))]),
                },
            ),
        );

        assert_parser_output!(
            XLocationTypeProperty::parse_ical("X-LOCATION-TYPE;OP=AND:RESTAURANT,HOTEL DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XLocationTypeProperty {
                    params: XLocationTypePropertyParams { op: WhereOperator::And },
                    types: List::from(vec![Text(String::from("RESTAURANT")), Text(String::from("HOTEL"))]),
                },
            ),
        );

        assert_parser_output!(
            XLocationTypeProperty::parse_ical("X-LOCATION-TYPE;OP=OR:RESTAURANT,HOTEL DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XLocationTypeProperty {
                    params: XLocationTypePropertyParams { op: WhereOperator::Or },
                    types: List::from(vec![Text(String::from("RESTAURANT")), Text(String::from("HOTEL"))]),
                },
            ),
        );

        assert!(XLocationTypeProperty::parse_ical(":".into()).is_err());
        assert!(XLocationTypeProperty::parse_ical("X-LOCATION-TYPE;OP=WRONG:RESTAURANT".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XLocationTypeProperty {
                params: XLocationTypePropertyParams { op: WhereOperator::And },
                types: List::from(vec![Text(String::from("RESTAURANT")), Text(String::from("HOTEL"))]),
            }.render_ical(),
            String::from("X-LOCATION-TYPE;OP=AND:HOTEL,RESTAURANT"),
        );

        assert_eq!(
            XLocationTypeProperty {
                params: XLocationTypePropertyParams { op: WhereOperator::Or },
                types: List::from(vec![Text(String::from("RESTAURANT")), Text(String::from("HOTEL"))]),
            }.render_ical(),
            String::from("X-LOCATION-TYPE;OP=OR:HOTEL,RESTAURANT"),
        );
    }
}
