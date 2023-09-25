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
    IResult
};

#[derive(Debug)]
pub enum ParsedPropertyContentError<'a> {
    ParseError { message: &'a str },
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ParsedPropertyContent<'a> {
    /// Property name.
    pub name: Option<&'a str>,

    /// HashMap of parameters (before : delimiter).
    pub params: Option<HashMap<&'a str, Vec<&'a str>>>,

    /// Property value.
    pub value: ParsedValue<'a>,

    /// The whole property content line.
    pub content_line: KeyValuePair,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum ParsedValue<'a> {
    List(Vec<&'a str>),
    Single(&'a str),
    Params(HashMap<&'a str, Vec<&'a str>>)
}

impl<'a> ParsedValue<'a> {

    pub fn parse_list(input: &'a str) -> IResult<&'a str, Self> {
        let (remaining, parsed_value_list) = separated_list1(
            char(','),
            alt(
                (
                    param_text,
                    quoted_string
                )
            )
        )(input)?;

        Ok(
            (
                remaining,
                Self::List(parsed_value_list)
            )
        )
    }

    pub fn parse_single(input: &'a str) -> IResult<&'a str, Self> {
        let (remaining, parsed_single_value) = value(input)?;

        Ok(
            (
                remaining,
                Self::Single(parsed_single_value)
            )
        )
    }

    pub fn parse_params(input: &'a str) -> IResult<&'a str, Self> {
        let (remaining, parsed_param_value) = params(input)?;

        Ok(
            (
                remaining,
                Self::Params(parsed_param_value)
            )
        )
    }
}

pub fn is_name_char(chr: char) -> bool {
    (chr >= '\x30' && chr <= '\x39') ||
    (chr >= '\x41' && chr <= '\x5A') ||
    (chr >= '\x61' && chr <= '\x7A') ||
    chr == '\x2D'
}

// iana-token    = 1*(ALPHA / DIGIT / "-")
// ; iCalendar identifier registered with IANA
pub fn iana_token(input: &str) -> IResult<&str, &str> {
    take_while1(is_name_char)(input)
}

// x-name        = "X-" [vendorid "-"] 1*(ALPHA / DIGIT / "-")
// ; Reserved for experimental use.
// vendorid      = 3*(ALPHA / DIGIT)
// ; Vendor identification
pub fn x_name(input: &str) -> IResult<&str, &str> {
    recognize(
        preceded(
            tag_no_case("X-"),
            separated_list1(char('-'), alphanumeric1)
        )
    )(input)
}

// name          = iana-token / x-name
pub fn name(input: &str) -> IResult<&str, &str> {
    preceded(
        take_while(is_white_space_char),
        alt(
            (
                iana_token,
                x_name
            )
        )
    )(input)
}

pub fn params(input: &str) -> IResult<&str, HashMap<&str, Vec<&str>>> {
    map(
        separated_list1(semicolon_delimeter, param),
        |tuple_vec| {
            tuple_vec.into_iter()
                     .map(|(key, value)| (key, value))
                     .collect()
        }
    )(input)
}

// param         = param-name "=" param-value *("," param-value)
// ; Each property defines the specific ABNF for the parameters
// ; allowed on the property.  Refer to specific properties for
// ; precise parameter ABNF.
pub fn param(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
    separated_pair(
        param_name,
        char('='),
        separated_list1(
            char(','),
            param_value
        )
    )(input)
}

// param-name    = iana-token / x-name
pub fn param_name(input: &str) -> IResult<&str, &str> {
    alt(
        (
            iana_token,
            x_name
        )
    )(input)
}

// param-value   = paramtext / quoted-string
pub fn param_value(input: &str) -> IResult<&str, &str> {
    alt(
        (
            param_text,
            quoted_string
        )
    )(input)
}

// paramtext     = *SAFE-CHAR
pub fn param_text(input: &str) -> IResult<&str, &str> {
    let next_property_index: usize = match find_next_property_in_unquoted_value(input) {
        Some(found_property_index) => {
            // println!("param_text -- found_property_index - {found_property_index} -- {:#?}", &input[..=found_property_index]);
            found_property_index
        },
        None => {
            // println!("param_text -- not found_property_index -- {:#?}", input);
            input.len()
        }
    };

    let (_remaining, extracted_param_text) = take_while1(is_safe_char)(input)?;

    let extracted_param_text_index = extracted_param_text.len();

    let split_at_index = std::cmp::min(next_property_index, extracted_param_text_index);

    // println!("param_text - min - {split_at_index} - next_property_index {next_property_index} - extracted_param_text_index {extracted_param_text_index} - input: {input}");

    let result = input.split_at(split_at_index);

    Ok((result.1, result.0))
}

pub fn values(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list1(
        char(','),
        value
    )(input)
}

// value         = *VALUE-CHAR
pub fn value(input: &str) -> IResult<&str, &str> {
    let next_property_index: usize = match find_next_property_in_unquoted_value(input) {
        Some(found_property_index) => {
            // println!("value -- found_property_index - {found_property_index} -- {:#?}", &input[..=found_property_index]);
            found_property_index
        },
        None => {
            // println!("value -- not found_property_index -- {:#?}", input);
            input.len()
        }
    };

    let (_remaining, extracted_value) = take_while1(is_value_char)(input)?;

    let extracted_value_index = extracted_value.len();

    let split_at_index = std::cmp::min(next_property_index, extracted_value_index);

    // println!("value - min - {split_at_index} - next_property_index {next_property_index} - extracted_value_index {extracted_value_index} - input: {input}");

    let result = input.split_at(split_at_index);

    Ok((result.1, result.0))
}

// quoted-string = DQUOTE *QSAFE-CHAR DQUOTE
pub fn quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(
        char('"'),
        quote_safe_char,
        char('"')
    )(input)
}

// QSAFE-CHAR    = WSP / %x21 / %x23-7E / NON-US-ASCII
// ; Any character except CONTROL and DQUOTE
pub fn is_quote_safe_char(chr: char) -> bool {
    is_white_space_char(chr)    ||
    chr == '\x21'                 ||
    (chr >= '\x23' && chr <='\x7E') ||
    is_non_us_ascii_char(chr)
}

pub fn quote_safe_char(input: &str) -> IResult<&str, &str> {
    take_while(is_quote_safe_char)(input)
}

// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E / NON-US-ASCII
// ; Any character except CONTROL, DQUOTE, ";", ":", ","
pub fn is_safe_char(chr: char) -> bool {
    is_white_space_char(chr)         ||
    chr == '\x21'                    ||
    (chr >= '\x23' && chr <= '\x2B') ||
    (chr >= '\x2D' && chr <= '\x39') ||
    (chr >= '\x3C' && chr <= '\x7E') ||
    is_non_us_ascii_char(chr)
}

// VALUE-CHAR    = WSP / %x21-7E / NON-US-ASCII
pub fn is_value_char(chr: char) -> bool {
    is_white_space_char(chr) || is_ascii_char(chr) || is_non_us_ascii_char(chr)
}

pub fn white_space(input: &str) -> IResult<&str, &str> {
    take_while(is_white_space_char)(input)
}

pub fn is_white_space_char(chr: char) -> bool {
    chr == '\x09' || // TAB
    chr == '\x0A' || // LF
    chr == '\x0C' || // FF
    chr == '\x0D' || // CR
    chr == '\x20'    // SPACE
}

// ; Any textual character
// %x21-7E
pub fn is_ascii_char(chr: char) -> bool {
    chr >= '\x21' && chr <= '\x7E'
}

// NON-US-ASCII  = UTF8-2 / UTF8-3 / UTF8-4
// ; UTF8-2, UTF8-3, and UTF8-4 are defined in [RFC3629]
pub fn is_non_us_ascii_char(chr: char) -> bool {
    chr > '\x7f'
}

// CONTROL       = %x00-08 / %x0A-1F / %x7F
pub fn is_control_char(chr: char) -> bool {
    chr <= '\x08' || chr >= '\x0A' && chr <= '\x1F' || chr == '\x7F'
}

pub fn is_colon_delimeter(chr: char) -> bool {
    chr == '\x3A'
}

pub fn is_semicolon_delimeter(chr: char) -> bool {
    chr == '\x3B'
}

pub fn colon_delimeter(input: &str) -> IResult<&str, &str> {
    tag(":")(input)
}

pub fn semicolon_delimeter(input: &str) -> IResult<&str, &str> {
    tag(";")(input)
}

pub fn parse_property_parameters(input: &str) -> IResult<&str, Option<HashMap<&str, Vec<&str>>>> {
    opt(
        preceded(
            semicolon_delimeter,
            params
        )
    )(input)
}

pub fn consumed_input_string<'a>(original_input: &'a str, remaining_input: &'a str, property_name: &'a str) -> KeyValuePair {
    let consumed_input = original_input.len() - remaining_input.len();

    KeyValuePair::new(
        property_name.to_string(),
        original_input[property_name.len()..consumed_input].to_string(),
    )
}

pub fn parse_property_content(input: &str) -> IResult<&str, ParsedPropertyContent> {
    let (remaining, parsed_name) = name(input)?;

    let (remaining, parsed_params) = parse_property_parameters(remaining)?;

    let (remaining, _) = colon_delimeter(remaining)?;

    let (remaining, parsed_value) = ParsedValue::parse_single(remaining)?;

    let parsed_content_line = consumed_input_string(input, remaining, parsed_name);

    let parsed_property = ParsedPropertyContent {
        name: Some(parsed_name),
        params: parsed_params,
        value: parsed_value,
        content_line: parsed_content_line
    };

    Ok((remaining, parsed_property))
}

pub fn take_until_next_property(input: &str) -> IResult<&str, &str> {
    match find_next_property_in_unquoted_value(input) {
        Some(found_property_index) => {
            take(found_property_index)(input)
        },
        None => {
            Ok(("", input))
        }
    }
}

pub fn find_next_property_in_unquoted_value(input: &str) -> Option<usize> {
    enum State {
        Unset,
        WhiteSpace,
        NameChar
    }

    let mut state = State::Unset;

    let mut found_index = None;

    for (index, input_char) in input.chars().enumerate() {
        match state {
            State::Unset if is_white_space_char(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::Unset if is_white_space_char");
                state = State::WhiteSpace;
                found_index = Some(index);
            },

            State::Unset => {
                // println!("index - {index} input_char - {input_char} -- State::Unset");
                state = State::Unset;
                found_index = None;
            },

            State::WhiteSpace if is_white_space_char(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::WhiteSpace if is_white_space_char");
            },

            State::WhiteSpace if is_name_char(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::WhiteSpace if is_name_char");
                state = State::NameChar;
            },

            State::WhiteSpace => {
                // println!("index - {index} input_char - {input_char} -- State::WhiteSpace");
                state = State::Unset;
                found_index = None;
            },

            State::NameChar if ";:".contains(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::NameChar if ;:");

                match parse_property_content(&input[found_index.unwrap()..]) {
                    Ok(_) => {
                        // println!("index - {index} input_char - {input_char} -- State::NameChar if ;: -- parsed_property_content Ok -- {:#?}", &input[found_index.unwrap()..]);
                        return found_index;
                    },
                    Err(_) => {
                        // println!("index - {index} input_char - {input_char} -- State::NameChar if ;: -- parsed_property_content Err -- {:#?}", &input[found_index.unwrap()..]);
                    }
                }

                break;
            },

            State::NameChar if is_white_space_char(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::NameChar if is_white_space_char");
                state = State::WhiteSpace;
                found_index = Some(index);
            },

            State::NameChar if is_name_char(input_char) => {
                // println!("index - {index} input_char - {input_char} -- State::NameChar if is_name_char");
            },

            State::NameChar => {
                // println!("index - {index} input_char - {input_char} -- State::NameChar");
                state = State::Unset;
                found_index = None;
            }
        }
    }

    None
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_next_property_in_unquoted_value() {
        let data: &str = "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA; DETAILS: Some random facts...";

        assert_eq!(find_next_property_in_unquoted_value(data), None);

        let data: &str = "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        assert_eq!(find_next_property_in_unquoted_value(data), Some(58));
    }

    #[test]
    fn test_parsed_value_enum() {
        let data: &str = "MO,TU,TH";

        assert_eq!(
            ParsedValue::parse_list(data).unwrap(),
            (
                "",
                ParsedValue::List(
                    vec![
                        "MO",
                        "TU",
                        "TH"
                    ]
                )
            )
        );

        assert_eq!(
            ParsedValue::parse_single(data).unwrap(),
            (
                "",
                ParsedValue::Single(
                    "MO,TU,TH"
                )
            )
        );
    }

    #[test]
    fn test_parse_property_content() {
        let data: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        assert_eq!(
            parse_property_content(data).unwrap(),
            (
                "",
                ParsedPropertyContent {
                    name: Some("RRULE"),
                    params: None,
                    value: ParsedValue::Single(
                        "FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
                    ),
                    content_line: KeyValuePair::new(
                        String::from("RRULE"),
                        String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                    )
                }
            )
        );

        assert_eq!(
            recognize(parse_property_content)(data).unwrap(),
            (
                "",
                "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
            )
        );

        let data: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA";

        assert_eq!(
            parse_property_content(data).unwrap(),
            (
                "",
                ParsedPropertyContent {
                    name: Some("DESCRIPTION"),
                    params: Some(
                        HashMap::from(
                            [
                                ("ALTREP", vec!["cid:part1.0001@example.org"]),
                            ]
                        )
                    ),
                    value: ParsedValue::Single(
                        "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                    ),
                    content_line: KeyValuePair::new(
                        String::from("DESCRIPTION"),
                        String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"),
                    )
                }
            )
        );

        assert_eq!(
            recognize(parse_property_content)(data).unwrap(),
            (
                "",
                "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
            )
        );
    }
}
