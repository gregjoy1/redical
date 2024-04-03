use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::value_data_types::recur::Recur;

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ExRulePropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for ExRulePropertyParams {
    define_property_params_ical_parser!(
        ExRulePropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut ExRulePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&ExRulePropertyParams> for ContentLineParams {
    fn from(exrule_params: &ExRulePropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in exrule_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<ExRulePropertyParams> for ContentLineParams {
    fn from(exrule_params: ExRulePropertyParams) -> Self {
        ContentLineParams::from(&exrule_params)
    }
}

// Exception Recurrence Rule
//
// Property Name:  EXRULE
//
// Deprecated officially, supporting for legacy purposes.
//
// Purpose:  This property defines a rule or repeating pattern for
//    exceptions to recurring events, to-dos, journal entries, or
//    time zone definitions.
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
//    The recurrence set generated with multiple "EXRULE" properties is
//    undefined.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     exrule      = "EXRULE" rrulparam ":" recur CRLF
//
//     rrulparam  = *(";" other-param)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExRuleProperty {
    pub params: ExRulePropertyParams,
    pub value: Recur,
}

impl ICalendarEntity for ExRuleProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "EXRULE",
            preceded(
                tag("EXRULE"),
                cut(
                    map(
                        pair(
                            opt(ExRulePropertyParams::parse_ical),
                            preceded(colon, Recur::parse_ical),
                        ),
                        |(params, value)| {
                            ExRuleProperty {
                                params: params.unwrap_or(ExRulePropertyParams::default()),
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

impl From<&ExRuleProperty> for ContentLine {
    fn from(exrule_property: &ExRuleProperty) -> Self {
        ContentLine::from((
            "EXRULE",
            (
                ContentLineParams::from(&exrule_property.params),
                exrule_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(ExRuleProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::value_data_types::integer::Integer;
    use crate::value_data_types::list::List;

    use crate::value_data_types::recur::{FreqParam, Frequency, IntervalParam, ByminuteParam, ByhourParam, BydayParam, BymonthParam, WeekDayNum, WeekDay};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ExRuleProperty::parse_ical("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ExRuleProperty {
                    params: ExRulePropertyParams::default(),
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
            ExRuleProperty::parse_ical("EXRULE;X-TEST=X_VALUE;TEST=VALUE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 TESTING".into()),
            (
                " TESTING",
                ExRuleProperty {
                    params: ExRulePropertyParams {
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

        assert!(ExRuleProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ExRuleProperty {
                params: ExRulePropertyParams::default(),
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
            String::from("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMINUTE=30;BYHOUR=8,9;BYDAY=-1MO,SU;BYMONTH=1"),
        );

        assert_eq!(
            ExRuleProperty {
                params: ExRulePropertyParams {
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
            String::from("EXRULE;TEST=VALUE;X-TEST=X_VALUE:FREQ=YEARLY;INTERVAL=2;BYMINUTE=30;BYHOUR=8,9;BYDAY=-1MO,SU;BYMONTH=1"),
        );
    }
}
