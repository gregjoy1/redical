use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// ENCODING = ( "8BIT"
//            ; "8bit" text encoding is defined in [RFC2045]
//            / "BASE64"
//            ; "BASE64" binary encoding format is defined in [RFC4648]
//            )
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Encoding {
    _8Bit,  // "8bit" text encoding is defined in [RFC2045]
    Base64, // "BASE64" binary encoding format is defined in [RFC4648]
}

impl ICalendarEntity for Encoding {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ENCODING",
            alt((
                map(tag("8BIT"), |_| Encoding::_8Bit),
                map(tag("BASE64"), |_| Encoding::Base64),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
           Self::_8Bit => String::from("8BIT"),
           Self::Base64 => String::from("BASE64"),
        }
    }
}

impl_icalendar_entity_traits!(Encoding);

// Inline Encoding
//
// Parameter Name:  ENCODING
//
// Purpose:  To specify an alternate inline encoding for the property
//    value.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     encodingparam      = "ENCODING" "="
//                        ( "8BIT"
//        ; "8bit" text encoding is defined in [RFC2045]
//                        / "BASE64"
//        ; "BASE64" binary encoding format is defined in [RFC4648]
//                        )
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct EncodingParam(Encoding);

impl ICalendarEntity for EncodingParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ENCODINGPARAM",
            map(
                pair(
                    tag("ENCODING"),
                    preceded(tag("="), cut(Encoding::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ENCODING={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(EncodingParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            EncodingParam::parse_ical(r#"ENCODING=8BIT TESTING"#.into()),
            (
                " TESTING",
                EncodingParam(Encoding::_8Bit),
            ),
        );

        assert_parser_output!(
            EncodingParam::parse_ical(r#"ENCODING=BASE64 TESTING"#.into()),
            (
                " TESTING",
                EncodingParam(Encoding::Base64),
            ),
        );

        assert!(EncodingParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            EncodingParam(Encoding::_8Bit).render_ical(),
            String::from("ENCODING=8BIT"),
        );

        assert_eq!(
            EncodingParam(Encoding::Base64).render_ical(),
            String::from("ENCODING=BASE64"),
        );
    }
}
