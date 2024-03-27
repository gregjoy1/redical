use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon, List};
use crate::property_parameters::{
    fbtype::FbtypeParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::period::Period;

use crate::properties::define_property_params;

define_property_params!(
    FbParams,
    FbParam,
    "FBPARAM",
    (Fbtype, FbtypeParam, fbtype, Option<FbtypeParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Free/Busy Time
//
// Property Name:  FREEBUSY
//
// Purpose:  This property defines one or more free or busy time
//    intervals.
//
// Value Type:  PERIOD
//
// Property Parameters:  IANA, non-standard, and free/busy time type
//    property parameters can be specified on this property.
//
// Conformance:  The property can be specified in a "VFREEBUSY" calendar
//    component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     freebusy   = "FREEBUSY" fbparam ":" fbvalue CRLF
//
//     fbparam    = *(
//                ;
//                ; The following is OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" fbtypeparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//     fbvalue    = period *("," period)
//     ;Time value MUST be in the UTC time format.
//
// Example:  The following are some examples of this property:
//
//     FREEBUSY;FBTYPE=BUSY-UNAVAILABLE:19970308T160000Z/PT8H30M
//
//     FREEBUSY;FBTYPE=FREE:19970308T160000Z/PT3H,19970308T200000Z/PT1H
//
//     FREEBUSY;FBTYPE=FREE:19970308T160000Z/PT3H,19970308T200000Z/PT1H
//      ,19970308T230000Z/19970309T000000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Freebusy {
    pub params: FbParams,
    pub value: List<Period>,
}

impl ICalendarEntity for Freebusy {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "FREEBUSY",
            preceded(
                tag("FREEBUSY"),
                cut(
                    map(
                        pair(
                            opt(FbParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, value)| {
                            Freebusy {
                                params: params.unwrap_or(FbParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("FREEBUSY{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Freebusy);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::fbtype::Fbtype;

    use crate::property_value_data_types::{
        duration::Duration,
        date_time::DateTime,
        date::Date,
        time::Time,
    };

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Freebusy::parse_ical("FREEBUSY;FBTYPE=BUSY-UNAVAILABLE:19970308T160000Z/PT8H30M".into()),
            (
                "",
                Freebusy {
                    params: FbParams {
                        fbtype: Some(FbtypeParam(Fbtype::BusyUnavailable)),
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: List::from(
                        vec![
                            Period::Start(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 16_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                Duration {
                                    positive_negative: None,
                                    weeks: None,
                                    days: None,
                                    hours: Some(8),
                                    minutes: Some(30),
                                    seconds: None,
                                },
                            )
                        ]
                    )
                },
            ),
        );

        assert_parser_output!(
            Freebusy::parse_ical("FREEBUSY;FBTYPE=FREE:19970308T160000Z/PT3H,19970308T200000Z/PT1H".into()),
            (
                "",
                Freebusy {
                    params: FbParams {
                        fbtype: Some(FbtypeParam(Fbtype::Free)),
                        iana: IanaParams::default(),
                        x: XParams::default()
                    },
                    value: List::from(
                        vec![
                            Period::Start(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 16_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                Duration {
                                    positive_negative: None,
                                    weeks: None,
                                    days: None,
                                    hours: Some(3),
                                    minutes: None,
                                    seconds: None,
                                },
                            ),
                            Period::Start(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 20_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                Duration {
                                    positive_negative: None,
                                    weeks: None,
                                    days: None,
                                    hours: Some(1),
                                    minutes: None,
                                    seconds: None,
                                },
                            )
                        ]
                    )
                },
            ),
        );

        assert_parser_output!(
            Freebusy::parse_ical("FREEBUSY;X-TEST=X_VALUE;TEST=VALUE;FBTYPE=FREE:19970308T160000Z/PT3H,19970308T200000Z/PT1H,19970308T230000Z/19970309T000000Z".into()),
            (
                "",
                Freebusy {
                    params: FbParams {
                        fbtype: Some(FbtypeParam(Fbtype::Free)),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: List::from(
                        vec![
                            Period::Start(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 16_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                Duration {
                                    positive_negative: None,
                                    weeks: None,
                                    days: None,
                                    hours: Some(3),
                                    minutes: None,
                                    seconds: None,
                                },
                            ),
                            Period::Start(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 20_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                Duration {
                                    positive_negative: None,
                                    weeks: None,
                                    days: None,
                                    hours: Some(1),
                                    minutes: None,
                                    seconds: None,
                                },
                            ),
                            Period::Explicit(
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 8_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 23_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                                DateTime {
                                    date: Date {
                                        year: 1997_i32,
                                        month: 3_u32,
                                        day: 9_u32,
                                    },
                                    time: Some(
                                        Time{
                                            hour: 0_u32,
                                            minute: 0_u32,
                                            second: 0_u32,
                                            is_utc: true,
                                        }
                                    )
                                },
                            )
                        ]
                    )
                },
            ),
        );

        assert!(Freebusy::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Freebusy {
                params: FbParams {
                    fbtype: Some(FbtypeParam(Fbtype::BusyUnavailable)),
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: List::from(
                    vec![
                        Period::Start(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 16_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            Duration {
                                positive_negative: None,
                                weeks: None,
                                days: None,
                                hours: Some(8),
                                minutes: Some(30),
                                seconds: None,
                            },
                        )
                    ]
                )
            }.render_ical(),
            String::from("FREEBUSY;FBTYPE=BUSY-UNAVAILABLE:19970308T160000Z/PT8H30M"),
        );

        assert_eq!(
            Freebusy {
                params: FbParams {
                    fbtype: Some(FbtypeParam(Fbtype::Free)),
                    iana: IanaParams::default(),
                    x: XParams::default()
                },
                value: List::from(
                    vec![
                        Period::Start(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 16_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            Duration {
                                positive_negative: None,
                                weeks: None,
                                days: None,
                                hours: Some(3),
                                minutes: None,
                                seconds: None,
                            },
                        ),
                        Period::Start(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 20_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            Duration {
                                positive_negative: None,
                                weeks: None,
                                days: None,
                                hours: Some(1),
                                minutes: None,
                                seconds: None,
                            },
                        )
                    ]
                )
            }.render_ical(),
            String::from("FREEBUSY;FBTYPE=FREE:19970308T160000Z/PT3H,19970308T200000Z/PT1H"),
        );

        assert_eq!(
            Freebusy {
                params: FbParams {
                    fbtype: Some(FbtypeParam(Fbtype::BusyUnavailable)),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: List::from(
                    vec![
                        Period::Start(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 16_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            Duration {
                                positive_negative: None,
                                weeks: None,
                                days: None,
                                hours: Some(3),
                                minutes: None,
                                seconds: None,
                            },
                        ),
                        Period::Start(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 20_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            Duration {
                                positive_negative: None,
                                weeks: None,
                                days: None,
                                hours: Some(1),
                                minutes: None,
                                seconds: None,
                            },
                        ),
                        Period::Explicit(
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 8_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 23_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                            DateTime {
                                date: Date {
                                    year: 1997_i32,
                                    month: 3_u32,
                                    day: 9_u32,
                                },
                                time: Some(
                                    Time{
                                        hour: 0_u32,
                                        minute: 0_u32,
                                        second: 0_u32,
                                        is_utc: true,
                                    }
                                )
                            },
                        )
                    ]
                )
            }.render_ical(),
            String::from("FREEBUSY;FBTYPE=BUSY-UNAVAILABLE;X-TEST=X_VALUE;TEST=VALUE:19970308T160000Z/PT3H,19970308T200000Z/PT1H,19970308T230000Z/19970309T000000Z"),
        );
    }
}
