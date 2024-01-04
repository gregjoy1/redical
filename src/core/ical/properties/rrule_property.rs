use rrule::Tz;

use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1},
    combinator::{cut, map, opt, recognize},
    error::{context, VerboseError, VerboseErrorKind},
    multi::separated_list1,
    sequence::{preceded, separated_pair, tuple},
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::{ParsedValue, ParserResult};
use crate::core::ical::parser::macros::*;
use crate::core::ical::serializer::{
    quote_string_if_needed, serialize_timestamp_to_ical_datetime, SerializableICalProperty,
    SerializedValue,
};

#[derive(Debug, PartialEq)]
pub struct RRuleProperty {
    freq: String,
    interval: usize,
    count: Option<usize>,
    wkst: Option<String>,
    until_utc_timestamp: Option<i64>,
    by_second: Option<Vec<String>>,
    by_minute: Option<Vec<String>>,
    by_hour: Option<Vec<String>>,
    by_day: Option<Vec<String>>,
    by_week_no: Option<Vec<String>>,
    by_month: Option<Vec<String>>,
    by_month_day: Option<Vec<String>>,
    by_year_day: Option<Vec<String>>,
    by_easter: Option<Vec<String>>,
    by_set_pos: Option<Vec<String>>,

    x_params: Option<HashMap<String, Vec<String>>>,
}

impl SerializableICalProperty for RRuleProperty {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

        if let Some(x_params) = &self.x_params {
            for (key, values) in x_params {
                let param_value = values
                    .iter()
                    .map(|value| quote_string_if_needed(value, common::param_value))
                    .collect::<Vec<String>>()
                    .join(",");

                param_key_value_pairs.push((key.clone(), param_value));
            }
        }

        param_key_value_pairs.sort();

        let params = if param_key_value_pairs.is_empty() {
            None
        } else {
            Some(param_key_value_pairs)
        };

        let mut values: Vec<(String, SerializedValue)> = Vec::new();

        values.push((
            String::from("FREQ"),
            SerializedValue::Single(self.freq.clone()),
        ));

        values.push((
            String::from("INTERVAL"),
            SerializedValue::Single(self.interval.to_string()),
        ));

        if let Some(count) = self.count {
            values.push((
                String::from("COUNT"),
                SerializedValue::Single(count.to_string()),
            ));
        }

        if let Some(wkst) = &self.wkst {
            values.push((String::from("WKST"), SerializedValue::Single(wkst.clone())));
        }

        if let Some(until_utc_timestamp) = self.until_utc_timestamp {
            values.push((
                String::from("UNTIL"),
                SerializedValue::Single(serialize_timestamp_to_ical_datetime(
                    &until_utc_timestamp,
                    &Tz::UTC,
                )),
            ));
        }

        if let Some(by_second) = &self.by_second {
            values.push((
                String::from("BYSECOND"),
                SerializedValue::List(by_second.clone()),
            ));
        }

        if let Some(by_minute) = &self.by_minute {
            values.push((
                String::from("BYMINUTE"),
                SerializedValue::List(by_minute.clone()),
            ));
        }

        if let Some(by_hour) = &self.by_hour {
            values.push((
                String::from("BYHOUR"),
                SerializedValue::List(by_hour.clone()),
            ));
        }

        if let Some(by_day) = &self.by_day {
            values.push((String::from("BYDAY"), SerializedValue::List(by_day.clone())));
        }

        if let Some(by_week_no) = &self.by_week_no {
            values.push((
                String::from("BYWEEKNO"),
                SerializedValue::List(by_week_no.clone()),
            ));
        }

        if let Some(by_month) = &self.by_month {
            values.push((
                String::from("BYMONTH"),
                SerializedValue::List(by_month.clone()),
            ));
        }

        if let Some(by_month_day) = &self.by_month_day {
            values.push((
                String::from("BYMONTHDAY"),
                SerializedValue::List(by_month_day.clone()),
            ));
        }

        if let Some(by_year_day) = &self.by_year_day {
            values.push((
                String::from("BYYEARDAY"),
                SerializedValue::List(by_year_day.clone()),
            ));
        }

        if let Some(by_easter) = &self.by_easter {
            values.push((
                String::from("BYEASTER"),
                SerializedValue::List(by_easter.clone()),
            ));
        }

        if let Some(by_set_pos) = &self.by_set_pos {
            values.push((
                String::from("BYSETPOS"),
                SerializedValue::List(by_set_pos.clone()),
            ));
        }

        values.sort();

        let value = SerializedValue::Params(values);

        (String::from(RRuleProperty::NAME), params, value)
    }
}

impl RRuleProperty {
    const NAME: &'static str = "RRULE";

    // freq       = "SECONDLY" / "MINUTELY" / "HOURLY" / "DAILY"
    //            / "WEEKLY" / "MONTHLY" / "YEARLY"
    fn freq(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context(
            "freq",
            alt((
                tag("SECONDLY"),
                tag("MINUTELY"),
                tag("HOURLY"),
                tag("DAILY"),
                tag("WEEKLY"),
                tag("MONTHLY"),
                tag("YEARLY"),
            )),
        ))(input)
    }

    // interval    = 1*DIGIT
    fn interval(input: &str) -> ParserResult<&str, ParsedValue> {
        context("interval", ParsedValue::parse_single(digit1))(input)
    }

    // count    = 1*DIGIT
    fn count(input: &str) -> ParserResult<&str, ParsedValue> {
        context("count", ParsedValue::parse_single(digit1))(input)
    }

    // enddate    = date
    // enddate    =/ date-time            ;An UTC value
    fn enddate(input: &str) -> ParserResult<&str, ParsedValue> {
        context("enddate", ParsedValue::parse_date_string)(input)
    }

    // byseclist  = seconds / ( seconds *("," seconds) )
    fn byseclist(input: &str) -> ParserResult<&str, ParsedValue> {
        context("byseclist", ParsedValue::parse_list(Self::seconds))(input)
    }

    // seconds    = 1DIGIT / 2DIGIT       ;0 to 59
    fn seconds(input: &str) -> ParserResult<&str, &str> {
        context("seconds", digit1)(input)
    }

    // byminlist  = minutes / ( minutes *("," minutes) )
    fn byminlist(input: &str) -> ParserResult<&str, ParsedValue> {
        context("byminlist", ParsedValue::parse_list(Self::minutes))(input)
    }

    // minutes    = 1DIGIT / 2DIGIT       ;0 to 59
    fn minutes(input: &str) -> ParserResult<&str, &str> {
        context("minutes", digit1)(input)
    }

    // byhrlist   = hour / ( hour *("," hour) )
    fn byhrlist(input: &str) -> ParserResult<&str, ParsedValue> {
        context("byhrlist", ParsedValue::parse_list(Self::hour))(input)
    }

    // hour       = 1DIGIT / 2DIGIT       ;0 to 23
    fn hour(input: &str) -> ParserResult<&str, &str> {
        context("hour", digit1)(input)
    }

    // bywdaylist = weekdaynum / ( weekdaynum *("," weekdaynum) )
    fn bywdaylist(input: &str) -> ParserResult<&str, ParsedValue> {
        context("bywdaylist", ParsedValue::parse_list(Self::weekdaynum))(input)
    }

    // weekdaynum = [([plus] ordwk / minus ordwk)] weekday
    fn weekdaynum(input: &str) -> ParserResult<&str, &str> {
        context(
            "weekdaynum",
            recognize(tuple((
                opt(alt((Self::plus, Self::minus))),
                opt(Self::ordwk),
                Self::weekday,
            ))),
        )(input)
    }

    // plus       = "+"
    fn plus(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context("plus", tag("+")))(input)
    }

    // minus      = "-"
    fn minus(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context("minus", tag("-")))(input)
    }

    // ordwk      = 1DIGIT / 2DIGIT       ;1 to 53
    fn ordwk(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context("ordwk", digit1))(input)
    }

    // weekday    = "SU" / "MO" / "TU" / "WE" / "TH" / "FR" / "SA"
    // ;Corresponding to SUNDAY, MONDAY, TUESDAY, WEDNESDAY, THURSDAY,
    // ;FRIDAY, SATURDAY and SUNDAY days of the week.
    fn weekday(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context(
            "weekday",
            alt((
                tag("SU"),
                tag("MO"),
                tag("TU"),
                tag("WE"),
                tag("TH"),
                tag("FR"),
                tag("SA"),
            )),
        ))(input)
    }

    // bymodaylist = monthdaynum / ( monthdaynum *("," monthdaynum) )
    fn bymodaylist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("bymodaylist", Self::monthdaynum))(input)
    }

    // monthdaynum = ([plus] ordmoday) / (minus ordmoday)
    fn monthdaynum(input: &str) -> ParserResult<&str, &str> {
        context(
            "monthdaynum",
            recognize(tuple((opt(alt((Self::plus, Self::minus))), Self::ordmoday))),
        )(input)
    }

    // ordmoday   = 1DIGIT / 2DIGIT       ;1 to 31
    fn ordmoday(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context("ordmoday", digit1))(input)
    }

    // byyrdaylist = yeardaynum / ( yeardaynum *("," yeardaynum) )
    fn byyrdaylist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("byyrdaylist", Self::yeardaynum))(input)
    }

    // yeardaynum = ([plus] ordyrday) / (minus ordyrday)
    fn yeardaynum(input: &str) -> ParserResult<&str, &str> {
        context(
            "yeardaynum",
            recognize(tuple((opt(alt((Self::plus, Self::minus))), Self::ordyrday))),
        )(input)
    }

    // ordyrday   = 1DIGIT / 2DIGIT / 3DIGIT      ;1 to 366
    fn ordyrday(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_single(context("ordyrday", digit1))(input)
    }

    // bywknolist = weeknum / ( weeknum *("," weeknum) )
    fn bywknolist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("bywknolist", Self::weeknum))(input)
    }

    // weeknum    = ([plus] ordwk) / (minus ordwk)
    fn weeknum(input: &str) -> ParserResult<&str, &str> {
        context(
            "weeknum",
            recognize(tuple((opt(alt((Self::plus, Self::minus))), Self::ordwk))),
        )(input)
    }

    // bymolist   = monthnum / ( monthnum *("," monthnum) )
    fn bymolist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("bymolist", Self::monthnum))(input)
    }

    // monthnum   = 1DIGIT / 2DIGIT       ;1 to 12
    fn monthnum(input: &str) -> ParserResult<&str, &str> {
        context("monthnum", digit1)(input)
    }

    // bysplist   = setposday / ( setposday *("," setposday) )
    fn bysplist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("bysplist", Self::setposday))(input)
    }

    // setposday  = yeardaynum
    fn setposday(input: &str) -> ParserResult<&str, &str> {
        context("setposday", Self::yeardaynum)(input)
    }

    // byeasterlist = easternum / ( easternum *("," easternum) )
    fn byeasterlist(input: &str) -> ParserResult<&str, ParsedValue> {
        ParsedValue::parse_list(context("byeasterlist", Self::easternum))(input)
    }

    // easternum = ([plus] ordeaster) / (minus ordeaster)
    fn easternum(input: &str) -> ParserResult<&str, &str> {
        context(
            "yeardaynum",
            recognize(tuple((
                opt(alt((Self::plus, Self::minus))),
                Self::ordeaster,
            ))),
        )(input)
    }

    // ordeaster   = 1DIGIT / 2DIGIT      ;0+
    fn ordeaster(input: &str) -> ParserResult<&str, &str> {
        context("ordeaster", digit1)(input)
    }

    pub fn parse_ical(input: &str) -> ParserResult<&str, RRuleProperty> {
        preceded(
            tag("RRULE"),
            cut(context(
                "RRULE",
                tuple((
                    build_property_params_parser!("RRULE"),
                    common::colon_delimeter,
                    build_property_params_value_parser!(
                        "RRULE",
                        ("FREQ", Self::freq),
                        ("INTERVAL", Self::interval),
                        ("COUNT", Self::count),
                        ("WKST", Self::weekday),
                        ("UNTIL", Self::enddate),
                        ("BYSECOND", Self::byseclist),
                        ("BYMINUTE", Self::byminlist),
                        ("BYHOUR", Self::byhrlist),
                        ("BYDAY", Self::bywdaylist),
                        ("BYWEEKNO", Self::bywknolist),
                        ("BYMONTH", Self::bymolist),
                        ("BYMONTHDAY", Self::bymodaylist),
                        ("BYYEARDAY", Self::byyrdaylist),
                        ("BYEASTER", Self::byeasterlist),
                        ("BYSETPOS", Self::bysplist),
                    ),
                )),
            )),
        )(input)
        .and_then(
            |(remaining, (x_params, _colon_delimeter, parsed_value_params)): (
                &str,
                (
                    Option<HashMap<&str, ParsedValue>>,
                    &str,
                    HashMap<&str, ParsedValue>,
                ),
            )| {
                let mut freq: String = String::from("");
                let mut interval: usize = 0;
                let mut count: Option<usize> = None;
                let mut wkst: Option<String> = None;
                let mut until_utc_timestamp: Option<i64> = None;
                let mut by_second: Option<Vec<String>> = None;
                let mut by_minute: Option<Vec<String>> = None;
                let mut by_hour: Option<Vec<String>> = None;
                let mut by_day: Option<Vec<String>> = None;
                let mut by_week_no: Option<Vec<String>> = None;
                let mut by_month: Option<Vec<String>> = None;
                let mut by_month_day: Option<Vec<String>> = None;
                let mut by_year_day: Option<Vec<String>> = None;
                let mut by_easter: Option<Vec<String>> = None;
                let mut by_set_pos: Option<Vec<String>> = None;

                let x_params: Option<HashMap<String, Vec<String>>> =
                    x_params.and_then(|x_params| {
                        Some(
                            x_params
                                .into_iter()
                                .map(|(key, value)| {
                                    let parsed_value = value
                                        .expect_list()
                                        .into_iter()
                                        .map(String::from)
                                        .collect();

                                    (String::from(key), parsed_value)
                                })
                                .collect(),
                        )
                    });

                for (key, value) in parsed_value_params {
                    match key {
                        "FREQ" => {
                            freq = String::from(value.expect_single());
                        }

                        "INTERVAL" => {
                            match value.expect_single().parse() {
                                Ok(parsed_interval) => {
                                    interval = parsed_interval;
                                }

                                Err(_error) => {
                                    return Err(nom::Err::Error(VerboseError {
                                        errors: vec![(
                                            input,
                                            VerboseErrorKind::Context("parse interval error"),
                                        )],
                                    }));
                                }
                            };
                        }

                        "COUNT" => {
                            match value.expect_single().parse() {
                                Ok(parsed_count) => {
                                    let _ = count.insert(parsed_count);
                                }

                                Err(_error) => {
                                    return Err(nom::Err::Error(VerboseError {
                                        errors: vec![(
                                            input,
                                            VerboseErrorKind::Context("parse count error"),
                                        )],
                                    }));
                                }
                            };
                        }

                        "WKST" => {
                            let parsed_wkst = String::from(value.expect_single());
                            let _ = wkst.insert(parsed_wkst);
                        }

                        "UNTIL" => {
                            match value.expect_date_string().to_date(Some(Tz::UTC), "UNTIL") {
                                Ok(parsed_datetime) => {
                                    let _ = until_utc_timestamp.insert(parsed_datetime.timestamp());
                                }

                                Err(_error) => {
                                    return Err(nom::Err::Error(VerboseError {
                                        errors: vec![(
                                            input,
                                            VerboseErrorKind::Context("parse until date error"),
                                        )],
                                    }));
                                }
                            };
                        }

                        "BYSECOND" => {
                            let _ = by_second.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYMINUTE" => {
                            let _ = by_minute.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYHOUR" => {
                            let _ = by_hour.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYDAY" => {
                            let _ = by_day.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYWEEKNO" => {
                            let _ = by_week_no.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYMONTH" => {
                            let _ = by_month.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYMONTHDAY" => {
                            let parsed_by_month_day = value
                                .expect_list()
                                .into_iter()
                                .map(String::from)
                                .collect();
                            let _ = by_month_day.insert(parsed_by_month_day);
                        }

                        "BYYEARDAY" => {
                            let _ = by_year_day.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYEASTER" => {
                            let _ = by_easter.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        "BYSETPOS" => {
                            let _ = by_set_pos.insert(
                                value
                                    .expect_list()
                                    .into_iter()
                                    .map(String::from)
                                    .collect(),
                            );
                        }

                        _ => {}
                    }
                }

                let parsed_property = RRuleProperty {
                    freq,
                    interval,
                    count,
                    wkst,
                    until_utc_timestamp,
                    by_second,
                    by_minute,
                    by_hour,
                    by_day,
                    by_week_no,
                    by_month,
                    by_month_day,
                    by_year_day,
                    by_easter,
                    by_set_pos,

                    x_params,
                };

                Ok((remaining, parsed_property))
            },
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            RRuleProperty::parse_ical("RRULE:FREQ=DAILY;COUNT=10;INTERVAL=2"),
            Ok((
                "",
                RRuleProperty {
                    freq: String::from("DAILY"),
                    interval: 2,
                    count: Some(10),
                    wkst: None,
                    until_utc_timestamp: None,
                    by_second: None,
                    by_minute: None,
                    by_hour: None,
                    by_day: None,
                    by_week_no: None,
                    by_month: None,
                    by_month_day: None,
                    by_year_day: None,
                    by_easter: None,
                    by_set_pos: None,

                    x_params: None,
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            RRuleProperty::parse_ical(
                r#"RRULE;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":FREQ=MONTHLY;INTERVAL=2;COUNT=10;WKST=SU;UNTIL=19971007T000000Z;BYSECOND=1,30;BYMINUTE=1,30;BYHOUR=1,6;BYDAY=-1SU,2WE;BYWEEKNO=20;BYMONTH=3,6;BYMONTHDAY=7,10;BYYEARDAY=1,30,60;BYEASTER=-1,3;BYSETPOS=3"#,
            ),
            Ok((
                "",
                RRuleProperty {
                    freq: String::from("MONTHLY"),
                    interval: 2,
                    count: Some(10),
                    wkst: Some(String::from("SU")),
                    until_utc_timestamp: Some(876182400),
                    by_second: Some(vec![String::from("1"), String::from("30")]),
                    by_minute: Some(vec![String::from("1"), String::from("30")]),
                    by_hour: Some(vec![String::from("1"), String::from("6")]),
                    by_day: Some(vec![String::from("-1SU"), String::from("2WE")]),
                    by_week_no: Some(vec![String::from("20")]),
                    by_month: Some(vec![String::from("3"), String::from("6")]),
                    by_month_day: Some(vec![String::from("7"), String::from("10")]),
                    by_year_day: Some(vec![
                        String::from("1"),
                        String::from("30"),
                        String::from("60"),
                    ]),
                    by_easter: Some(vec![String::from("-1"), String::from("3")]),
                    by_set_pos: Some(vec![String::from("3")]),

                    x_params: Some(HashMap::from([
                        (
                            String::from("X-TEST-KEY-TWO"),
                            vec![String::from("KEY -🎄- TWO")]
                        ),
                        (
                            String::from("X-TEST-KEY-ONE"),
                            vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                        ),
                    ])),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            RRuleProperty::parse_ical(
                r#"RRULE;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":FREQ=MONTHLY;INTERVAL=2;COUNT=10;WKST=SU;UNTIL=19971007T000000Z;BYSECOND=1,30;BYMINUTE=1,30;BYHOUR=1,6;BYDAY=-1SU,2WE;BYWEEKNO=20;BYMONTH=3,6;BYMONTHDAY=7,10;BYYEARDAY=1,30,60;BYEASTER=-1,3;BYSETPOS=3 SUMMARY:Summary text."#,
            ),
            Ok((
                " SUMMARY:Summary text.",
                RRuleProperty {
                    freq: String::from("MONTHLY"),
                    interval: 2,
                    count: Some(10),
                    wkst: Some(String::from("SU")),
                    until_utc_timestamp: Some(876182400),
                    by_second: Some(vec![String::from("1"), String::from("30")]),
                    by_minute: Some(vec![String::from("1"), String::from("30")]),
                    by_hour: Some(vec![String::from("1"), String::from("6")]),
                    by_day: Some(vec![String::from("-1SU"), String::from("2WE")]),
                    by_week_no: Some(vec![String::from("20")]),
                    by_month: Some(vec![String::from("3"), String::from("6")]),
                    by_month_day: Some(vec![String::from("7"), String::from("10")]),
                    by_year_day: Some(vec![
                        String::from("1"),
                        String::from("30"),
                        String::from("60"),
                    ]),
                    by_easter: Some(vec![String::from("-1"), String::from("3")]),
                    by_set_pos: Some(vec![String::from("3")]),

                    x_params: Some(HashMap::from([
                        (
                            String::from("X-TEST-KEY-TWO"),
                            vec![String::from("KEY -🎄- TWO")]
                        ),
                        (
                            String::from("X-TEST-KEY-ONE"),
                            vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                        ),
                    ])),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_property = RRuleProperty::parse_ical(
            r#"RRULE;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":FREQ=MONTHLY;INTERVAL=2;COUNT=10;WKST=SU;UNTIL=19971007T000000Z;BYSECOND=1,30;BYMINUTE=1,30;BYHOUR=1,6;BYDAY=-1SU,2WE;BYWEEKNO=20;BYMONTH=3,6;BYMONTHDAY=7,10;BYYEARDAY=1,30,60;BYEASTER=-1,3;BYSETPOS=3"#,
        ).unwrap().1;

        assert_eq!(
            parsed_property,
            RRuleProperty {
                freq: String::from("MONTHLY"),
                interval: 2,
                count: Some(10),
                wkst: Some(String::from("SU")),
                until_utc_timestamp: Some(876182400),
                by_second: Some(vec![String::from("1"), String::from("30")]),
                by_minute: Some(vec![String::from("1"), String::from("30")]),
                by_hour: Some(vec![String::from("1"), String::from("6")]),
                by_day: Some(vec![String::from("-1SU"), String::from("2WE")]),
                by_week_no: Some(vec![String::from("20")]),
                by_month: Some(vec![String::from("3"), String::from("6")]),
                by_month_day: Some(vec![String::from("7"), String::from("10")]),
                by_year_day: Some(vec![
                    String::from("1"),
                    String::from("30"),
                    String::from("60"),
                ]),
                by_easter: Some(vec![String::from("-1"), String::from("3")]),
                by_set_pos: Some(vec![String::from("3")]),

                x_params: Some(HashMap::from([
                    (
                        String::from("X-TEST-KEY-TWO"),
                        vec![String::from("KEY -🎄- TWO")]
                    ),
                    (
                        String::from("X-TEST-KEY-ONE"),
                        vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                    ),
                ])),
            },
        );

        let serialized_ical = parsed_property.serialize_to_ical();

        assert_eq!(
            RRuleProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"RRULE;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:BYDAY=-1SU,2WE;BYEASTER=-1,3;BYHOUR=1,6;BYMINUTE=1,30;BYMONTH=3,6;BYMONTHDAY=7,10;BYSECOND=1,30;BYSETPOS=3;BYWEEKNO=20;BYYEARDAY=1,30,60;COUNT=10;FREQ=MONTHLY;INTERVAL=2;UNTIL=19971007T000000Z;WKST=SU"#,
            ),
        );
    }
}
