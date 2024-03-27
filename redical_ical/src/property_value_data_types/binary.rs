use nom::sequence::{tuple, pair};
use nom::branch::alt;
use nom::error::context;
use nom::combinator::{recognize, map};
use nom::bytes::complete::{tag, take_while_m_n};
use nom::character::{is_alphabetic, is_digit};
use nom::multi::{many0, many_m_n};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};
use crate::grammar;

/// Parse binary chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::property_value_data_types::binary::binary;
///
/// assert!(binary("aB1+bC2/Ab==".into()).is_ok());
/// assert!(binary("B+/=".into()).is_ok());
///
/// assert!(binary("Abc".into()).is_err());
/// assert!(binary("cB+/=".into()).is_err());
/// assert!(binary(":".into()).is_err());
/// ```
///
/// binary     = *(4b-char) [b-end]
/// ; A "BASE64" encoded character string, as defined by [RFC4648].
pub fn binary(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "BINARY",
        recognize(
            pair(
                many0(
                    many_m_n(4, 4, b_char),
                ),
                b_end,
            )
        )
    )(input)
}

/// Parse b_end chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::property_value_data_types::binary::b_end;
///
/// assert!(b_end("Ab==".into()).is_ok());
/// assert!(b_end("B+/=".into()).is_ok());
///
/// assert!(b_end("Abc".into()).is_err());
/// assert!(b_end("cB+/=".into()).is_err());
/// assert!(b_end(":".into()).is_err());
/// ```
///
/// b-end      = (2b-char "==") / (3b-char "=")
pub fn b_end(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        alt(
            (
                tuple(
                    (
                        b_char,
                        b_char,
                        tag("="),
                        tag("="),
                    )
                ),
                tuple(
                    (
                        b_char,
                        b_char,
                        b_char,
                        tag("="),
                    )
                ),
            )
        )
    )(input)
}

/// Parse b_char char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::property_value_data_types::binary::b_char;
///
/// assert!(b_char("a".into()).is_ok());
/// assert!(b_char("B".into()).is_ok());
/// assert!(b_char("+".into()).is_ok());
/// assert!(b_char("/".into()).is_ok());
///
/// assert!(b_char(":".into()).is_err());
/// ```
///
/// b-char = ALPHA / DIGIT / "+" / "/"
pub fn b_char(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_b_char)(input)
}

/// Returns if b_char
///
/// b-char = ALPHA / DIGIT / "+" / "/"
pub fn is_b_char(input: char) -> bool {
    is_alphabetic(input as u8) ||
    is_digit(input as u8) ||
    grammar::is_plus_sign_char(input) ||
    grammar::is_solidus_char(input)
}

// Value Name:  BINARY
//
// Purpose:  This value type is used to identify properties that contain
//    a character encoding of inline binary data.  For example, an
//    inline attachment of a document might be included in an iCalendar
//    object.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     binary     = *(4b-char) [b-end]
//     ; A "BASE64" encoded character string, as defined by [RFC4648].
//
//     b-end      = (2b-char "==") / (3b-char "=")
//
//     b-char = ALPHA / DIGIT / "+" / "/"
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Binary(pub String);

impl ICalendarEntity for Binary {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(binary, |value: ParserInput| Self(value.to_string()))(input)
    }

    fn render_ical(&self) -> String {
        self.0.clone()
    }
}

impl_icalendar_entity_traits!(Binary);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Binary::parse_ical("aB1+bC2/Ab== TESTING".into()),
            (
                " TESTING",
                Binary(String::from("aB1+bC2/Ab==")),
            ),
        );

        assert_parser_output!(
            Binary::parse_ical("B+/= TESTING".into()),
            (
                " TESTING",
                Binary(String::from("B+/=")),
            ),
        );

        assert!(Binary::parse_ical("Abc".into()).is_err());
        assert!(Binary::parse_ical("cB+/=".into()).is_err());
        assert!(Binary::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Binary(String::from("aB1+bC2/Ab==")).render_ical(),
            String::from("aB1+bC2/Ab=="),
        );

        assert_eq!(
            Binary(String::from("B+/= TESTING")).render_ical(),
            String::from("B+/= TESTING"),
        );
    }
}
