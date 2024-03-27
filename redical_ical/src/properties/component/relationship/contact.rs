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
    ContParams,
    ContParam,
    "CONTPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Contact
//
// Property Name:  CONTACT
//
// Purpose:  This property is used to represent contact information or
//    alternately a reference to contact information associated with the
//    calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  This property can be specified in a "VEVENT", "VTODO",
//    "VJOURNAL", or "VFREEBUSY" calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     contact    = "CONTACT" contparam ":" text CRLF
//
//     contparam  = *(
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
// Example:  The following is an example of this property referencing
//    textual contact information:
//
//     CONTACT:Jim Dolittle\, ABC Industries\, +1-919-555-1234
//
//    The following is an example of this property with an alternate
//    representation of an LDAP URI to a directory entry containing the
//    contact information:
//
//     CONTACT;ALTREP="ldap://example.com:6666/o=ABC%20Industries\,
//      c=US???(cn=Jim%20Dolittle)":Jim Dolittle\, ABC Industries\,
//      +1-919-555-1234
//
//    The following is an example of this property with an alternate
//    representation of a MIME body part containing the contact
//    information, such as a vCard [RFC2426] embedded in a text/
//    directory media type [RFC2425]:
//
//     CONTACT;ALTREP="CID:part3.msg970930T083000SILVER@example.com":
//      Jim Dolittle\, ABC Industries\, +1-919-555-1234
//
//    The following is an example of this property referencing a network
//    resource, such as a vCard [RFC2426] object containing the contact
//    information:
//
//     CONTACT;ALTREP="http://example.com/pdi/jdoe.vcf":Jim
//       Dolittle\, ABC Industries\, +1-919-555-1234
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Contact {
    pub params: ContParams,
    pub value: Text,
}

impl ICalendarEntity for Contact {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CONTACT",
            preceded(
                tag("CONTACT"),
                cut(
                    map(
                        pair(
                            opt(ContParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Contact {
                                params: params.unwrap_or(ContParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CONTACT{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Contact);

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
            Contact::parse_ical(
                r#"CONTACT:Jim Dolittle\, ABC Industries\, +1-919-555-1234"#.into()
            ),
            (
                "",
                Contact {
                    params: ContParams::default(),
                    value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
                },
            ),
        );

        assert_parser_output!(
            Contact::parse_ical(
                r#"CONTACT;ALTREP="CID:part3.msg970930T083000SILVER@example.com":Jim Dolittle\, ABC Industries\, +1-919-555-1234"#.into()
            ),
            (
                "",
                Contact {
                    params: ContParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from(r#"CID:part3.msg970930T083000SILVER@example.com"#))))),
                        language: None,
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
                },
            ),
        );

        assert_parser_output!(
            Contact::parse_ical(r#"CONTACT;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US;ALTREP="http://example.com/pdi/jdoe.vcf":Jim Dolittle\, ABC Industries\, +1-919-555-1234"#.into()),
            (
                "",
                Contact {
                    params: ContParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("http://example.com/pdi/jdoe.vcf"))))),
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
                },
            ),
        );

        assert!(Contact::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Contact {
                params: ContParams::default(),
                value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
            }.render_ical(),
            String::from(r#"CONTACT:Jim Dolittle\, ABC Industries\, +1-919-555-1234"#),
        );

        assert_eq!(
            Contact {
                params: ContParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from(r#"CID:part3.msg970930T083000SILVER@example.com"#))))),
                    language: None,
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
            }.render_ical(),
            String::from(r#"CONTACT;ALTREP="CID:part3.msg970930T083000SILVER@example.com":Jim Dolittle\, ABC Industries\, +1-919-555-1234"#),
        );

        assert_eq!(
            Contact {
                params: ContParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("http://example.com/pdi/jdoe.vcf"))))),
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from(r#"Jim Dolittle\, ABC Industries\, +1-919-555-1234"#)),
            }.render_ical(),
            String::from(r#"CONTACT;ALTREP="http://example.com/pdi/jdoe.vcf";LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:Jim Dolittle\, ABC Industries\, +1-919-555-1234"#),
        );
    }
}
