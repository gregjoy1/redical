use std::str;

use nom::{
    error::context,
    multi::separated_list1,
    sequence::{preceded, terminated, tuple, separated_pair},
    branch::alt,
    combinator::{cut, opt, map},
    bytes::complete::tag,
    character::complete::char,
    number::complete::recognize_float,
};

use crate::parsers::ical_common;
use crate::parsers::ical_common::ParserResult;

#[derive(Debug, PartialEq, Clone)]
pub enum ParsedProperty<'a> {
    Categories(ical_common::ParsedPropertyContent<'a>),
    RRule(ical_common::ParsedPropertyContent<'a>),
    ExRule(ical_common::ParsedPropertyContent<'a>),
    RDate(ical_common::ParsedPropertyContent<'a>),
    ExDate(ical_common::ParsedPropertyContent<'a>),
    Duration(ical_common::ParsedPropertyContent<'a>),
    DtStart(ical_common::ParsedPropertyContent<'a>),
    DtEnd(ical_common::ParsedPropertyContent<'a>),
    Description(ical_common::ParsedPropertyContent<'a>),
    RelatedTo(ical_common::ParsedPropertyContent<'a>),
    Geo(ical_common::ParsedPropertyContent<'a>),
    Other(ical_common::ParsedPropertyContent<'a>),
}

pub fn parse_properties(input: &str) -> ParserResult<&str, Vec<ParsedProperty>> {
    terminated(
        separated_list1(
            tag(" "),
            parse_property
        ),
        opt(tag(" ")),
    )(input)
}

macro_rules! build_date_time_property_parser {
    (
        $property_name:expr,
        $input_variable:ident
    ) => {
        preceded(
            tag($property_name),
            cut(
                context(
                    $property_name,
                    tuple(
                        (
                            build_property_params_parser!($property_name, ("TZID", ical_common::ParsedValue::parse_timezone)),
                            ical_common::colon_delimeter,
                            ical_common::ParsedValue::parse_date_string,
                        )
                    )
                )
            )
        )($input_variable).map(
            |(remaining, (parsed_params, _colon_delimeter, parsed_value))| {
                let parsed_content_line =
                    ical_common::consumed_input_string(
                        $input_variable,
                        remaining,
                        $property_name
                    );

                let parsed_property = ical_common::ParsedPropertyContent {
                    name: Some($property_name),
                    params: parsed_params,
                    value: parsed_value,
                    content_line: parsed_content_line
                };

                (remaining, parsed_property)
            }
        )
    }
}

macro_rules! build_property_params_parser {
    ($property_name:tt) => {
        opt(
            preceded(
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!($property_name)
            )
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        opt(
            preceded(
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!(
                    $property_name,
                    $(
                        ($param_name, $param_parser),
                    )+
                )
            )
        )
    }
}

macro_rules! build_property_params_value_parser {
    ($property_name:tt) => {
        context(
            "$property_name params",
            map(
                separated_list1(
                    ical_common::semicolon_delimeter,
                    context(
                        "param",
                        separated_pair(
                            ical_common::param_name,
                            char('='),
                            ical_common::param_value,
                        ),
                    ),
                ),
                |parsed_params| {
                    let params: HashMap<&str, ical_common::ParsedValue> =
                        parsed_params.into_iter()
                                     .map(|(key, value)| (key, value))
                                     .collect();

                    ical_common::ParsedValue::Params(params)
                }
            ),
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        context(
            concat!($property_name, " params"),
            map(
                separated_list1(
                    ical_common::semicolon_delimeter,
                    alt(
                        (
                            $(
                                context(
                                    concat!($property_name, " param"),
                                    separated_pair(
                                        tag($param_name),
                                        char('='),
                                        cut($param_parser),
                                    ),
                                ),
                            )+
                            context(
                                "param",
                                separated_pair(
                                    ical_common::param_name,
                                    char('='),
                                    ical_common::param_value,
                                ),
                            ),
                        ),
                    ),
                ),
                |parsed_params| {
                    parsed_params.into_iter()
                                 .map(|(key, value)| (key, value))
                                 .collect()
                }
            ),
        )
    }
}

fn parse_rrule_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("RRULE"),
        cut(
            context(
                "RRULE",
                tuple(
                    (
                        ical_common::colon_delimeter,
                        build_property_params_value_parser!(
                            "RRULE",
                            ("FREQ",       ical_common::ParsedValue::parse_single_param),
                            ("INTERVAL",   ical_common::ParsedValue::parse_single_param),
                            ("COUNT",      ical_common::ParsedValue::parse_single_param),
                            ("WKST",       ical_common::ParsedValue::parse_single_param),
                            ("UNTIL",      ical_common::ParsedValue::parse_date_string),
                            ("BYSECOND",   ical_common::ParsedValue::parse_list),
                            ("BYMINUTE",   ical_common::ParsedValue::parse_list),
                            ("BYHOUR",     ical_common::ParsedValue::parse_list),
                            ("BYDAY",      ical_common::ParsedValue::parse_list),
                            ("BYWEEKNO",   ical_common::ParsedValue::parse_list),
                            ("BYMONTH",    ical_common::ParsedValue::parse_list),
                            ("BYMONTHDAY", ical_common::ParsedValue::parse_list),
                            ("BYYEARDAY",  ical_common::ParsedValue::parse_list),
                            ("BYEASTER",   ical_common::ParsedValue::parse_list),
                            ("BYSETPOS",   ical_common::ParsedValue::parse_list),
                        )
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_colon_delimeter, parsed_value))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "RRULE"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("RRULE"),
                params: None,
                value: ical_common::ParsedValue::Params(parsed_value),
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

fn parse_exrule_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("EXRULE"),
        cut(
            context(
                "EXRULE",
                tuple(
                    (
                        ical_common::colon_delimeter,
                        build_property_params_value_parser!(
                            "RRULE",
                            ("FREQ",       ical_common::ParsedValue::parse_single_param),
                            ("INTERVAL",   ical_common::ParsedValue::parse_single_param),
                            ("COUNT",      ical_common::ParsedValue::parse_single_param),
                            ("WKST",       ical_common::ParsedValue::parse_single_param),
                            ("UNTIL",      ical_common::ParsedValue::parse_date_string),
                            ("BYSECOND",   ical_common::ParsedValue::parse_list),
                            ("BYMINUTE",   ical_common::ParsedValue::parse_list),
                            ("BYHOUR",     ical_common::ParsedValue::parse_list),
                            ("BYDAY",      ical_common::ParsedValue::parse_list),
                            ("BYWEEKNO",   ical_common::ParsedValue::parse_list),
                            ("BYMONTH",    ical_common::ParsedValue::parse_list),
                            ("BYMONTHDAY", ical_common::ParsedValue::parse_list),
                            ("BYYEARDAY",  ical_common::ParsedValue::parse_list),
                            ("BYEASTER",   ical_common::ParsedValue::parse_list),
                            ("BYSETPOS",   ical_common::ParsedValue::parse_list),
                        )
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_colon_delimeter, parsed_value))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "EXRULE"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("EXRULE"),
                params: None,
                value: ical_common::ParsedValue::Params(parsed_value),
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

// https://www.kanzaki.com/docs/ical/dateTime.html
fn parse_rdate_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    build_date_time_property_parser!("RDATE", input)
}

// https://www.kanzaki.com/docs/ical/dateTime.html
fn parse_exdate_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    build_date_time_property_parser!("EXDATE", input)
}

// TODO: parse exact duration format
// https://icalendar.org/iCalendar-RFC-5545/3-3-6-duration.html
// https://www.kanzaki.com/docs/ical/duration.html
fn parse_duration_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("DURATION"),
        cut(
            context(
                "DURATION",
                tuple(
                    (
                        ical_common::parse_property_parameters,
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_single_value,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (parsed_params, _colon_delimeter, parsed_value))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "DURATION"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("DURATION"),
                params: parsed_params,
                value: parsed_value,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

// TODO: parse exact datetime format
// https://www.kanzaki.com/docs/ical/dtstart.html
fn parse_dtstart_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    build_date_time_property_parser!("DTSTART", input)
}

// TODO: parse exact datetime format
// https://www.kanzaki.com/docs/ical/dtend.html
fn parse_dtend_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    build_date_time_property_parser!("DTEND", input)
}

fn parse_description_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("DESCRIPTION"),
        cut(
            context(
                "DESCRIPTION",
                tuple(
                    (
                        build_property_params_parser!(
                            "DESCRIPTION",
                            ("ALTREP",   ical_common::ParsedValue::parse_single_param),
                            ("LANGUAGE", ical_common::ParsedValue::parse_single_param),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_single_value,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (parsed_params, _colon_delimeter, parsed_value))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "DESCRIPTION"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("DESCRIPTION"),
                params: parsed_params,
                value: parsed_value,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

fn parse_categories_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("CATEGORIES"),
        cut(
            context(
                "CATEGORIES",
                tuple(
                    (
                        build_property_params_parser!(
                            "CATEGORIES",
                            ("LANGUAGE", ical_common::ParsedValue::parse_single_param),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_list,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (parsed_params, _colon_delimeter, parsed_value_list))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "CATEGORIES"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("CATEGORIES"),
                params: parsed_params,
                value: parsed_value_list,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

fn parse_related_to_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("RELATED-TO"),
        cut(
            context(
                "RELATED-TO",
                tuple(
                    (
                        build_property_params_parser!(
                            "RELATED-TO",
                            ("RELTYPE", ical_common::ParsedValue::parse_single_param),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_list,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (parsed_params, _colon_delimeter, parsed_value_list))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "RELATED-TO"
                );

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("RELATED-TO"),
                params: parsed_params,
                value: parsed_value_list,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

fn parse_geo_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    preceded(
        tag("GEO"),
        cut(
            context(
                "GEO",
                tuple(
                    (
                        ical_common::colon_delimeter,
                        recognize_float,
                        ical_common::semicolon_delimeter,
                        recognize_float,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_colon_delimeter, parsed_latitude, _semicolon_delimeter, parsed_longitude))| {
            let parsed_content_line =
                ical_common::consumed_input_string(
                    input,
                    remaining,
                    "GEO"
                );

            let parsed_value_pair = ical_common::ParsedValue::Pair((parsed_latitude, parsed_longitude));

            let parsed_property = ical_common::ParsedPropertyContent {
                name: Some("GEO"),
                params: None,
                value: parsed_value_pair,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

fn parse_property(input: &str) -> ParserResult<&str, ParsedProperty> {
    // println!("parse_property - input - {input}");
    alt(
        (
            map(parse_rrule_property_content,        ParsedProperty::RRule),
            map(parse_exrule_property_content,       ParsedProperty::ExRule),
            map(parse_rdate_property_content,        ParsedProperty::RDate),
            map(parse_exdate_property_content,       ParsedProperty::ExDate),
            map(parse_duration_property_content,     ParsedProperty::Duration),
            map(parse_dtstart_property_content,      ParsedProperty::DtStart),
            map(parse_dtend_property_content,        ParsedProperty::DtEnd),
            map(parse_description_property_content,  ParsedProperty::Description),
            map(parse_categories_property_content,   ParsedProperty::Categories),
            map(parse_related_to_property_content,   ParsedProperty::RelatedTo),
            map(parse_geo_property_content,          ParsedProperty::Geo),
            map(ical_common::parse_property_content, ParsedProperty::Other),
        )
    )(input)
}

#[cfg(test)]
mod test {
    use super::*;

    use nom::error::{VerboseError, VerboseErrorKind, ErrorKind};

    use crate::data_types::KeyValuePair;
    use crate::parsers::ical_common::ParsedValue;
    use crate::parsers::datetime::{ParsedDateString, ParsedDateStringTime, ParsedDateStringFlags};

    use std::collections::HashMap;

    #[test]
    fn test_parse_property() {
        let data: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        assert_eq!(
            parse_property(data).unwrap(),
            (
                "",
                ParsedProperty::RRule(
                    ical_common::ParsedPropertyContent {
                        name: Some("RRULE"),
                        params: None,
                        value: ical_common::ParsedValue::Params(
                            HashMap::from(
                                [
                                    ("FREQ", ParsedValue::Single("WEEKLY")),
                                    ("INTERVAL", ParsedValue::Single("1")),
                                    ("BYDAY", ParsedValue::List(vec!["TU","TH"])),

                                    (
                                        "UNTIL", 
                                        ParsedValue::DateString(
                                            ParsedDateString {
                                                year: 2021,
                                                month: 12,
                                                day: 31,
                                                time: Some(ParsedDateStringTime {
                                                    hour: 18,
                                                    min: 30,
                                                    sec: 0,
                                                }),
                                                flags: ParsedDateStringFlags {
                                                    zulu_timezone_set: true,
                                                },
                                                dt: "20211231T183000Z".to_string(),
                                            },
                                        ),
                                    ),
                                ]
                            )
                        ),
                        content_line: KeyValuePair::new(
                            String::from("RRULE"),
                            String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                        )
                    }
                )
            )
        );

        let data: &str = "GEO:37.386013;-122.082932";

        assert_eq!(
            parse_property(data).unwrap(),
            (
                "",
                ParsedProperty::Geo(
                    ical_common::ParsedPropertyContent {
                        name: Some("GEO"),
                        params: None,
                        value: ical_common::ParsedValue::Pair(
                            (
                                "37.386013",
                                "-122.082932",
                            )
                        ),
                        content_line: KeyValuePair::new(
                            String::from("GEO"),
                            String::from(":37.386013;-122.082932"),
                        )
                    }
                )
            )
        );

        let data: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA";

        assert_eq!(
            parse_property(data).unwrap(),
            (
                "",
                ParsedProperty::Description(
                    ical_common::ParsedPropertyContent {
                        name: Some("DESCRIPTION"),
                        params: Some(
                            HashMap::from(
                                [
                                    ("ALTREP", ParsedValue::Single("cid:part1.0001@example.org")),
                                ]
                            )
                        ),
                        value: ical_common::ParsedValue::Single(
                            "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                        ),
                        content_line: KeyValuePair::new(
                            String::from("DESCRIPTION"),
                            String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"),
                        )
                    }
                )
            )
        );

        let data: &str = "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            parse_property(data).unwrap(),
            (
                "",
                ParsedProperty::Categories(
                    ical_common::ParsedPropertyContent {
                        name: Some("CATEGORIES"),
                        params: None,
                        value: ical_common::ParsedValue::List(
                            vec![
                                "CATEGORY_ONE",
                                "CATEGORY_TWO",
                                "CATEGORY THREE",
                            ]
                        ),
                        content_line: KeyValuePair::new(
                            String::from("CATEGORIES"),
                            String::from(":CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\""),
                        )
                    }
                )
            )
        );
    }

    fn test_parse_properties() {
        let data: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            parse_properties(data).unwrap(),
            (
                "",
                vec![
                    ParsedProperty::Description(
                        ical_common::ParsedPropertyContent {
                            name: Some("DESCRIPTION"),
                            params: Some(
                                HashMap::from([
                                    ("ALTREP", ParsedValue::Single("cid:part1.0001@example.org")),
                                ])
                            ),
                            value: ical_common::ParsedValue::Single(
                                "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                            ),
                            content_line: KeyValuePair::new(
                                String::from("DESCRIPTION"),
                                String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"),
                            )
                        }
                    ),
                    ParsedProperty::RRule(
                        ical_common::ParsedPropertyContent {
                            name: Some("RRULE"),
                            params: None,
                            value: ical_common::ParsedValue::Params(
                                HashMap::from(
                                    [
                                        ("FREQ", ParsedValue::Single("WEEKLY")),
                                        ("UNTIL", ParsedValue::Single("20211231T183000Z")),
                                        ("INTERVAL", ParsedValue::Single("1")),
                                        ("BYDAY", ParsedValue::List(vec!["TU","TH"])),
                                    ]
                                )
                            ),
                            content_line: KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                            )
                        }
                    ),
                    ParsedProperty::Categories(
                        ical_common::ParsedPropertyContent {
                            name: Some("CATEGORIES"),
                            params: None,
                            value: ical_common::ParsedValue::List(
                                vec![
                                    "CATEGORY_ONE",
                                    "CATEGORY_TWO",
                                    "CATEGORY THREE",
                                ]
                            ),
                            content_line: KeyValuePair::new(
                                String::from("CATEGORIES"),
                                String::from(":CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\""),
                            )
                        }
                    )
                ]
            )
        );
    }

    #[test]
    fn test_parse_dtstart_property_content() {
        assert_eq!(
            parse_dtstart_property_content("DTSTART:20201231T183000Z").unwrap(),
            (
                "",
                ical_common::ParsedPropertyContent {
                    name: Some("DTSTART"),
                    params: None,
                    value: ical_common::ParsedValue::DateString(
                        ParsedDateString {
                            year: 2020,
                            month: 12,
                            day: 31,
                            time: Some(
                                ParsedDateStringTime {
                                    hour: 18,
                                    min: 30,
                                    sec: 0,
                                }
                            ),
                            flags: ParsedDateStringFlags {
                                zulu_timezone_set: true
                            },
                            dt: "20201231T183000Z".to_string()
                        },
                    ),
                    content_line: KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(":20201231T183000Z"),
                    )
                }
            )
        );

        assert_eq!(
            parse_dtstart_property_content("DTSTART;TZID=Europe/London:20201231T183000").unwrap(),
            (
                "",
                ical_common::ParsedPropertyContent {
                    name: Some("DTSTART"),
                    params: Some(
                        HashMap::from([
                            ("TZID", ParsedValue::TimeZone(rrule::Tz::Europe__London)),
                        ])
                    ),
                    value: ical_common::ParsedValue::DateString(
                        ParsedDateString {
                            year: 2020,
                            month: 12,
                            day: 31,
                            time: Some(
                                ParsedDateStringTime {
                                    hour: 18,
                                    min: 30,
                                    sec: 0,
                                }
                            ),
                            flags: ParsedDateStringFlags {
                                zulu_timezone_set: false
                            },
                            dt: "20201231T183000".to_string()
                        },
                    ),
                    content_line: KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(";TZID=Europe/London:20201231T183000"),
                    )
                }
            )
        );

        assert_eq!(
            parse_dtstart_property_content("DTSTART;TZID=Europe/London:20201231T183000Z").unwrap(),
            (
                "",
                ical_common::ParsedPropertyContent {
                    name: Some("DTSTART"),
                    params: Some(
                        HashMap::from([
                            ("TZID", ParsedValue::TimeZone(rrule::Tz::Europe__London)),
                        ])
                    ),
                    value: ical_common::ParsedValue::DateString(
                        ParsedDateString {
                            year: 2020,
                            month: 12,
                            day: 31,
                            time: Some(
                                ParsedDateStringTime {
                                    hour: 18,
                                    min: 30,
                                    sec: 0,
                                }
                            ),
                            flags: ParsedDateStringFlags {
                                zulu_timezone_set: true
                            },
                            dt: "20201231T183000Z".to_string()
                        },
                    ),
                    content_line: KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(";TZID=Europe/London:20201231T183000Z"),
                    )
                }
            )
        );
    }

    #[test]
    fn test_parse_rrule_property_content() {
        // Testing valid RRULE
        assert_eq!(
            parse_rrule_property_content("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH").unwrap(),
            (
                "",
                ical_common::ParsedPropertyContent {
                    name: Some("RRULE"),
                    params: None,
                    value: ical_common::ParsedValue::Params(
                        HashMap::from(
                            [
                                ("FREQ", ParsedValue::Single("WEEKLY")),
                                ("INTERVAL", ParsedValue::Single("1")),
                                ("BYDAY", ParsedValue::List(vec!["TU","TH"])),

                                (
                                    "UNTIL", 
                                    ParsedValue::DateString(
                                        ParsedDateString {
                                            year: 2021,
                                            month: 12,
                                            day: 31,
                                            time: Some(ParsedDateStringTime {
                                                hour: 18,
                                                min: 30,
                                                sec: 0,
                                            }),
                                            flags: ParsedDateStringFlags {
                                                zulu_timezone_set: true,
                                            },
                                            dt: "20211231T183000Z".to_string(),
                                        },
                                    ),
                                ),
                            ]
                        )
                    ),
                    content_line: KeyValuePair::new(
                        String::from("RRULE"),
                        String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                    )
                }
            )
        );

        // Testing invalid RRULE
        assert_eq!(
            parse_rrule_property_content("RRULE;FREQ=WEEKLY;SOMETHING,ELSE"),
            Err(
                nom::Err::Failure(
                    VerboseError {
                        errors: vec![
                            (
                                ";FREQ=WEEKLY;SOMETHING,ELSE", VerboseErrorKind::Nom(ErrorKind::Tag),
                            ),
                            (
                                ";FREQ=WEEKLY;SOMETHING,ELSE", VerboseErrorKind::Context("RRULE"),
                            )
                        ]
                    }
                )
            )
        );
    }
}
