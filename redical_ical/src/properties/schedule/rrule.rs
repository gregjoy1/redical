use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::property_value_data_types::recur::Recur;

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RRulePropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for RRulePropertyParams {
    define_property_params_ical_parser!(
        RRulePropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut RRulePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&RRulePropertyParams> for ContentLineParams {
    fn from(rrule_params: &RRulePropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in rrule_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<RRulePropertyParams> for ContentLineParams {
    fn from(rrule_params: RRulePropertyParams) -> Self {
        ContentLineParams::from(&rrule_params)
    }
}

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
pub struct RRuleProperty {
    pub params: RRulePropertyParams,
    pub value: Recur,
}

impl ICalendarEntity for RRuleProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RRULE",
            preceded(
                tag("RRULE"),
                cut(
                    map(
                        pair(
                            opt(RRulePropertyParams::parse_ical),
                            preceded(colon, Recur::parse_ical),
                        ),
                        |(params, value)| {
                            RRuleProperty {
                                params: params.unwrap_or(RRulePropertyParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        ContentLine::from(self).render_ical()
    }
}

impl From<&RRuleProperty> for ContentLine {
    fn from(rrule_property: &RRuleProperty) -> Self {
        ContentLine::from((
            "RRULE",
            (
                ContentLineParams::from(&rrule_property.params),
                rrule_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(RRuleProperty);

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
            RRuleProperty::parse_ical("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                RRuleProperty {
                    params: RRulePropertyParams::default(),
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
            RRuleProperty::parse_ical("RRULE;X-TEST=X_VALUE;TEST=VALUE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                RRuleProperty {
                    params: RRulePropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
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

        assert!(RRuleProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RRuleProperty {
                params: RRulePropertyParams::default(),
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
            RRuleProperty {
                params: RRulePropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
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
            String::from("RRULE;TEST=VALUE;X-TEST=X_VALUE:FREQ=YEARLY;INTERVAL=2;BYMINUTE=30;BYHOUR=8,9;BYDAY=-1MO,SU;BYMONTH=1"),
        );
    }
}
