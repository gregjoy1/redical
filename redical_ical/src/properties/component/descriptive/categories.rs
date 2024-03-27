use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon, List};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    CatParams,
    CatParam,
    "CATPARAM",
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

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
pub struct Categories {
    pub params: CatParams,
    pub value: List<Text>,
}

impl ICalendarEntity for Categories {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CATEGORIES",
            preceded(
                tag("CATEGORIES"),
                cut(
                    map(
                        pair(
                            opt(CatParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, value)| {
                            Categories {
                                params: params.unwrap_or(CatParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CATEGORIES{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Categories);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::language::Language;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Categories::parse_ical("CATEGORIES:APPOINTMENT,EDUCATION".into()),
            (
                "",
                Categories {
                    params: CatParams::default(),
                    value: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                },
            ),
        );

        assert_parser_output!(
            Categories::parse_ical("CATEGORIES;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US:EDUCATION".into()),
            (
                "",
                Categories {
                    params: CatParams {
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: List::from(vec![Text(String::from("EDUCATION"))]),
                },
            ),
        );

        assert!(Categories::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Categories {
                params: CatParams::default(),
                value: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
            }.render_ical(),
            String::from("CATEGORIES:APPOINTMENT,EDUCATION"),
        );

        assert_eq!(
            Categories {
                params: CatParams {
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: List::from(vec![Text(String::from("EDUCATION"))]),
            }.render_ical(),
            String::from("CATEGORIES;LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:EDUCATION"),
        );
    }
}
