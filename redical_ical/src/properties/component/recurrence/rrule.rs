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

use crate::property_value_data_types::recur::Recur;

use crate::properties::define_property_params;

define_property_params!(
    RrulParams,
    RrulParam,
    "RRULPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Recurrence Rule
//
// Property Name:  RRULE
//
// Purpose:  This property defines a rule or repeating pattern for
//    recurring events, to-dos, journal entries, or time zone
//    definitions.
//
// Value Type:  RECUR
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in recurring "VEVENT",
//    "VTODO", and "VJOURNAL" calendar components as well as in the
//    "STANDARD" and "DAYLIGHT" sub-components of the "VTIMEZONE"
//    calendar component, but it SHOULD NOT be specified more than once.
//    The recurrence set generated with multiple "RRULE" properties is
//    undefined.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     rrule      = "RRULE" rrulparam ":" recur CRLF
//
//     rrulparam  = *(";" other-param)
//
// Example:  All examples assume the Eastern United States time zone.
//
//    Daily for 10 occurrences:
//
//     DTSTART;TZID=America/New_York:19970902T090000
//     RRULE:FREQ=DAILY;COUNT=10
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Rrule {
    pub params: RrulParams,
    pub value: Recur,
}

impl ICalendarEntity for Rrule {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RRULE",
            preceded(
                tag("RRULE"),
                cut(
                    map(
                        pair(
                            opt(RrulParams::parse_ical),
                            preceded(colon, Recur::parse_ical),
                        ),
                        |(params, value)| {
                            Rrule {
                                params: params.unwrap_or(RrulParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RRULE{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Rrule);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::grammar::List;

    use crate::property_value_data_types::integer::Integer;

    use crate::property_value_data_types::recur::{FreqParam, Frequency, IntervalParam, ByminuteParam, ByhourParam, BydayParam, BymonthParam, WeekDayNum, WeekDay};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Rrule::parse_ical("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                Rrule {
                    params: RrulParams::default(),
                    value: Recur {
                        freq: Some(FreqParam(Frequency::Yearly)),
                        until: None,
                        count: None,
                        interval: Some(IntervalParam(Integer(2))),
                        bysecond: None,
                        byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                        byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                        byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                        bymonthday: None,
                        byyearday: None,
                        byweekno: None,
                        bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                        bysetpos: None,
                        wkst: None,
                    },
                }
            )
        );

        assert_parser_output!(
            Rrule::parse_ical("RRULE;X-TEST=X_VALUE;TEST=VALUE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                Rrule {
                    params: RrulParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Recur {
                        freq: Some(FreqParam(Frequency::Yearly)),
                        until: None,
                        count: None,
                        interval: Some(IntervalParam(Integer(2))),
                        bysecond: None,
                        byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                        byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                        byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                        bymonthday: None,
                        byyearday: None,
                        byweekno: None,
                        bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                        bysetpos: None,
                        wkst: None,
                    },
                }
            )
        );

        assert!(Rrule::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Rrule {
                params: RrulParams::default(),
                value: Recur {
                    freq: Some(FreqParam(Frequency::Yearly)),
                    until: None,
                    count: None,
                    interval: Some(IntervalParam(Integer(2))),
                    bysecond: None,
                    byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                    byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                    byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                    bymonthday: None,
                    byyearday: None,
                    byweekno: None,
                    bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                    bysetpos: None,
                    wkst: None,
                },
            }.render_ical(),
            String::from("RRULE:FREQ=YEARLY;INTERVAL=2;BYMINUTE=30;BYHOUR=8,9;BYDAY=-1MO,SU;BYMONTH=1"),
        );

        assert_eq!(
            Rrule {
                params: RrulParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Recur {
                    freq: Some(FreqParam(Frequency::Yearly)),
                    until: None,
                    count: None,
                    interval: Some(IntervalParam(Integer(2))),
                    bysecond: None,
                    byminute: Some(ByminuteParam(List::from(vec![Integer(30)]))),
                    byhour: Some(ByhourParam(List::from(vec![Integer(8), Integer(9)]))),
                    byday: Some(BydayParam(List::from(vec![WeekDayNum(Some(Integer(-1)), WeekDay::Monday), WeekDayNum(None, WeekDay::Sunday)]))),
                    bymonthday: None,
                    byyearday: None,
                    byweekno: None,
                    bymonth: Some(BymonthParam(List::from(vec![Integer(1)]))),
                    bysetpos: None,
                    wkst: None,
                },
            }.render_ical(),
            String::from("RRULE;X-TEST=X_VALUE;TEST=VALUE:FREQ=YEARLY;INTERVAL=2;BYMINUTE=30;BYHOUR=8,9;BYDAY=-1MO,SU;BYMONTH=1"),
        );
    }
}
