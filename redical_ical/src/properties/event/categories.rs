use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::value_data_types::text::Text;
use crate::value_data_types::list::List;

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct CategoriesPropertyParams {
    pub language: Option<String>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for CategoriesPropertyParams {
    define_property_params_ical_parser!(
        CategoriesPropertyParams,
        (
            pair(tag("LANGUAGE"), cut(preceded(tag("="), param_value))),
            |params: &mut CategoriesPropertyParams, (_key, value): (ParserInput, ParserInput)| params.language = Some(value.to_string()),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut CategoriesPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for CategoriesPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(language) = self.language.as_ref() {
            content_line_params.insert(String::from("LANGUAGE"), language.to_owned());
        }

        content_line_params
    }
}

impl From<CategoriesPropertyParams> for ContentLineParams {
    fn from(categories_params: CategoriesPropertyParams) -> Self {
        ContentLineParams::from(&categories_params)
    }
}

// Categories
//
// Property Name:  CATEGORIES
//
// Purpose:  This property defines the categories for a calendar
//    component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, and language property
//    parameters can be specified on this property.
//
// Conformance:  The property can be specified within "VEVENT", "VTODO",
//    or "VJOURNAL" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     categories = "CATEGORIES" catparam ":" text *("," text)
//                  CRLF
//
//     catparam   = *(
//                ;
//                ; The following is OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" languageparam ) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
// Example:  The following are examples of this property:
//
//     CATEGORIES:APPOINTMENT,EDUCATION
//
//     CATEGORIES:MEETING
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CategoriesProperty {
    pub params: CategoriesPropertyParams,
    pub categories: List<Text>,
}

impl ICalendarEntity for CategoriesProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CATEGORIES",
            preceded(
                tag("CATEGORIES"),
                cut(
                    map(
                        pair(
                            opt(CategoriesPropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, categories)| {
                            CategoriesProperty {
                                params: params.unwrap_or(CategoriesPropertyParams::default()),
                                categories,
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

impl ICalendarProperty for CategoriesProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "CATEGORIES",
            (
                ContentLineParams::from(&self.params),
                self.categories.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for CategoriesProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(CategoriesProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            CategoriesProperty::parse_ical("CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                CategoriesProperty {
                    params: CategoriesPropertyParams::default(),
                    categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                },
            ),
        );

        assert_parser_output!(
            CategoriesProperty::parse_ical("CATEGORIES;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US:EDUCATION".into()),
            (
                "",
                CategoriesProperty {
                    params: CategoriesPropertyParams {
                        language: Some(String::from("en-US")),
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    categories: List::from(vec![Text(String::from("EDUCATION"))]),
                },
            ),
        );

        assert!(CategoriesProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            CategoriesProperty {
                params: CategoriesPropertyParams::default(),
                categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
            }.render_ical(),
            String::from("CATEGORIES:APPOINTMENT,EDUCATION"),
        );

        assert_eq!(
            CategoriesProperty {
                params: CategoriesPropertyParams {
                    language: Some(String::from("en-US")),
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                categories: List::from(vec![Text(String::from("EDUCATION"))]),
            }.render_ical(),
            String::from("CATEGORIES;TEST=VALUE;X-TEST=X_VALUE;LANGUAGE=en-US:EDUCATION"),
        );
    }
}
