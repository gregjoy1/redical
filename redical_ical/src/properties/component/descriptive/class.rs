use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon, x_name, iana_token};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    ClassParams,
    ClassParam,
    "CLASSPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

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

    fn render_ical(&self) -> String {
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
pub struct Class {
    pub params: ClassParams,
    pub value: ClassValue,
}

impl ICalendarEntity for Class {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CLASS",
            preceded(
                tag("CLASS"),
                cut(
                    map(
                        pair(
                            opt(ClassParams::parse_ical),
                            preceded(colon, ClassValue::parse_ical),
                        ),
                        |(params, value)| {
                            Class {
                                params: params.unwrap_or(ClassParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CLASS{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Class);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Class::parse_ical("CLASS:PRIVATE".into()),
            (
                "",
                Class {
                    params: ClassParams::default(),
                    value: ClassValue::Private,
                },
            ),
        );

        assert_parser_output!(
            Class::parse_ical("CLASS:X-HIDDEN".into()),
            (
                "",
                Class {
                    params: ClassParams::default(),
                    value: ClassValue::XName(String::from("X-HIDDEN")),
                },
            ),
        );

        assert_parser_output!(
            Class::parse_ical("CLASS;X-TEST=X_VALUE;TEST=VALUE:IANA-TOKEN".into()),
            (
                "",
                Class {
                    params: ClassParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: ClassValue::IanaToken(String::from("IANA-TOKEN")),
                },
            ),
        );

        assert!(Class::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Class {
                params: ClassParams::default(),
                value: ClassValue::Private,
            }.render_ical(),
            String::from("CLASS:PRIVATE"),
        );

        assert_eq!(
            Class {
                params: ClassParams::default(),
                value: ClassValue::XName(String::from("X-HIDDEN")),
            }.render_ical(),
            String::from("CLASS:X-HIDDEN"),
        );

        assert_eq!(
            Class {
                params: ClassParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: ClassValue::IanaToken(String::from("IANA-TOKEN")),
            }.render_ical(),
            String::from("CLASS;X-TEST=X_VALUE;TEST=VALUE:IANA-TOKEN"),
        );
    }
}
