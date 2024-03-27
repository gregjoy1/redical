use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::bytes::complete::{tag, take_while1};
use nom::multi::fold_many0;
use nom::combinator::{opt, map, cut};

use crate::grammar::{is_safe_char, is_wsp_char, colon, semicolon};
use crate::property_parameters::{
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::property_value_data_types::text::Text;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    TznParams,
    TznParam,
    "TZNPARAM",
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Zone Name
//
// Property Name:  TZNAME
//
// Purpose:  This property specifies the customary designation for a
//    time zone description.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, and language property
//    parameters can be specified on this property.
//
// Conformance:  This property can be specified in "STANDARD" and
//    "DAYLIGHT" sub-components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     tzname     = "TZNAME" tznparam ":" text CRLF
//
//     tznparam   = *(
//                ;
//                ; The following is OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" languageparam) /
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
//     TZNAME:EST
//
//     TZNAME;LANGUAGE=fr-CA:HNE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzname {
    pub params: TznParams,
    pub value: Text,
}

impl ICalendarEntity for Tzname {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZNAME",
            preceded(
                tag("TZNAME"),
                cut(
                    map(
                        pair(
                            opt(TznParams::parse_ical),
                            preceded(
                                colon,
                                // Small hack that allows paramtext chars except whitespace.
                                take_while1(|input: char| {
                                    is_safe_char(input) && !is_wsp_char(input)
                                }),
                            )
                        ),
                        |(params, value)| {
                            Tzname {
                                params: params.unwrap_or(TznParams::default()),
                                value: Text(value.to_string()),
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZNAME{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Tzname);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::language::Language;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzname::parse_ical("TZNAME:EST TESTING".into()),
            (
                " TESTING",
                Tzname {
                    params: TznParams::default(),
                    value: Text(String::from("EST")),
                },
            )
        );

        assert_parser_output!(
            Tzname::parse_ical("TZNAME;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=fr-CA:HNE TESTING".into()),
            (
                " TESTING",
                Tzname {
                    params: TznParams {
                        language: Some(LanguageParam(Language(String::from("fr-CA")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("HNE")),
                },
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzname {
                params: TznParams::default(),
                value: Text(String::from("EST")),
            }.render_ical(),
            String::from("TZNAME:EST"),
        );

        assert_eq!(
            Tzname {
                params: TznParams {
                    language: Some(LanguageParam(Language(String::from("fr-CA")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("HNE")),
            }.render_ical(),
            String::from("TZNAME;LANGUAGE=fr-CA;X-TEST=X_VALUE;TEST=VALUE:HNE"),
        );
    }
}
