use nom::combinator::{recognize, map, cut, map_res, opt};
use nom::character::complete::{one_of, digit1};
use nom::sequence::{preceded, tuple};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::grammar::period;

// float      = (["+"] / "-") 1*DIGIT ["." 1*DIGIT]
pub fn float(input: ParserInput) -> ParserResult<f64> {
    map_res(
        recognize(
            tuple((
                opt(one_of("+-")),
                digit1,
                opt(preceded(period, cut(digit1))),
            )),
        ),
        |value: ParserInput| value.parse::<f64>(),
    )(input)
}

// Value Name:  FLOAT
//
//    Purpose:  This value type is used to identify properties that contain
//       a real-number value.
//
//    Format Definition:  This value type is defined by the following
//       notation:
//
//        float      = (["+"] / "-") 1*DIGIT ["." 1*DIGIT]
#[derive(Debug, Clone, PartialOrd, PartialEq)]
pub struct Float(pub f64);

impl Eq for Float {}

impl ICalendarEntity for Float {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(float, Self)(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        self.0.to_string()
    }
}

impl Into<f64> for Float {
    fn into(self) -> f64 {
        self.0.to_owned()
    }
}

impl_icalendar_entity_traits!(Float);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Float::parse_ical("1000000.0000001 TESTING".into()),
            (
                " TESTING",
                Float(1000000.0000001_f64),
            ),
        );

        assert_parser_output!(
            Float::parse_ical("1.333 TESTING".into()),
            (
                " TESTING",
                Float(1.333_f64),
            ),
        );

        assert_parser_output!(
            Float::parse_ical("-3.141592653589793 TESTING".into()),
            (
                " TESTING",
                Float(-std::f64::consts::PI),
            ),
        );

        assert!(Float::parse_ical("OTHER".into()).is_err());
        assert!(Float::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Float(1000000.0000001_f64).render_ical(),
            String::from("1000000.0000001"),
        );

        assert_eq!(
            Float(1.333_f64).render_ical(),
            String::from("1.333"),
        );

        assert_eq!(
            Float(-std::f64::consts::PI).render_ical(),
            String::from("-3.141592653589793"),
        );
    }
}
