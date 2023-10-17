use serde::{Serialize, Deserialize};
use std::option::Option;
use std::collections::HashMap;
use std::str;

use crate::data_types::KeyValuePair;

use nom::{
    error::{context, ParseError, ContextError, ErrorKind},
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

pub fn parse_properties(input: &str) -> IResult<&str, Vec<ParsedProperty>> {
    terminated(
        separated_list1(
            tag(" "),
            parse_property
        ),
        opt(tag(" ")),
    )(input)
}

fn parse_rrule_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("RRULE")(input)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value) = ical_common::ParsedValue::parse_params(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: None,
        value: parsed_value,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_exrule_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("EXRULE")(input)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value) = ical_common::ParsedValue::parse_params(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: None,
        value: parsed_value,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

// TODO: parse exact date format
// https://www.kanzaki.com/docs/ical/dateTime.html
fn parse_rdate_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("RDATE")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_list(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

// TODO: parse exact date format
// https://www.kanzaki.com/docs/ical/dateTime.html
fn parse_exdate_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("EXDATE")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_list(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

// TODO: parse exact duration format
// https://icalendar.org/iCalendar-RFC-5545/3-3-6-duration.html
// https://www.kanzaki.com/docs/ical/duration.html
fn parse_duration_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("DURATION")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_single(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

// TODO: parse exact datetime format
// https://www.kanzaki.com/docs/ical/dtstart.html
fn parse_dtstart_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("DTSTART")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_single(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

// TODO: parse exact datetime format
// https://www.kanzaki.com/docs/ical/dtend.html
fn parse_dtend_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("DTEND")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_single(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_description_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("DESCRIPTION")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value) = ical_common::ParsedValue::parse_single(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_categories_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("CATEGORIES")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_list(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_related_to_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("RELATED-TO")(input)?;

    let (remaining, parsed_params) = ical_common::parse_property_parameters(remaining)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_value_list) = ical_common::ParsedValue::parse_list(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value_list,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_geo_property_content(input: &str) -> IResult<&str, ical_common::ParsedPropertyContent> {
    let (remaining, parsed_name) = tag("GEO")(input)?;

    let (remaining, _) = ical_common::colon_delimeter(remaining)?;

    let (remaining, parsed_latitude) = recognize_float(remaining)?;

    let (remaining, _) = ical_common::semicolon_delimeter(remaining)?;

    let (remaining, parsed_longitude) = recognize_float(remaining)?;

    let parsed_content_line = ical_common::consumed_input_string(input, remaining, parsed_name);

    let parsed_value = ical_common::ParsedValue::List(vec![parsed_latitude, parsed_longitude]);

    let parsed_property = ical_common::ParsedPropertyContent {
        name: Some(parsed_name),
        params: None,
        value: parsed_value,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

fn parse_property(input: &str) -> IResult<&str, ParsedProperty> {
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
                        value: ical_common::ParsedValue::List(vec![
                            "37.386013",
                            "-122.082932",
                        ]),
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
    fn test_parse_rrule_property_content() {
        let data: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        assert_eq!(
            parse_rrule_property_content(data).unwrap(),
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
    }
}
