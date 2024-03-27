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
    LocParams,
    LocParam,
    "LOCPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Location
//
// Property Name:  LOCATION
//
// Purpose:  This property defines the intended venue for the activity
//    defined by a calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  This property can be specified in "VEVENT" or "VTODO"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     location   = "LOCATION"  locparam ":" text CRLF
//
//     locparam   = *(
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
// Example:  The following are some examples of this property:
//
//     LOCATION:Conference Room - F123\, Bldg. 002
//
//     LOCATION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf":
//      Conference Room - F123\, Bldg. 002
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Location {
    pub params: LocParams,
    pub value: Text,
}

impl ICalendarEntity for Location {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "LOCATION",
            preceded(
                tag("LOCATION"),
                cut(
                    map(
                        pair(
                            opt(LocParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Location {
                                params: params.unwrap_or(LocParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("LOCATION{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Location);

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
            Location::parse_ical(
                r#"LOCATION:Conference Room - F123\, Bldg. 002"#.into()
            ),
            (
                "",
                Location {
                    params: LocParams::default(),
                    value: Text(String::from(r#"Conference Room - F123\, Bldg. 002"#)),
                },
            ),
        );

        assert_parser_output!(
            Location::parse_ical(r#"LOCATION;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf":Conference Room - F123\, Bldg. 002"#.into()),
            (
                "",
                Location {
                    params: LocParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("http://xyzcorp.com/conf-rooms/f123.vcf"))))),
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from(r#"Conference Room - F123\, Bldg. 002"#)),
                },
            ),
        );

        assert!(Location::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Location {
                params: LocParams::default(),
                value: Text(String::from(r#"Conference Room - F123\, Bldg. 002"#)),
            }.render_ical(),
            String::from(
                r#"LOCATION:Conference Room - F123\, Bldg. 002"#
            ),
        );

        assert_eq!(
            Location {
                params: LocParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("http://xyzcorp.com/conf-rooms/f123.vcf"))))),
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from(r#"Conference Room - F123\, Bldg. 002"#)),
            }.render_ical(),
            String::from(r#"LOCATION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:Conference Room - F123\, Bldg. 002"#),
        );
    }
}
