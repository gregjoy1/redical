use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::cal_address::CalAddress;
use crate::property_parameters::{
    cn::CnParam,
    dir::DirParam,
    sent_by::SentByParam,
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    OrgParams,
    OrgParam,
    "ORGPARAM",
    (Cn, CnParam, cn, Option<CnParam>),
    (Dir, DirParam, dir, Option<DirParam>),
    (SentBy, SentByParam, sent_by, Option<SentByParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Organizer
//
// Property Name:  ORGANIZER
//
// Purpose:  This property defines the organizer for a calendar
//    component.
//
// Value Type:  CAL-ADDRESS
//
// Property Parameters:  IANA, non-standard, language, common name,
//    directory entry reference, and sent-by property parameters can be
//    specified on this property.
//
// Conformance:  This property MUST be specified in an iCalendar object
//    that specifies a group-scheduled calendar entity.  This property
//    MUST be specified in an iCalendar object that specifies the
//    publication of a calendar user's busy time.  This property MUST
//    NOT be specified in an iCalendar object that specifies only a time
//    zone definition or that defines calendar components that are not
//    group-scheduled components, but are components only on a single
//    user's calendar.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     organizer  = "ORGANIZER" orgparam ":"
//                  cal-address CRLF
//
//     orgparam   = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" cnparam) / (";" dirparam) / (";" sentbyparam) /
//                (";" languageparam) /
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
//     ORGANIZER;CN=John Smith:mailto:jsmith@example.com
//
//    The following is an example of this property with a pointer to the
//    directory information associated with the organizer:
//
//     ORGANIZER;CN=JohnSmith;DIR="ldap://example.com:6666/o=DC%20Ass
//      ociates,c=US???(cn=John%20Smith)":mailto:jsmith@example.com
//
//    The following is an example of this property used by another
//    calendar user who is acting on behalf of the organizer, with
//    responses intended to be sent back to the organizer, not the other
//    calendar user:
//
//     ORGANIZER;SENT-BY="mailto:jane_doe@example.com":
//      mailto:jsmith@example.com
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Organizer {
    pub params: OrgParams,
    pub value: CalAddress,
}

impl ICalendarEntity for Organizer {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ORGANIZER",
            preceded(
                tag("ORGANIZER"),
                cut(
                    map(
                        pair(
                            opt(OrgParams::parse_ical),
                            preceded(colon, CalAddress::parse_ical),
                        ),
                        |(params, value)| {
                            Organizer {
                                params: params.unwrap_or(OrgParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ORGANIZER{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Organizer);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::uri::Uri;

    use crate::grammar::Quoted;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Organizer::parse_ical(
                r#"ORGANIZER;CN=John Smith:mailto:jsmith@example.com"#.into()
            ),
            (
                "",
                Organizer {
                    params: OrgParams {
                        cn: Some(CnParam(String::from("John Smith"))),
                        dir: None,
                        sent_by: None,
                        language: None,
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: CalAddress(Uri(String::from(r#"mailto:jsmith@example.com"#))),
                },
            ),
        );

        assert_parser_output!(
            Organizer::parse_ical(
                r#"ORGANIZER;X-TEST=X_VALUE;TEST=VALUE;SENT-BY="mailto:jane_doe@example.com":mailto:jsmith@example.com"#.into()
            ),
            (
                "",
                Organizer {
                    params: OrgParams {
                        cn: None,
                        dir: None,
                        sent_by: Some(SentByParam(Quoted(CalAddress(Uri(String::from("mailto:jane_doe@example.com")))))),
                        language: None,
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: CalAddress(Uri(String::from(r#"mailto:jsmith@example.com"#))),
                },
            ),
        );

        assert!(Organizer::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Organizer {
                params: OrgParams {
                    cn: Some(CnParam(String::from("John Smith"))),
                    dir: None,
                    sent_by: None,
                    language: None,
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: CalAddress(Uri(String::from(r#"mailto:jsmith@example.com"#))),
            }.render_ical(),
            String::from(r#"ORGANIZER;CN=John Smith:mailto:jsmith@example.com"#),
        );

        assert_eq!(
            Organizer {
                params: OrgParams {
                    cn: None,
                    dir: None,
                    sent_by: Some(SentByParam(Quoted(CalAddress(Uri(String::from("mailto:jane_doe@example.com")))))),
                    language: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: CalAddress(Uri(String::from(r#"mailto:jsmith@example.com"#))),
            }.render_ical(),
            String::from(r#"ORGANIZER;SENT-BY="mailto:jane_doe@example.com";X-TEST=X_VALUE;TEST=VALUE:mailto:jsmith@example.com"#),
        );
    }
}
