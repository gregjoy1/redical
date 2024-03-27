use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{x_name, iana_token};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

pub trait Partstat {}

//     partstat-event   = ("NEEDS-ACTION"    ; Event needs action
//                      / "ACCEPTED"         ; Event accepted
//                      / "DECLINED"         ; Event declined
//                      / "TENTATIVE"        ; Event tentatively
//                                           ; accepted
//                      / "DELEGATED"        ; Event delegated
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PartstatEvent {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
    Delegated,
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl Partstat for PartstatEvent {}

impl ICalendarEntity for PartstatEvent {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("NEEDS-ACTION"), |_| PartstatEvent::NeedsAction),
            map(tag("ACCEPTED"), |_| PartstatEvent::Accepted),
            map(tag("DECLINED"), |_| PartstatEvent::Declined),
            map(tag("TENTATIVE"), |_| PartstatEvent::Tentative),
            map(tag("DELEGATED"), |_| PartstatEvent::Delegated),
            map(x_name, |value| PartstatEvent::XName(value.to_string())),
            map(iana_token, |value| PartstatEvent::IanaToken(value.to_string())),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::NeedsAction => String::from("NEEDS-ACTION"),
            Self::Accepted => String::from("ACCEPTED"),
            Self::Declined => String::from("DECLINED"),
            Self::Tentative => String::from("TENTATIVE"),
            Self::Delegated => String::from("DELEGATED"),
            Self::XName(name) => name.to_owned(),
            Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(PartstatEvent);

//     partstat-todo    = ("NEEDS-ACTION"    ; To-do needs action
//                      / "ACCEPTED"         ; To-do accepted
//                      / "DECLINED"         ; To-do declined
//                      / "TENTATIVE"        ; To-do tentatively
//                                           ; accepted
//                      / "DELEGATED"        ; To-do delegated
//                      / "COMPLETED"        ; To-do completed
//                                           ; COMPLETED property has
//                                           ; DATE-TIME completed
//                      / "IN-PROCESS"       ; To-do in process of
//                                           ; being completed
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PartstatTodo {
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
    Delegated,
    Completed,
    InProcess,
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl Partstat for PartstatTodo {}

impl ICalendarEntity for PartstatTodo {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("NEEDS-ACTION"), |_| PartstatTodo::NeedsAction),
            map(tag("ACCEPTED"), |_| PartstatTodo::Accepted),
            map(tag("DECLINED"), |_| PartstatTodo::Declined),
            map(tag("TENTATIVE"), |_| PartstatTodo::Tentative),
            map(tag("DELEGATED"), |_| PartstatTodo::Delegated),
            map(tag("COMPLETED"), |_| PartstatTodo::Completed),
            map(tag("IN-PROCESS"), |_| PartstatTodo::InProcess),
            map(x_name, |value| PartstatTodo::XName(value.to_string())),
            map(iana_token, |value| PartstatTodo::IanaToken(value.to_string())),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::NeedsAction => String::from("NEEDS-ACTION"),
            Self::Accepted => String::from("ACCEPTED"),
            Self::Declined => String::from("DECLINED"),
            Self::Tentative => String::from("TENTATIVE"),
            Self::Delegated => String::from("DELEGATED"),
            Self::Completed => String::from("COMPLETED"),
            Self::InProcess => String::from("IN-PROCESS"),
            Self::XName(name) => name.to_owned(),
            Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(PartstatTodo);

//     partstat-jour    = ("NEEDS-ACTION"    ; Journal needs action
//                      / "ACCEPTED"         ; Journal accepted
//                      / "DECLINED"         ; Journal declined
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PartstatJour {
    NeedsAction,
    Accepted,
    Declined,
    XName(String),     // Experimental type
    IanaToken(String), // Other IANA-registered
}

impl Partstat for PartstatJour {}

impl ICalendarEntity for PartstatJour {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("NEEDS-ACTION"), |_| PartstatJour::NeedsAction),
            map(tag("ACCEPTED"), |_| PartstatJour::Accepted),
            map(tag("DECLINED"), |_| PartstatJour::Declined),
            map(x_name, |value| PartstatJour::XName(value.to_string())),
            map(iana_token, |value| PartstatJour::IanaToken(value.to_string())),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::NeedsAction => String::from("NEEDS-ACTION"),
            Self::Accepted => String::from("ACCEPTED"),
            Self::Declined => String::from("DECLINED"),
            Self::XName(name) => name.to_owned(),
            Self::IanaToken(name) => name.to_owned(),
        }
    }
}

impl_icalendar_entity_traits!(PartstatJour);

// Participation Status
//
// Parameter Name:  PARTSTAT
//
// Purpose:  To specify the participation status for the calendar user
//    specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     partstatparam    = "PARTSTAT" "="
//                       (partstat-event
//                      / partstat-todo
//                      / partstat-jour)
//
//     partstat-event   = ("NEEDS-ACTION"    ; Event needs action
//                      / "ACCEPTED"         ; Event accepted
//                      / "DECLINED"         ; Event declined
//                      / "TENTATIVE"        ; Event tentatively
//                                           ; accepted
//                      / "DELEGATED"        ; Event delegated
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
//     ; These are the participation statuses for a "VEVENT".
//     ; Default is NEEDS-ACTION.
//
//     partstat-todo    = ("NEEDS-ACTION"    ; To-do needs action
//                      / "ACCEPTED"         ; To-do accepted
//                      / "DECLINED"         ; To-do declined
//                      / "TENTATIVE"        ; To-do tentatively
//                                           ; accepted
//                      / "DELEGATED"        ; To-do delegated
//                      / "COMPLETED"        ; To-do completed
//                                           ; COMPLETED property has
//                                           ; DATE-TIME completed
//                      / "IN-PROCESS"       ; To-do in process of
//                                           ; being completed
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
//     ; These are the participation statuses for a "VTODO".
//     ; Default is NEEDS-ACTION.
//
//     partstat-jour    = ("NEEDS-ACTION"    ; Journal needs action
//                      / "ACCEPTED"         ; Journal accepted
//                      / "DECLINED"         ; Journal declined
//                      / x-name             ; Experimental status
//                      / iana-token)        ; Other IANA-registered
//                                           ; status
//     ; These are the participation statuses for a "VJOURNAL".
//     ; Default is NEEDS-ACTION.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PartstatParam<T>(pub T)
where
    T: Partstat + ICalendarEntity,
;

impl<T> ICalendarEntity for PartstatParam<T>
where
    T: Partstat + ICalendarEntity,
{
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PARTSTATPARAM",
            map(
                pair(
                    tag("PARTSTAT"),
                    preceded(tag("="), cut(T::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("PARTSTAT={}", self.0.render_ical())
    }
}

impl<T> std::str::FromStr for PartstatParam<T>
where
    T: Partstat + ICalendarEntity,
{
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parser_result = nom::combinator::all_consuming(Self::parse_ical)(input.into());

        match parser_result {
            Ok((_remaining, value)) => Ok(value),

            Err(error) => {
                if let nom::Err::Error(error) = error {
                    Err(crate::convert_error(input, error))
                } else {
                    Err(error.to_string())
                }
            }
        }
    }
}

impl<T> ToString for PartstatParam<T>
where
    T: Partstat + ICalendarEntity,
{
    fn to_string(&self) -> String {
        self.render_ical()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            PartstatParam::parse_ical(r#"PARTSTAT=DECLINED TESTING"#.into()),
            (
                " TESTING",
                PartstatParam(PartstatEvent::Declined),
            ),
        );

        assert_parser_output!(
            PartstatParam::parse_ical(r#"PARTSTAT=DECLINED TESTING"#.into()),
            (
                " TESTING",
                PartstatParam(PartstatTodo::Declined),
            ),
        );

        assert_parser_output!(
            PartstatParam::parse_ical(r#"PARTSTAT=DECLINED TESTING"#.into()),
            (
                " TESTING",
                PartstatParam(PartstatJour::Declined),
            ),
        );

        assert!(PartstatParam::<PartstatEvent>::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            PartstatParam(PartstatEvent::Declined).render_ical(),
            String::from("PARTSTAT=DECLINED"),
        );

        assert_eq!(
            PartstatParam(PartstatTodo::XName(String::from("X-TEST-NAME"))).render_ical(),
            String::from("PARTSTAT=X-TEST-NAME"),
        );

        assert_eq!(
            PartstatParam(PartstatJour::IanaToken(String::from("TEST-IANA-NAME"))).render_ical(),
            String::from("PARTSTAT=TEST-IANA-NAME"),
        );
    }
}
