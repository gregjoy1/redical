use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{x_name, iana_token};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// ROLE = ("CHAIR"             ; Indicates chair of the
//                             ; calendar entity
//       / "REQ-PARTICIPANT"   ; Indicates a participant whose
//                             ; participation is required
//       / "OPT-PARTICIPANT"   ; Indicates a participant whose
//                             ; participation is optional
//       / "NON-PARTICIPANT"   ; Indicates a participant who
//                             ; is copied for information
//                             ; purposes only
//       / x-name              ; Experimental role
//       / iana-token)         ; Other IANA role
//     ; Default is REQ-PARTICIPANT
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Roletype {
    Chair,
    ReqParticipant,
    OptParticipant,
    NonParticipant,
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl ICalendarEntity for Roletype {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ROLE",
            alt((
                map(tag("CHAIR"), |_| Roletype::Chair),
                map(tag("REQ-PARTICIPANT"), |_| Roletype::ReqParticipant),
                map(tag("OPT-PARTICIPANT"), |_| Roletype::OptParticipant),
                map(tag("NON-PARTICIPANT"), |_| Roletype::NonParticipant),
                map(x_name, |value| Roletype::XName(value.to_string())),
                map(iana_token, |value| Roletype::IanaToken(value.to_string())),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
           Self::Chair => String::from("CHAIR"),
           Self::ReqParticipant => String::from("REQ-PARTICIPANT"),
           Self::OptParticipant => String::from("OPT-PARTICIPANT"),
           Self::NonParticipant => String::from("NON-PARTICIPANT"),
           Self::XName(name) => name.to_owned(),
           Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(Roletype);

// Participation Role
//
// Parameter Name:  ROLE
//
// Purpose:  To specify the participation role for the calendar user
//    specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     roleparam  = "ROLE" "="
//                 ("CHAIR"             ; Indicates chair of the
//                                      ; calendar entity
//                / "REQ-PARTICIPANT"   ; Indicates a participant whose
//                                      ; participation is required
//                / "OPT-PARTICIPANT"   ; Indicates a participant whose
//                                      ; participation is optional
//                / "NON-PARTICIPANT"   ; Indicates a participant who
//                                      ; is copied for information
//                                      ; purposes only
//                / x-name              ; Experimental role
//                / iana-token)         ; Other IANA role
//     ; Default is REQ-PARTICIPANT
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RoletypeParam(Roletype);

impl ICalendarEntity for RoletypeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ROLEPARAM",
            map(
                pair(
                    tag("ROLETYPE"),
                    preceded(tag("="), cut(Roletype::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ROLETYPE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(RoletypeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=CHAIR TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::Chair),
            ),
        );

        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=REQ-PARTICIPANT TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::ReqParticipant),
            ),
        );

        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=OPT-PARTICIPANT TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::OptParticipant),
            ),
        );

        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=NON-PARTICIPANT TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::NonParticipant),
            ),
        );

        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=X-TEST-NAME TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::XName(String::from("X-TEST-NAME"))),
            ),
        );

        assert_parser_output!(
            RoletypeParam::parse_ical(r#"ROLETYPE=TEST-IANA-NAME TESTING"#.into()),
            (
                " TESTING",
                RoletypeParam(Roletype::IanaToken(String::from("TEST-IANA-NAME"))),
            ),
        );

        assert!(RoletypeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RoletypeParam(Roletype::Chair).render_ical(),
            String::from("ROLETYPE=CHAIR"),
        );

        assert_eq!(
            RoletypeParam(Roletype::ReqParticipant).render_ical(),
            String::from("ROLETYPE=REQ-PARTICIPANT"),
        );

        assert_eq!(
            RoletypeParam(Roletype::OptParticipant).render_ical(),
            String::from("ROLETYPE=OPT-PARTICIPANT"),
        );

        assert_eq!(
            RoletypeParam(Roletype::NonParticipant).render_ical(),
            String::from("ROLETYPE=NON-PARTICIPANT"),
        );

        assert_eq!(
            RoletypeParam(Roletype::XName(String::from("X-TEST-NAME"))).render_ical(),
            String::from("ROLETYPE=X-TEST-NAME"),
        );

        assert_eq!(
            RoletypeParam(Roletype::IanaToken(String::from("TEST-IANA-NAME"))).render_ical(),
            String::from("ROLETYPE=TEST-IANA-NAME"),
        );
    }
}
