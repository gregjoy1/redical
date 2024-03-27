use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon, List};
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
    ResrcParams,
    ResrcParam,
    "RESRCPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Resources
//
// Property Name:  RESOURCES
//
// Purpose:  This property defines the equipment or resources
//    anticipated for an activity specified by a calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  This property can be specified once in "VEVENT" or
//    "VTODO" calendar component.
//
// Description:  The property value is an arbitrary text.  More than one
//    resource can be specified as a COMMA-separated list of resources.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     resources  = "RESOURCES" resrcparam ":" text *("," text) CRLF
//
//     resrcparam = *(
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
//     RESOURCES:EASEL,PROJECTOR,VCR
//
//     RESOURCES;LANGUAGE=fr:Nettoyeur haute pression
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Resources {
    pub params: ResrcParams,
    pub value: List<Text>,
}

impl ICalendarEntity for Resources {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RESOURCES",
            preceded(
                tag("RESOURCES"),
                cut(
                    map(
                        pair(
                            opt(ResrcParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, value)| {
                            Resources {
                                params: params.unwrap_or(ResrcParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RESOURCES{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Resources);

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
            Resources::parse_ical(
                "RESOURCES:EASEL,PROJECTOR,VCR".into()
            ),
            (
                "",
                Resources {
                    params: ResrcParams::default(),
                    value: List::from(vec![Text(String::from("EASEL")), Text(String::from("PROJECTOR")), Text(String::from("VCR"))]),
                },
            ),
        );

        assert_parser_output!(
            Resources::parse_ical(r#"RESOURCES;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=fr;ALTREP="CID:part3.msg.970415T083000@example.com":Nettoyeur haute pression"#.into()),
            (
                "",
                Resources {
                    params: ResrcParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                        language: Some(LanguageParam(Language(String::from("fr")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: List::from(vec![Text(String::from("Nettoyeur haute pression"))]),
                },
            ),
        );

        assert!(Resources::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Resources {
                params: ResrcParams::default(),
                value: List::from(vec![Text(String::from("EASEL")), Text(String::from("PROJECTOR")), Text(String::from("VCR"))]),
            }.render_ical(),
            String::from("RESOURCES:EASEL,PROJECTOR,VCR"),
        );

        assert_eq!(
            Resources {
                params: ResrcParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                    language: Some(LanguageParam(Language(String::from("fr")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: List::from(vec![Text(String::from("Nettoyeur haute pression"))]),
            }.render_ical(),
            String::from(r#"RESOURCES;ALTREP="CID:part3.msg.970415T083000@example.com";LANGUAGE=fr;X-TEST=X_VALUE;TEST=VALUE:Nettoyeur haute pression"#),
        );
    }
}
