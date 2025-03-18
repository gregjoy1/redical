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
pub struct XCategoriesPropertyParams {
    pub op: WhereOperator,
}

impl ICalendarEntity for XCategoriesPropertyParams {
    define_property_params_ical_parser!(
        XCategoriesPropertyParams,
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereOperator::parse_ical))),
            |params: &mut XCategoriesPropertyParams, (_key, value): (ParserInput, WhereOperator)| params.op = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XCategoriesPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("OP"), self.op.render_ical());

        content_line_params
    }
}

impl From<XCategoriesPropertyParams> for ContentLineParams {
    fn from(categories_params: XCategoriesPropertyParams) -> Self {
        ContentLineParams::from(&categories_params)
    }
}

impl Default for XCategoriesPropertyParams {
    fn default() -> Self {
        XCategoriesPropertyParams {
            op: WhereOperator::And,
        }
    }
}

/// Query CATEGORIES where condition property.
///
/// Example:
///
/// X-CATEGORIES:CATEGORY_ONE
/// X-CATEGORIES:CATEGORY_ONE,CATEGORY_TWO (equivalent X-CATEGORIES;OP=AND:CATEGORY_ONE,CATEGORY_TWO)
/// X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO
/// X-CATEGORIES;OP=AND:CATEGORY_ONE,CATEGORY_TWO
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XCategoriesProperty {
    pub params: XCategoriesPropertyParams,
    pub categories: List<Text>,
    pub negated: bool,
}

impl ICalendarEntity for XCategoriesProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-CATEGORIES",
            preceded(
                tag("X-CATEGORIES"),
                cut(
                    map(
                        pair(
                            opt(XCategoriesPropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, categories)| {
                            XCategoriesProperty {
                                params: params.unwrap_or(XCategoriesPropertyParams::default()),
                                categories,
                                negated: false,
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

impl ICalendarProperty for XCategoriesProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-CATEGORIES",
            (
                ContentLineParams::from(&self.params),
                self.categories.to_string(),
            )
        ))
    }
}

impl XCategoriesProperty {
    /// Return all category Strings (blanks stripped out).
    pub fn get_categories(&self) -> Vec<String> {
        self.categories
            .iter()
            .map(|text| text.to_string())
            .skip_while(|text| text.is_empty())
            .collect::<Vec<String>>()
    }
}

impl std::hash::Hash for XCategoriesProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XCategoriesProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XCategoriesProperty::parse_ical("X-CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XCategoriesProperty {
                    params: XCategoriesPropertyParams { op: WhereOperator::And },
                    categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XCategoriesProperty::parse_ical("X-CATEGORIES;OP=AND:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XCategoriesProperty {
                    params: XCategoriesPropertyParams { op: WhereOperator::And },
                    categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                    negated: false,
                },
            ),
        );

        assert_parser_output!(
            XCategoriesProperty::parse_ical("X-CATEGORIES;OP=OR:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XCategoriesProperty {
                    params: XCategoriesPropertyParams { op: WhereOperator::Or },
                    categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                    negated: false,
                },
            ),
        );

        assert!(XCategoriesProperty::parse_ical(":".into()).is_err());
        assert!(XCategoriesProperty::parse_ical("X-CATEGORIES;OP=WRONG:APPOINTMENT".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XCategoriesProperty {
                params: XCategoriesPropertyParams { op: WhereOperator::And },
                categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                negated: false,
            }.render_ical(),
            String::from("X-CATEGORIES;OP=AND:APPOINTMENT,EDUCATION"),
        );

        assert_eq!(
            XCategoriesProperty {
                params: XCategoriesPropertyParams { op: WhereOperator::Or },
                categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                negated: false,
            }.render_ical(),
            String::from("X-CATEGORIES;OP=OR:APPOINTMENT,EDUCATION"),
        );
    }
}
