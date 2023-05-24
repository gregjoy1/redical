use std::option::Option;
use std::collections::HashMap;
use std::str;

use nom::{
    error::{context, ParseError, ContextError, ErrorKind},
    multi::{separated_list0, separated_list1},
    sequence::{preceded, delimited, terminated, tuple, separated_pair},
    branch::alt,
    combinator::{cut, opt, recognize, map},
    bytes::complete::{take_while, take_while1, tag, tag_no_case, escaped},
    character::complete::{char, alphanumeric1, one_of, space1},
    IResult
};

#[derive(Debug)]
pub enum ParsedPropertyError<'a> {
    ParseError { message: &'a str },
}

#[derive(Debug)]
pub struct ParsedProperty<'a> {
    /// Property name.
    pub name: Option<&'a str>,

    /// HashMap of parameters (before : delimiter).
    pub params: Option<HashMap<&'a str, Vec<&'a str>>>,

    /// HashMap of value parameters (after : delimiter).
    pub value_params: Option<HashMap<&'a str, Vec<&'a str>>>,

    /// Property value.
    pub value: Option<Vec<&'a str>>,
}

impl PartialEq for ParsedProperty<'_> {

    fn eq(&self, other: &Self) -> bool {
        self.name == other.name &&
        self.params == other.params &&
        self.value_params == other.value_params &&
        self.value == other.value
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
fn iana_token(input: &str) -> IResult<&str, &str> {
    take_while1(is_name_char)(input)
}
// x-name        = "X-" [vendorid "-"] 1*(ALPHA / DIGIT / "-")
// ; Reserved for experimental use.
// vendorid      = 3*(ALPHA / DIGIT)
// ; Vendor identification
fn x_name(input: &str) -> IResult<&str, &str> {
    recognize(
        preceded(
            tag_no_case("X-"),
            separated_list1(char('-'), alphanumeric1)
        )
    )(input)
}

// name          = iana-token / x-name
fn name(input: &str) -> IResult<&str, &str> {
    alt(
        (
            iana_token,
            x_name
        )
    )(input)
}

fn params(input: &str) -> IResult<&str, HashMap<&str, Vec<&str>>> {
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
fn param(input: &str) -> IResult<&str, (&str, Vec<&str>)> {
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
fn param_name(input: &str) -> IResult<&str, &str> {
    alt(
        (
            iana_token,
            x_name
        )
    )(input)
}

// param-value   = paramtext / quoted-string
fn param_value(input: &str) -> IResult<&str, &str> {
    alt(
        (
            param_text,
            quoted_string
        )
    )(input)
}

// paramtext     = *SAFE-CHAR
fn param_text(input: &str) -> IResult<&str, &str> {
    take_while1(is_safe_char)(input)
}

fn values(input: &str) -> IResult<&str, Vec<&str>> {
    separated_list1(
        char(','),
        value
    )(input)
}

// value         = *VALUE-CHAR
fn value(input: &str) -> IResult<&str, &str> {
    take_while1(is_value_char)(input)
}

// quoted-string = DQUOTE *QSAFE-CHAR DQUOTE
fn quoted_string(input: &str) -> IResult<&str, &str> {
    delimited(
        char('"'),
        quote_safe_char,
        char('"')
    )(input)
}

// QSAFE-CHAR    = WSP / %x21 / %x23-7E / NON-US-ASCII
// ; Any character except CONTROL and DQUOTE
fn is_quote_safe_char(chr: char) -> bool {
    is_white_space_char(chr)    ||
    chr == '\x21'                 ||
    (chr >= '\x23' && chr <='\x7E') ||
    is_non_us_ascii_char(chr)
}

fn quote_safe_char(input: &str) -> IResult<&str, &str> {
    take_while(is_quote_safe_char)(input)
}

// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E / NON-US-ASCII
// ; Any character except CONTROL, DQUOTE, ";", ":", ","
fn is_safe_char(chr: char) -> bool {
    is_white_space_char(chr)         ||
    chr == '\x21'                    ||
    (chr >= '\x23' && chr <= '\x2B') ||
    (chr >= '\x2D' && chr <= '\x39') ||
    (chr >= '\x3C' && chr <= '\x7E') ||
    is_non_us_ascii_char(chr)
}

// VALUE-CHAR    = WSP / %x21-7E / NON-US-ASCII
fn is_value_char(chr: char) -> bool {
    is_white_space_char(chr) || is_ascii_char(chr) || is_non_us_ascii_char(chr)
}

fn is_white_space_char(chr: char) -> bool {
    chr == '\x09' || // TAB
    chr == '\x0A' || // LF
    chr == '\x0C' || // FF
    chr == '\x0D' || // CR
    chr == '\x20'    // SPACE
}

// ; Any textual character
// %x21-7E
fn is_ascii_char(chr: char) -> bool {
    chr >= '\x21' && chr <= '\x7E'
}

// NON-US-ASCII  = UTF8-2 / UTF8-3 / UTF8-4
// ; UTF8-2, UTF8-3, and UTF8-4 are defined in [RFC3629]
fn is_non_us_ascii_char(chr: char) -> bool {
    chr > '\x7f'
}

// CONTROL       = %x00-08 / %x0A-1F / %x7F
fn is_control_char(chr: char) -> bool {
    chr <= '\x08' || chr >= '\x0A' && chr <= '\x1F' || chr == '\x7F'
}

fn is_colon_delimeter(chr: char) -> bool {
    chr == '\x3A'
}

fn is_semicolon_delimeter(chr: char) -> bool {
    chr == '\x3B'
}

fn colon_delimeter(input: &str) -> IResult<&str, &str> {
    tag(":")(input)
}

fn semicolon_delimeter(input: &str) -> IResult<&str, &str> {
    tag(";")(input)
}

fn parse_property_parameters(input: &str) -> IResult<&str, Option<HashMap<&str, Vec<&str>>>> {
    opt(
        preceded(
            semicolon_delimeter,
            params
        )
    )(input)
}

fn parse_property(input: &str) -> IResult<&str, ParsedProperty> {
    let (remaining, parsed_name) = name(input)?;

    let (remaining, parsed_params) = parse_property_parameters(remaining)?;

    let (remaining, _) = colon_delimeter(remaining)?;

    let (remaining, parsed_value_params) = opt(params)(remaining)?;

    let (remaining, parsed_value) = opt(values)(remaining)?;

    let parsed_property = ParsedProperty {
        name: Some(parsed_name),
        params: parsed_params,
        value_params: parsed_value_params,
        value: parsed_value,
    };

    Ok((remaining, parsed_property))
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
                ParsedProperty {
                    name: Some("RRULE"),
                    params: None,
                    value_params: Some(
                        HashMap::from(
                            [
                                ("FREQ", vec!["WEEKLY"]),
                                ("UNTIL", vec!["20211231T183000Z"]),
                                ("INTERVAL", vec!["1"]),
                                ("BYDAY", vec!["TU","TH"])
                            ]
                        )
                    ),
                    value: None
                }
            )
        );

        assert_eq!(
            recognize(parse_property)(data).unwrap(),
            (
                "",
                "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
            )
        );

        let data: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA";

        assert_eq!(
            parse_property(data).unwrap(),
            (
                "",
                ParsedProperty {
                    name: Some("DESCRIPTION"),
                    params: Some(
                        HashMap::from(
                            [
                                ("ALTREP", vec!["cid:part1.0001@example.org"]),
                            ]
                        )
                    ),
                    value_params: None,
                    value: Some(
                        vec![
                            "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                        ]
                    )
                }
            )
        );

        assert_eq!(
            recognize(parse_property)(data).unwrap(),
            (
                "",
                "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
            )
        );
    }
}
