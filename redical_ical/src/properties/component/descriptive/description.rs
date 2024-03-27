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
    DescParams,
    DescParam,
    "DESCPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Description
//
// Property Name:  DESCRIPTION
//
// Purpose:  This property provides a more complete description of the
//    calendar component than that provided by the "SUMMARY" property.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  The property can be specified in the "VEVENT", "VTODO",
//    "VJOURNAL", or "VALARM" calendar components.  The property can be
//    specified multiple times only within a "VJOURNAL" calendar
//    component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     description = "DESCRIPTION" descparam ":" text CRLF
//
//     descparam   = *(
//                 ;
//                 ; The following are OPTIONAL,
//                 ; but MUST NOT occur more than once.
//                 ;
//                 (";" altrepparam) / (";" languageparam) /
//                 ;
//                 ; The following is OPTIONAL,
//                 ; and MAY occur more than once.
//                 ;
//                 (";" other-param)
//                 ;
//                 )
//
// Example:  The following is an example of this property with formatted
//    line breaks in the property value:
//
//     DESCRIPTION:Meeting to provide technical review for "Phoenix"
//       design.\nHappy Face Conference Room. Phoenix design team
//       MUST attend this meeting.\nRSVP to team leader.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Description {
    pub params: DescParams,
    pub value: Text,
}

impl ICalendarEntity for Description {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DESCRIPTION",
            preceded(
                tag("DESCRIPTION"),
                cut(
                    map(
                        pair(
                            opt(DescParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Description {
                                params: params.unwrap_or(DescParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DESCRIPTION{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Description);

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
            Description::parse_ical(
                r#"DESCRIPTION:Meeting to provide technical review for "Phoenix" design.\nHappy Face Conference Room. Phoenix design team MUST attend this meeting.\nRSVP to team leader."#.into()
            ),
            (
                "",
                Description {
                    params: DescParams::default(),
                    value: Text(String::from(r#"Meeting to provide technical review for "Phoenix" design.\nHappy Face Conference Room. Phoenix design team MUST attend this meeting.\nRSVP to team leader."#)),
                },
            ),
        );

        assert_parser_output!(
            Description::parse_ical(r#"DESCRIPTION;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US;ALTREP="CID:part3.msg.970415T083000@example.com":Some description text\, by Wilson\n in Maine"#.into()),
            (
                "",
                Description {
                    params: DescParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from(r#"Some description text\, by Wilson\n in Maine"#)),
                },
            ),
        );

        assert!(Description::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Description {
                params: DescParams::default(),
                value: Text(String::from(r#"Meeting to provide technical review for "Phoenix" design.\nHappy Face Conference Room. Phoenix design team MUST attend this meeting.\nRSVP to team leader."#)),
            }.render_ical(),
            String::from(
                r#"DESCRIPTION:Meeting to provide technical review for "Phoenix" design.\nHappy Face Conference Room. Phoenix design team MUST attend this meeting.\nRSVP to team leader."#
            ),
        );

        assert_eq!(
            Description {
                params: DescParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from(r#"Some description text\, by Wilson\n in Maine"#)),
            }.render_ical(),
            String::from(r#"DESCRIPTION;ALTREP="CID:part3.msg.970415T083000@example.com";LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:Some description text\, by Wilson\n in Maine"#),
        );
    }
}
