use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    altrep::AltrepParam,
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    SummParams,
    SummParam,
    "SUMMPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Summary
//
// Property Name:  SUMMARY
//
// Purpose:  This property defines a short summary or subject for the
//    calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  The property can be specified in "VEVENT", "VTODO",
//    "VJOURNAL", or "VALARM" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     summary    = "SUMMARY" summparam ":" text CRLF
//
//     summparam  = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" altrepparam) / (";" languageparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
// Example:  The following is an example of this property:
//
//     SUMMARY:Department Party
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Summary {
    pub params: SummParams,
    pub value: Text,
}

impl ICalendarEntity for Summary {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "SUMMARY",
            preceded(
                tag("SUMMARY"),
                cut(
                    map(
                        pair(
                            opt(SummParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Summary {
                                params: params.unwrap_or(SummParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("SUMMARY{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Summary);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::language::Language;
    use crate::property_value_data_types::uri::Uri;

    use crate::grammar::Quoted;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Summary::parse_ical(
                r#"SUMMARY:Department Party"#.into()
            ),
            (
                "",
                Summary {
                    params: SummParams::default(),
                    value: Text(String::from(r#"Department Party"#)),
                },
            ),
        );

        assert_parser_output!(
            Summary::parse_ical(r#"SUMMARY;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US;ALTREP="CID:part3.msg.970415T083000@example.com":Some summary text\, by Wilson\n in Maine"#.into()),
            (
                "",
                Summary {
                    params: SummParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from(r#"Some summary text\, by Wilson\n in Maine"#)),
                },
            ),
        );

        assert!(Summary::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Summary {
                params: SummParams::default(),
                value: Text(String::from(r#"Department Party"#)),
            }.render_ical(),
            String::from(
                r#"SUMMARY:Department Party"#
            ),
        );

        assert_eq!(
            Summary {
                params: SummParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from(r#"Some summary text\, by Wilson\n in Maine"#)),
            }.render_ical(),
            String::from(r#"SUMMARY;ALTREP="CID:part3.msg.970415T083000@example.com";LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:Some summary text\, by Wilson\n in Maine"#),
        );
    }
}
