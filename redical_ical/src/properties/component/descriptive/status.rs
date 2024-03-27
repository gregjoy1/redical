use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    StatParams,
    StatParam,
    "STATPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

pub trait Statvalue {}

// statvalue-event = "TENTATIVE"    ;Indicates event is tentative.
//                 / "CONFIRMED"    ;Indicates event is definite.
//                 / "CANCELLED"    ;Indicates event was cancelled.
// ;Status values for a "VEVENT"
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StatvalueEvent {
    Tentative,
    Confirmed,
    Cancelled,
}

impl Statvalue for StatvalueEvent {}

impl ICalendarEntity for StatvalueEvent {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("TENTATIVE"), |_| StatvalueEvent::Tentative),
            map(tag("CONFIRMED"), |_| StatvalueEvent::Confirmed),
            map(tag("CANCELLED"), |_| StatvalueEvent::Cancelled),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Tentative => String::from("TENTATIVE"),
            Self::Confirmed => String::from("CONFIRMED"),
            Self::Cancelled => String::from("CANCELLED"),
        }
    }
}

impl_icalendar_entity_traits!(StatvalueEvent);

// statvalue-todo  = "NEEDS-ACTION" ;Indicates to-do needs action.
//                 / "COMPLETED"    ;Indicates to-do completed.
//                 / "IN-PROCESS"   ;Indicates to-do in process of.
//                 / "CANCELLED"    ;Indicates to-do was cancelled.
// ;Status values for "VTODO".
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StatvalueTodo {
    NeedsAction,
    Completed,
    InProcess,
    Cancelled,
}

impl Statvalue for StatvalueTodo {}

impl ICalendarEntity for StatvalueTodo {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("NEEDS-ACTION"), |_| StatvalueTodo::NeedsAction),
            map(tag("COMPLETED"), |_| StatvalueTodo::Completed),
            map(tag("IN-PROCESS"), |_| StatvalueTodo::InProcess),
            map(tag("CANCELLED"), |_| StatvalueTodo::Cancelled),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::NeedsAction => String::from("NEEDS-ACTION"),
            Self::Completed => String::from("COMPLETED"),
            Self::InProcess => String::from("IN-PROCESS"),
            Self::Cancelled => String::from("CANCELLED"),
        }
    }
}

impl_icalendar_entity_traits!(StatvalueTodo);

//  statvalue-jour  = "DRAFT"        ;Indicates journal is draft.
//                  / "FINAL"        ;Indicates journal is final.
//                  / "CANCELLED"    ;Indicates journal is removed.
// ;Status values for "VJOURNAL".
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum StatvalueJour {
    Draft,
    Final,
    Cancelled,
}

impl Statvalue for StatvalueJour {}

impl ICalendarEntity for StatvalueJour {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(tag("DRAFT"), |_| StatvalueJour::Draft),
            map(tag("FINAL"), |_| StatvalueJour::Final),
            map(tag("CANCELLED"), |_| StatvalueJour::Cancelled),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Draft => String::from("DRAFT"),
            Self::Final => String::from("FINAL"),
            Self::Cancelled => String::from("CANCELLED"),
        }
    }
}

impl_icalendar_entity_traits!(StatvalueJour);

// Status
//
// Property Name:  STATUS
//
// Purpose:  This property defines the overall status or confirmation
//    for the calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified once in "VEVENT",
//    "VTODO", or "VJOURNAL" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     status          = "STATUS" statparam ":" statvalue CRLF
//
//     statparam       = *(";" other-param)
//
//     statvalue       = (statvalue-event
//                     /  statvalue-todo
//                     /  statvalue-jour)
//
//     statvalue-event = "TENTATIVE"    ;Indicates event is tentative.
//                     / "CONFIRMED"    ;Indicates event is definite.
//                     / "CANCELLED"    ;Indicates event was cancelled.
//     ;Status values for a "VEVENT"
//
//     statvalue-todo  = "NEEDS-ACTION" ;Indicates to-do needs action.
//                     / "COMPLETED"    ;Indicates to-do completed.
//                     / "IN-PROCESS"   ;Indicates to-do in process of.
//                     / "CANCELLED"    ;Indicates to-do was cancelled.
//     ;Status values for "VTODO".
//
//     statvalue-jour  = "DRAFT"        ;Indicates journal is draft.
//                     / "FINAL"        ;Indicates journal is final.
//                     / "CANCELLED"    ;Indicates journal is removed.
//    ;Status values for "VJOURNAL".
//
// Example:  The following is an example of this property for a "VEVENT"
//    calendar component:
//
//     STATUS:TENTATIVE
//
//    The following is an example of this property for a "VTODO"
//    calendar component:
//
//     STATUS:NEEDS-ACTION
//
//    The following is an example of this property for a "VJOURNAL"
//    calendar component:
//
//     STATUS:DRAFT
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Status<T: Statvalue + ICalendarEntity> {
    pub params: StatParams,
    pub value: T,
}

impl<T> ICalendarEntity for Status<T>
where
    T: Statvalue + ICalendarEntity,
{
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "STATUS",
            preceded(
                tag("STATUS"),
                cut(
                    map(
                        pair(
                            opt(StatParams::parse_ical),
                            preceded(colon, T::parse_ical),
                        ),
                        |(params, value)| {
                            Status {
                                params: params.unwrap_or(StatParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("STATUS{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl<T> std::str::FromStr for Status<T>
where
    T: Statvalue + ICalendarEntity,
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

impl<T> ToString for Status<T>
where
    T: Statvalue + ICalendarEntity,
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
            Status::parse_ical("STATUS:TENTATIVE".into()),
            (
                "",
                Status {
                    params: StatParams::default(),
                    value: StatvalueEvent::Tentative,
                },
            ),
        );

        assert_parser_output!(
            Status::parse_ical("STATUS;X-TEST=X_VALUE;TEST=VALUE:NEEDS-ACTION".into()),
            (
                "",
                Status {
                    params: StatParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: StatvalueTodo::NeedsAction,
                },
            ),
        );

        assert_parser_output!(
            Status::parse_ical("STATUS;X-TEST=X_VALUE;TEST=VALUE:DRAFT".into()),
            (
                "",
                Status {
                    params: StatParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: StatvalueJour::Draft,
                },
            ),
        );

        assert!(Status::<StatvalueEvent>::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Status {
                params: StatParams::default(),
                value: StatvalueEvent::Tentative,
            }.render_ical(),
            String::from("STATUS:TENTATIVE"),
        );

        assert_eq!(
            Status {
                params: StatParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: StatvalueTodo::NeedsAction,
            }.render_ical(),
            String::from("STATUS;X-TEST=X_VALUE;TEST=VALUE:NEEDS-ACTION"),
        );

        assert_eq!(
            Status {
                params: StatParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: StatvalueJour::Draft,
            }.render_ical(),
            String::from("STATUS;X-TEST=X_VALUE;TEST=VALUE:DRAFT"),
        );
    }
}
