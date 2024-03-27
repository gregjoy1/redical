use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::bytes::complete::tag;
use nom::multi::fold_many0;
use nom::combinator::{opt, map, cut};

use crate::grammar::{colon, semicolon};
use crate::property_parameters::{
    cutype::CutypeParam,
    member::MemberParam,
    role::RoletypeParam,
    partstat::{PartstatParam, PartstatEvent},
    rsvp::RsvpParam,
    delegated_to::DeltoParam,
    delegated_from::DelfromParam,
    sent_by::SentByParam,
    cn::CnParam,
    dir::DirParam,
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::property_value_data_types::uri::Uri;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

// TODO: Accomodate PartstatJournal/PartstatTodo
pub type EventPartstatParam = PartstatParam<PartstatEvent>;

define_property_params!(
    AttParams,
    AttParam,
    "ATTPARAM",
    (Cutype, CutypeParam, cutype, Option<CutypeParam>),
    (Member, MemberParam, member, Option<MemberParam>),
    (Roletype, RoletypeParam, role, Option<RoletypeParam>),
    (Partstat, EventPartstatParam, partstat, Option<EventPartstatParam>),
    (Rsvp, RsvpParam, rsvp, Option<RsvpParam>),
    (Delto, DeltoParam, delto, Option<DeltoParam>),
    (Delfrom, DelfromParam, delfrom, Option<DelfromParam>),
    (SentBy, SentByParam, sent_by, Option<SentByParam>),
    (Cn, CnParam, cn, Option<CnParam>),
    (Dir, DirParam, dir, Option<DirParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Attendee
//
// Property Name:  ATTENDEE
//
// Purpose:  This property defines an "Attendee" within a calendar
//    component.
//
// Value Type:  CAL-ADDRESS
//
// Property Parameters:  IANA, non-standard, language, calendar user
//    type, group or list membership, participation role, participation
//    status, RSVP expectation, delegatee, delegator, sent by, common
//    name, or directory entry reference property parameters can be
//    specified on this property.
//
// Conformance:  This property MUST be specified in an iCalendar object
//    that specifies a group-scheduled calendar entity.  This property
//    MUST NOT be specified in an iCalendar object when publishing the
//    calendar information (e.g., NOT in an iCalendar object that
//    specifies the publication of a calendar user's busy time, event,
//    to-do, or journal).  This property is not specified in an
//    iCalendar object that specifies only a time zone definition or
//    that defines calendar components that are not group-scheduled
//    components, but are components only on a single user's calendar.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     attendee   = "ATTENDEE" attparam ":" cal-address CRLF
//
//     attparam   = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" cutypeparam) / (";" memberparam) /
//                (";" roleparam) / (";" partstatparam) /
//                (";" rsvpparam) / (";" deltoparam) /
//                (";" delfromparam) / (";" sentbyparam) /
//                (";" cnparam) / (";" dirparam) /
//                (";" languageparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
// Example:  The following are examples of this property's use for a
//    to-do:
//
//     ATTENDEE;MEMBER="mailto:DEV-GROUP@example.com":
//      mailto:joecool@example.com
//     ATTENDEE;DELEGATED-FROM="mailto:immud@example.com":
//      mailto:ildoit@example.com
//
//    The following is an example of this property used for specifying
//    multiple attendees to an event:
//
//     ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=TENTATIVE;CN=Henry
//      Cabot:mailto:hcabot@example.com
//     ATTENDEE;ROLE=REQ-PARTICIPANT;DELEGATED-FROM="mailto:bob@
//      example.com";PARTSTAT=ACCEPTED;CN=Jane Doe:mailto:jdoe@
//      example.com
//
//    The following is an example of this property with a URI to the
//    directory information associated with the attendee:
//
//     ATTENDEE;CN=John Smith;DIR="ldap://example.com:6666/o=ABC%
//      20Industries,c=US???(cn=Jim%20Dolittle)":mailto:jimdo@
//      example.com
//
//    The following is an example of this property with "delegatee" and
//    "delegator" information for an event:
//
//     ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=TENTATIVE;DELEGATED-FROM=
//      "mailto:iamboss@example.com";CN=Henry Cabot:mailto:hcabot@
//      example.com
//     ATTENDEE;ROLE=NON-PARTICIPANT;PARTSTAT=DELEGATED;DELEGATED-TO=
//      "mailto:hcabot@example.com";CN=The Big Cheese:mailto:iamboss
//      @example.com
//     ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED;CN=Jane Doe
//      :mailto:jdoe@example.com
//
// Example:  The following is an example of this property's use when
//    another calendar user is acting on behalf of the "Attendee":
//
//     ATTENDEE;SENT-BY=mailto:jan_doe@example.com;CN=John Smith:
//      mailto:jsmith@example.com
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attendee {
    pub params: AttParams,
    pub value: Uri,
}

impl ICalendarEntity for Attendee {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ATTENDEE",
            preceded(
                tag("ATTENDEE"),
                cut(
                    map(
                        pair(
                            opt(AttParams::parse_ical),
                            preceded(
                                colon,
                                Uri::parse_ical,
                            )
                        ),
                        |(params, value)| {
                            Attendee {
                                params: params.unwrap_or(AttParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ATTENDEE{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Attendee);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::grammar::{List, Quoted};
    use crate::property_value_data_types::cal_address::CalAddress;
    use crate::property_value_data_types::uri::Uri;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Attendee::parse_ical(r#"ATTENDEE;MEMBER="mailto:DEV-GROUP@example.com":mailto:joecool@example.com TESTING"#.into()),
            (
                " TESTING",
                Attendee {
                    params: AttParams {
                        cutype: None,
                        member: Some(
                            MemberParam(
                                List::from(
                                    vec![
                                        Quoted(
                                            CalAddress(
                                                Uri(
                                                    String::from("mailto:DEV-GROUP@example.com")
                                                )
                                            )
                                        )
                                    ]
                                )
                            )
                        ),
                        role: None,
                        partstat: None,
                        rsvp: None,
                        delto: None,
                        delfrom: None,
                        sent_by: None,
                        cn: None,
                        dir: None,
                        language: None,
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: Uri(String::from("mailto:joecool@example.com")),
                },
            )
        );

        assert_parser_output!(
            Attendee::parse_ical(r#"ATTENDEE;X-TEST=X_VALUE;TEST=VALUE;DELEGATED-FROM="mailto:immud@example.com":mailto:ildoit@example.com TESTING"#.into()),
            (
                " TESTING",
                Attendee {
                    params: AttParams {
                        cutype: None,
                        member: None,
                        role: None,
                        partstat: None,
                        rsvp: None,
                        delto: None,
                        delfrom: Some(
                            DelfromParam(
                                List::from(
                                    vec![
                                        Quoted(
                                            CalAddress(
                                                Uri(
                                                    String::from("mailto:immud@example.com")
                                                )
                                            )
                                        )
                                    ]
                                )
                            )
                        ),
                        sent_by: None,
                        cn: None,
                        dir: None,
                        language: None,
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Uri(String::from("mailto:ildoit@example.com")),
                },
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Attendee {
                params: AttParams {
                    cutype: None,
                    member: Some(
                        MemberParam(
                            List::from(
                                vec![
                                    Quoted(
                                        CalAddress(
                                            Uri(
                                                String::from("mailto:DEV-GROUP@example.com")
                                            )
                                        )
                                    )
                                ]
                            )
                        )
                    ),
                    role: None,
                    partstat: None,
                    rsvp: None,
                    delto: None,
                    delfrom: None,
                    sent_by: None,
                    cn: None,
                    dir: None,
                    language: None,
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: Uri(String::from("mailto:joecool@example.com")),
            }.render_ical(),
            String::from(r#"ATTENDEE;MEMBER="mailto:DEV-GROUP@example.com":mailto:joecool@example.com"#),
        );

        assert_eq!(
            Attendee {
                params: AttParams {
                    cutype: None,
                    member: None,
                    role: None,
                    partstat: None,
                    rsvp: None,
                    delto: None,
                    delfrom: Some(
                        DelfromParam(
                            List::from(
                                vec![
                                    Quoted(
                                        CalAddress(
                                            Uri(
                                                String::from("mailto:immud@example.com")
                                            )
                                        )
                                    )
                                ]
                            )
                        )
                    ),
                    sent_by: None,
                    cn: None,
                    dir: None,
                    language: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Uri(String::from("mailto:ildoit@example.com")),
            }.render_ical(),
            String::from(r#"ATTENDEE;DELEGATED-FROM="mailto:immud@example.com";X-TEST=X_VALUE;TEST=VALUE:mailto:ildoit@example.com"#),
        );
    }
}
