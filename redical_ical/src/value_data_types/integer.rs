use std::ops::{Deref, DerefMut};

use nom::combinator::{recognize, map, map_res, opt};
use nom::character::complete::{one_of, digit1};
use nom::bytes::complete::take_while_m_n;
use nom::character::is_digit;
use nom::sequence::pair;

use crate::{ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};
use crate::grammar::PositiveNegative;

// integer      = (["+"] / "-") 1*DIGIT ["." 1*DIGIT]
pub fn integer(input: ParserInput) -> ParserResult<i64> {
    map_res(
        recognize(
            pair(opt(one_of("+-")), digit1)
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}

// Value Name:  integer
//
//    Purpose:  This value type is used to identify properties that contain
//       a real-number value.
//
//    Format Definition:  This value type is defined by the following
//       notation:
//
//        integer      = (["+"] / "-") 1*DIGIT ["." 1*DIGIT]
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Integer(pub i64);

impl ICalendarEntity for Integer {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(integer, |value| Self(value))(input)
    }

    fn render_ical(&self) -> String {
        self.0.to_string()
    }
}

impl Integer {
    /// Parses a potentially positive or negative digits
    ///
    /// # Examples
    ///
    /// ```rust
    ///
    /// use redical_ical::value_data_types::integer::Integer;
    ///
    /// // Create a parser that:
    /// //   1. Expects a digit 2-3 characters in length
    /// //   2. Expects it to be at greater than or equal to 15
    /// //   3. Expects it to be less than or equal to 500
    /// let mut parser = Integer::parse_signed_m_n(2, 3, 15, 500);
    ///
    /// // Testing "+22"
    /// let result = parser("+22 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(22_i64));
    ///
    /// // Testing "-250"
    /// let result = parser("-250 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(-250_i64));
    ///
    /// // Testing "500"
    /// let result = parser("500 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(500_i64));
    ///
    /// assert!(parser("1".into()).is_err());
    /// assert!(parser("501".into()).is_err());
    /// assert!(parser("10".into()).is_err());
    /// ```
    /// [plus / minus] 1*digit
    pub fn parse_signed_m_n<'a>(min_chars: usize, max_chars: usize, min_value: i64, max_value: i64) -> impl FnMut(ParserInput) -> ParserResult<Integer> {
        move |input: ParserInput| {
            let (remaining, parsed_positive_negative) = opt(PositiveNegative::parse_ical)(input)?;
            let (remaining, mut parsed_integer) = Self::parse_unsigned_m_n(min_chars, max_chars, min_value, max_value)(remaining)?;

            if let Some(PositiveNegative::Negative) = parsed_positive_negative {
                parsed_integer.0 = -parsed_integer.0;
            }

            Ok((remaining, parsed_integer))
        }
    }

    /// Parses purely positive digits
    ///
    /// # Examples
    ///
    /// ```rust
    ///
    /// use redical_ical::value_data_types::integer::Integer;
    /// use redical_ical::grammar::PositiveNegative;
    ///
    /// // Create a parser that:
    /// //   1. Expects a digit 2-3 characters in length
    /// //   2. Expects it to be at greater than or equal to 15
    /// //   3. Expects it to be less than or equal to 500
    /// let mut parser = Integer::parse_unsigned_m_n(2, 3, 15, 500);
    ///
    /// // Testing "22"
    /// let result = parser("22 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(22_i64));
    ///
    /// // Testing "250"
    /// let result = parser("250 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(250_i64));
    ///
    /// // Testing "500"
    /// let result = parser("500 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, Integer(500_i64));
    ///
    /// assert!(parser("-22".into()).is_err());
    /// assert!(parser("+22".into()).is_err());
    ///
    /// assert!(parser("1".into()).is_err());
    /// assert!(parser("501".into()).is_err());
    /// assert!(parser("10".into()).is_err());
    /// ```
    /// [plus / minus] 1*digit
    pub fn parse_unsigned_m_n<'a>(min_chars: usize, max_chars: usize, min_value: i64, max_value: i64) -> impl FnMut(ParserInput) -> ParserResult<Integer> {
        move |input: ParserInput| {
            let (remaining, parsed_value) = take_while_m_n(min_chars, max_chars, |value| is_digit(value as u8))(input)?;

            let Ok(value) = parsed_value.to_string().parse::<i64>() else {
                return Err(
                    nom::Err::Error(
                        ParserError::new(String::from("Invalid number"), input)
                    )
                );
            };

            if value < min_value || value > max_value {
                return Err(
                    nom::Err::Error(
                        ParserError::new(format!("Expected number between {min_value}-{max_value}"), input)
                    )
                );
            }

            Ok((remaining, Integer::from(value)))
        }
    }
}

impl Deref for Integer {
    type Target = i64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Integer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<u32> for Integer {
    fn from(value: u32) -> Self {
        Integer(value as i64)
    }
}

impl From<u64> for Integer {
    fn from(value: u64) -> Self {
        Integer(value as i64)
    }
}

impl From<i64> for Integer {
    fn from(value: i64) -> Self {
        Integer(value)
    }
}

impl_icalendar_entity_traits!(Integer);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Integer::parse_ical("1234567890 TESTING".into()),
            (
                " TESTING",
                Integer(1234567890_i64),
            ),
        );

        assert_parser_output!(
            Integer::parse_ical("-1234567890 TESTING".into()),
            (
                " TESTING",
                Integer(-1234567890_i64),
            ),
        );

        assert_parser_output!(
            Integer::parse_ical("+1234567890 TESTING".into()),
            (
                " TESTING",
                Integer(1234567890_i64),
            ),
        );

        assert_parser_output!(
            Integer::parse_ical("432109876 TESTING".into()),
            (
                " TESTING",
                Integer(432109876_i64),
            ),
        );

        assert!(Integer::parse_ical("OTHER".into()).is_err());
        assert!(Integer::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Integer(-1234567890_i64).render_ical(),
            String::from("-1234567890"),
        );

        assert_eq!(
            Integer(1234567890_i64).render_ical(),
            String::from("1234567890"),
        );

        assert_eq!(
            Integer(432109876_i64).render_ical(),
            String::from("432109876"),
        );
    }
}
