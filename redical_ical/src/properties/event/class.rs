use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

// classvalue = "PUBLIC" / "PRIVATE" / "CONFIDENTIAL" / iana-token
//            / x-name
// ;Default is PUBLIC
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ClassValue {
    Public,
    Private,
    Confidential,
    XName(String),
    IanaToken(String),
}

impl ICalendarEntity for ClassValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CLASSVALUE",
            alt((
                map(tag("PUBLUC"), |_| ClassValue::Public),
                map(tag("PRIVATE"), |_| ClassValue::Private),
                map(tag("CONFIDENTIAL"), |_| ClassValue::Confidential),
                map(x_name, |value| ClassValue::XName(value.to_string())),
                map(iana_token, |value| ClassValue::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::Public => String::from("PUBLIC"),
           Self::Private => String::from("PRIVATE"),
           Self::Confidential => String::from("CONFIDENTIAL"),
           Self::XName(name) => name.to_owned(),
           Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(ClassValue);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ClassPropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for ClassPropertyParams {
    define_property_params_ical_parser!(
        ClassPropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut ClassPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&ClassPropertyParams> for ContentLineParams {
    fn from(class_params: &ClassPropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in class_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<ClassPropertyParams> for ContentLineParams {
    fn from(class_params: ClassPropertyParams) -> Self {
        ContentLineParams::from(&class_params)
    }
}

// Classification
//
// Property Name:  CLASS
//
// Purpose:  This property defines the access classification for a
//    calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  The property can be specified once in a "VEVENT",
//    "VTODO", or "VJOURNAL" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     class      = "CLASS" classparam ":" classvalue CRLF
//
//     classparam = *(";" other-param)
//
//     classvalue = "PUBLIC" / "PRIVATE" / "CONFIDENTIAL" / iana-token
//                / x-name
//     ;Default is PUBLIC
//
// Example:  The following is an example of this property:
//
//     CLASS:PUBLIC
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ClassProperty {
    pub params: ClassPropertyParams,
    pub value: ClassValue,
}

impl ICalendarEntity for ClassProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CLASS",
            preceded(
                tag("CLASS"),
                cut(
                    map(
                        pair(
                            opt(ClassPropertyParams::parse_ical),
                            preceded(colon, ClassValue::parse_ical),
                        ),
                        |(params, value)| {
                            ClassProperty {
                                params: params.unwrap_or(ClassPropertyParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        ContentLine::from(self).render_ical()
    }
}

impl From<&ClassProperty> for ContentLine {
    fn from(class_property: &ClassProperty) -> Self {
        ContentLine::from((
            "CLASS",
            (
                ContentLineParams::from(&class_property.params),
                class_property.value.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for ClassProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(ClassProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ClassProperty::parse_ical("CLASS:PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ClassProperty {
                    params: ClassPropertyParams::default(),
                    value: ClassValue::Private,
                },
            ),
        );

        assert_parser_output!(
            ClassProperty::parse_ical("CLASS:X-HIDDEN".into()),
            (
                "",
                ClassProperty {
                    params: ClassPropertyParams::default(),
                    value: ClassValue::XName(String::from("X-HIDDEN")),
                },
            ),
        );

        assert_parser_output!(
            ClassProperty::parse_ical("CLASS;X-TEST=X_VALUE;TEST=VALUE:IANA-TOKEN".into()),
            (
                "",
                ClassProperty {
                    params: ClassPropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    value: ClassValue::IanaToken(String::from("IANA-TOKEN")),
                },
            ),
        );

        assert!(ClassProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ClassProperty {
                params: ClassPropertyParams::default(),
                value: ClassValue::Private,
            }.render_ical(),
            String::from("CLASS:PRIVATE"),
        );

        assert_eq!(
            ClassProperty {
                params: ClassPropertyParams::default(),
                value: ClassValue::XName(String::from("X-HIDDEN")),
            }.render_ical(),
            String::from("CLASS:X-HIDDEN"),
        );

        assert_eq!(
            ClassProperty {
                params: ClassPropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                value: ClassValue::IanaToken(String::from("IANA-TOKEN")),
            }.render_ical(),
            String::from("CLASS;TEST=VALUE;X-TEST=X_VALUE:IANA-TOKEN"),
        );
    }
}
