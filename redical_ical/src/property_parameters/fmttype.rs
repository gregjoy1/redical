use nom::error::context;
use nom::sequence::{separated_pair, pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::{tag, take_while_m_n};
use nom::character::{is_alphabetic, is_digit};

use crate::grammar::solidus;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

//     reg-name-chars = ALPHA / DIGIT / "!" /
//                     "#" / "$" / "&" / "." /
//                     "+" / "-" / "^" / "_"
pub fn is_reg_name_char(input: char) -> bool {
    is_alphabetic(input as u8) ||
    is_digit(input as u8) ||
    input == '!' ||
    input == '#' ||
    input == '$' ||
    input == '&' ||
    input == '.' ||
    input == '+' ||
    input == '-' ||
    input == '^' ||
    input == '_'
}

//     reg-name = 1*127reg-name-chars
//     reg-name-chars = ALPHA / DIGIT / "!" /
//                     "#" / "$" / "&" / "." /
//                     "+" / "-" / "^" / "_"
pub fn reg_name(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 127, is_reg_name_char)(input)
}

//     FMTTYPE = type-name "/" subtype-name
//               ; Where "type-name" and "subtype-name" are
//               ; defined in Section 4.2 of [RFC4288].
//
//     type-name = reg-name
//     subtype-name = reg-name
//
//     reg-name = 1*127reg-name-chars
//     reg-name-chars = ALPHA / DIGIT / "!" /
//                     "#" / "$" / "&" / "." /
//                     "+" / "-" / "^" / "_"
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Fmttype {
    pub type_name: String,
    pub subtype_name: String,
}

impl ICalendarEntity for Fmttype {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "FMTTYPE",
            map(
                separated_pair(
                    reg_name,
                    solidus,
                    reg_name,
                ),
                |(type_name, subtype_name)| {
                    Fmttype {
                        type_name: type_name.to_string(),
                        subtype_name: subtype_name.to_string(),
                    }
                }
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("{}/{}", self.type_name, self.subtype_name)
    }
}

impl_icalendar_entity_traits!(Fmttype);

// Format Type
//
// Parameter Name:  FMTTYPE
//
// Purpose:  To specify the content type of a referenced object.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     fmttypeparam = "FMTTYPE" "=" type-name "/" subtype-name
//                    ; Where "type-name" and "subtype-name" are
//                    ; defined in Section 4.2 of [RFC4288].
//
//     type-name = reg-name
//     subtype-name = reg-name
//
//     reg-name = 1*127reg-name-chars
//     reg-name-chars = ALPHA / DIGIT / "!" /
//                     "#" / "$" / "&" / "." /
//                     "+" / "-" / "^" / "_"
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FmttypeParam(pub Fmttype);

impl ICalendarEntity for FmttypeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "FMTTYPEPARAM",
            map(
                pair(
                    tag("FMTTYPE"),
                    preceded(tag("="), cut(Fmttype::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("FMTTYPE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(FmttypeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            FmttypeParam::parse_ical("FMTTYPE=application/msword TESTING".into()),
            (
                " TESTING",
                FmttypeParam(
                    Fmttype {
                        type_name: String::from("application"),
                        subtype_name: String::from("msword"),
                    }
                )
            ),
        );

        assert!(FmttypeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            FmttypeParam(
                Fmttype {
                    type_name: String::from("application"),
                    subtype_name: String::from("msword"),
                }
            ).render_ical(),
            String::from("FMTTYPE=application/msword"),
        );
    }
}
