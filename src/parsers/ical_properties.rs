use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::str;

use crate::data_types::KeyValuePair;

use nom::{
    error::{context, ParseError, ContextError, ErrorKind, VerboseError, VerboseErrorKind},
    multi::{separated_list0, separated_list1},
    sequence::{preceded, delimited, terminated, tuple, separated_pair},
    branch::alt,
    combinator::{cut, opt, recognize, map},
    bytes::complete::{take_while, take, take_while1, tag, tag_no_case, escaped},
    character::complete::{char, alphanumeric1, one_of, space1},
    number::complete::recognize_float,
    IResult
};

// ==============

use crate::parsers::ical_common;
use crate::parsers::ical_common::ParserResult;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ParsedProperty<'a> {
    #[serde(borrow)]
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

impl<'a> ParsedProperty<'a> {

    pub fn content_line(&self) -> &KeyValuePair {
        match self {
            ParsedProperty::Categories(parsed_property_content)  => { &parsed_property_content.content_line },
            ParsedProperty::RRule(parsed_property_content)       => { &parsed_property_content.content_line },
            ParsedProperty::ExRule(parsed_property_content)      => { &parsed_property_content.content_line },
            ParsedProperty::RDate(parsed_property_content)       => { &parsed_property_content.content_line },
            ParsedProperty::ExDate(parsed_property_content)      => { &parsed_property_content.content_line },
            ParsedProperty::Duration(parsed_property_content)    => { &parsed_property_content.content_line },
            ParsedProperty::DtStart(parsed_property_content)     => { &parsed_property_content.content_line },
            ParsedProperty::DtEnd(parsed_property_content)       => { &parsed_property_content.content_line },
            ParsedProperty::Description(parsed_property_content) => { &parsed_property_content.content_line },
            ParsedProperty::RelatedTo(parsed_property_content)   => { &parsed_property_content.content_line },
            ParsedProperty::Geo(parsed_property_content)         => { &parsed_property_content.content_line },
            ParsedProperty::Other(parsed_property_content)       => { &parsed_property_content.content_line }
        }
    }

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
                            ical_common::parse_property_parameters,
                            ical_common::colon_delimeter,
                            ical_common::ParsedValue::parse_single,
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

pub fn parse_properties(input: &str) -> ParserResult<&str, Vec<ParsedProperty>> {
    terminated(
        separated_list1(
            tag(" "),
            parse_property
        ),
        opt(tag(" ")),
    )(input)
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
                        ical_common::ParsedValue::parse_params,
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
                value: parsed_value,
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
                        ical_common::ParsedValue::parse_params,
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
                value: parsed_value,
                content_line: parsed_content_line
            };

            (remaining, parsed_property)
        }
    )
}

// TODO: parse exact date format
// https://www.kanzaki.com/docs/ical/dateTime.html
fn parse_rdate_property_content(input: &str) -> ParserResult<&str, ical_common::ParsedPropertyContent> {
    build_date_time_property_parser!("RDATE", input)
}

// TODO: parse exact date format
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
                        ical_common::ParsedValue::parse_single,
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
                        ical_common::parse_property_parameters,
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_single,
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
                        ical_common::parse_property_parameters,
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
                        ical_common::parse_property_parameters,
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
                                    ("FREQ", vec!["WEEKLY"]),
                                    ("UNTIL", vec!["20211231T183000Z"]),
                                    ("INTERVAL", vec!["1"]),
                                    ("BYDAY", vec!["TU","TH"])
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
                                    ("ALTREP", vec!["cid:part1.0001@example.org"]),
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
                                HashMap::from(
                                    [
                                        ("ALTREP", vec!["cid:part1.0001@example.org"]),
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
                    ),
                    ParsedProperty::RRule(
                        ical_common::ParsedPropertyContent {
                            name: Some("RRULE"),
                            params: None,
                            value: ical_common::ParsedValue::Params(
                                HashMap::from(
                                    [
                                        ("FREQ", vec!["WEEKLY"]),
                                        ("UNTIL", vec!["20211231T183000Z"]),
                                        ("INTERVAL", vec!["1"]),
                                        ("BYDAY", vec!["TU","TH"])
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
        let data: &str = "DTSTART:20201231T183000Z";

        assert_eq!(
            parse_dtstart_property_content(data).unwrap(),
            (
                "",
                ical_common::ParsedPropertyContent {
                    name: Some("DTSTART"),
                    params: None,
                    value: ical_common::ParsedValue::Single("20201231T183000Z"),
                    content_line: KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(":20201231T183000Z"),
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
                                ("FREQ", vec!["WEEKLY"]),
                                ("UNTIL", vec!["20211231T183000Z"]),
                                ("INTERVAL", vec!["1"]),
                                ("BYDAY", vec!["TU","TH"])
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
