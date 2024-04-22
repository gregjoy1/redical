use std::string::ToString;

use nom::combinator::{recognize, map};
use nom::multi::many0;
use nom::branch::alt;
use nom::bytes::complete::{tag, take_while_m_n};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, terminated_lookahead};
use crate::grammar::{colon, dquote, is_safe_char};

// text       = *(TSAFE-CHAR / ":" / DQUOTE / ESCAPED-CHAR)
pub fn text(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many0(
            alt((
                tsafe_char,
                colon,
                dquote,
                escaped_char,
            ))
        ),
    )(input)
}

//
// ESCAPED-CHAR = ("\\" / "\;" / "\," / "\N" / "\n")
//    ; \\ encodes \, \N or \n encodes newline
//    ; \; encodes ;, \, encodes ,
pub fn escaped_char(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        alt((
            tag("\\\\"),
            tag("\\;"),
            tag("\\,"),
            tag("\\N"),
            tag("\\n"),
        ))
    )(input)
}

// TSAFE-CHAR = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-5B /
//              %x5D-7E / NON-US-ASCII
//    ; Any character except CONTROLs not needed by the current
//    ; character set, DQUOTE, ";", ":", "\", ","
pub fn tsafe_char(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_tsafe_char)(input)
}

// TSAFE-CHAR    = %x20-21 / %x23-2B / %x2D-39 / %x3C-5B %x5D-7E / NON-US-ASCII
// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E / NON-US-ASCII
// ; Any character except CONTROL, DQUOTE, ";", ":", ","
// TSAFE-CHAR = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-5B /
//              %x5D-7E / NON-US-ASCII
//    ; Any character except CONTROLs not needed by the current
//    ; character set, DQUOTE, ";", ":", "\", ","
//
// NOTE: This is the same as SAFE-CHAR excluding %x5C ('\')
//
// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E / NON-US-ASCII
pub fn is_tsafe_char(input: char) -> bool {
    input != '\\' && is_safe_char(input)
}

// Value Name:  TEXT
//
// Purpose:  This value type is used to identify values that contain
//   human-readable text.
//
// Format Definition:  This value type is defined by the following
//   notation:
//    text       = *(TSAFE-CHAR / ":" / DQUOTE / ESCAPED-CHAR)
//       ; Folded according to description above
//
//    ESCAPED-CHAR = ("\\" / "\;" / "\," / "\N" / "\n")
//       ; \\ encodes \, \N or \n encodes newline
//       ; \; encodes ;, \, encodes ,
//
//    TSAFE-CHAR = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-5B /
//                 %x5D-7E / NON-US-ASCII
//       ; Any character except CONTROLs not needed by the current
//       ; character set, DQUOTE, ";", ":", "\", ","
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Text(pub String);

impl ICalendarEntity for Text {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(
            // Prevent greedily over consuming the text including following properties by looking
            // ahead for properties relevant to the parsing context and terminating where they
            // begin.
            terminated_lookahead(
                text,
                input.extra.terminating_property_lookahead(),
            ),
            |value| Self(value.to_string()),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        self.0.to_string()
    }
}

impl From<String> for Text {
    fn from(value: String) -> Self {
        Text(value)
    }
}

impl From<&str> for Text {
    fn from(value: &str) -> Self {
        Text(String::from(value))
    }
}

impl_icalendar_entity_traits!(Text);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Text::parse_ical("\t\r\n\x0C \\;\\,\\N\\n\\\\!#$%&'()*+-./0123456789<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~の:;".into()),
            (
                ";",
                Text(String::from("\t\r\n\x0C \\;\\,\\N\\n\\\\!#$%&'()*+-./0123456789<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~の:")),
            ),
        );

        use nom::combinator::all_consuming;

        assert!(all_consuming(Text::parse_ical)(",".into()).is_err());
        assert!(all_consuming(Text::parse_ical)(";".into()).is_err());
        assert!(all_consuming(Text::parse_ical)("\\".into()).is_err());
    }

    #[test]
    fn parse_ical_with_terminated_property_lookahead() {
        assert_parser_output!(
            Text::parse_ical("Some text\\, some more text! DESCRIPTION:Description Text".into()),
            (
                " DESCRIPTION:Description Text",
                Text(String::from("Some text\\, some more text!")),
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Text(String::from("\t\r\n\x0C \\;\\,\\N\\n\\\\!#$%&'()*+-./0123456789<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~の:")).render_ical(),
            String::from("\t\r\n\x0C \\;\\,\\N\\n\\\\!#$%&'()*+-./0123456789<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[]^_`abcdefghijklmnopqrstuvwxyz{|}~の:"),
        );
    }
}
