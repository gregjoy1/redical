use nom::{
    error::{context, ParseError, ContextError},
    multi::separated_list1,
    sequence::{preceded, delimited, terminated},
    branch::alt,
    combinator::{cut, opt, recognize},
    bytes::complete::{take_while, take_while1, tag, tag_no_case, escaped},
    character::complete::{char, alphanumeric1, one_of, space1},
    IResult
};

#[derive(Debug)]
pub enum ParsedPropertyError {
    ParseError { message: String },
}

#[derive(Debug)]
pub struct ParsedProperty {
    /// Property name.
    pub name: String,
    /// Property list of parameters.
    pub params: Option<Vec<(String, Vec<String>)>>,
    /// Property value.
    pub value: Option<String>,
}

pub fn is_name_char(chr: u8) -> bool {
    (chr >= 0x30 && chr <= 0x39) ||
    (chr >= 0x41 && chr <= 0x5A) ||
    (chr >= 0x61 && chr <= 0x7A) ||
    chr == 0x2D
}

// iana-token    = 1*(ALPHA / DIGIT / "-")
// ; iCalendar identifier registered with IANA
fn iana_token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while1(is_name_char)(input)
}
// x-name        = "X-" [vendorid "-"] 1*(ALPHA / DIGIT / "-")
// ; Reserved for experimental use.
// vendorid      = 3*(ALPHA / DIGIT)
// ; Vendor identification
fn x_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(
        preceded(
            tag_no_case("X-"),
            separated_list1(char('-'), alphanumeric1)
        )
    )(input)
}

// name          = iana-token / x-name
fn name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt(
        (
            iana_token,
            x_name
        )
    )(input)
}


// param         = param-name "=" param-value *("," param-value)
// ; Each property defines the specific ABNF for the parameters
// ; allowed on the property.  Refer to specific properties for
// ; precise parameter ABNF.
fn param(input: &[u8]) -> IResult<&[u8], &[u8]> {
    recognize(
        preceded(
            param_name,
            preceded(
                char('='),
                separated_list1(
                    char(','),
                    param_value
                )
            )
        )
    )(input)
}

// param-name    = iana-token / x-name
fn param_name(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt(
        (
            iana_token,
            x_name
        )
    )(input)
}

// param-value   = paramtext / quoted-string
fn param_value(input: &[u8]) -> IResult<&[u8], &[u8]> {
    alt(
        (
            param_text,
            quoted_string
        )
    )(input)
}

// paramtext     = *SAFE-CHAR
fn param_text(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_safe_char)(input)
}

// value         = *VALUE-CHAR
fn value(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_value_char)(input)
}

// quoted-string = DQUOTE *QSAFE-CHAR DQUOTE
fn quoted_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    delimited(
        char('"'),
        quote_safe_char,
        char('"')
    )(input)
}

// QSAFE-CHAR    = WSP / %x21 / %x23-7E / NON-US-ASCII
// ; Any character except CONTROL and DQUOTE
fn is_quote_safe_char(chr: u8) -> bool {
    is_white_space_char(chr)         ||
    chr == 0x21                 ||
    (chr >= 0x23 && chr <=0x7E) ||
    is_non_us_ascii_char(chr)
}

fn quote_safe_char(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_quote_safe_char)(input)
}

// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E / NON-US-ASCII
// ; Any character except CONTROL, DQUOTE, ";", ":", ","
fn is_safe_char(chr: u8) -> bool {
    is_white_space_char(chr)          ||
    chr == 0x21                  ||
    (chr >= 0x23 && chr <= 0x2B) ||
    (chr >= 0x2D && chr <= 0x39) ||
    (chr >= 0x3C && chr <= 0x7E) ||
    is_non_us_ascii_char(chr)
}

// VALUE-CHAR    = WSP / %x21-7E / NON-US-ASCII
fn is_value_char(chr: u8) -> bool {
    is_white_space_char(chr) || is_ascii_char(chr) || is_non_us_ascii_char(chr)
}

fn is_white_space_char(chr: u8) -> bool {
    chr == 0x09 || // TAB
    chr == 0x0A || // LF
    chr == 0x0C || // FF
    chr == 0x0D || // CR
    chr == 0x20    // SPACE
}

// ; Any textual character
// %x21-7E
fn is_ascii_char(chr: u8) -> bool {
    chr >= 0x21 && chr <= 0x7E
}

// NON-US-ASCII  = UTF8-2 / UTF8-3 / UTF8-4
// ; UTF8-2, UTF8-3, and UTF8-4 are defined in [RFC3629]
fn is_non_us_ascii_char(chr: u8) -> bool {
    chr >= 0x80
}

// CONTROL       = %x00-08 / %x0A-1F / %x7F
fn is_control_char(chr: u8) -> bool {
    chr <= 0x08 || chr >= 0x0A && chr <= 0x1F || chr == 0x7F
}

fn colon_delimeter(input: &[u8]) -> IResult<&[u8], char> {
    char(':')(input)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn ical_property() {
        let data: &[u8] = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH".as_bytes();

        let (input, parsed_name) = name(data).unwrap();

        assert_eq!(
            String::from_utf8_lossy(parsed_name),
            String::from("RRULE")
        );

        let (input, delimeter) = colon_delimeter(input).unwrap();

        assert_eq!(
            delimeter,
            ':'
        );

        let(input, parsed_params) = separated_list1(char(';'), param)(input).unwrap();

        assert_eq!(
            parsed_params,
            vec![
                "FREQ=WEEKLY".as_bytes(),
                "UNTIL=20211231T183000Z".as_bytes(),
                "INTERVAL=1".as_bytes(),
                "BYDAY=TU,TH".as_bytes()
            ]
        );

        assert_eq!(
            input,
            "".as_bytes()
        );
    }
}
