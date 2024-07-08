use nom::sequence::tuple;
use nom::error::context;
use nom::combinator::{recognize, map_res};
use nom::bytes::complete::take_while_m_n;
use nom::character::is_digit;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits, map_err_message};

/// Parse date chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date::date;
///
/// assert!(date("19970714".into()).is_ok());
///
/// assert!(date("1997071".into()).is_err());
/// assert!(date("c1997071/=".into()).is_err());
/// assert!(date(":".into()).is_err());
/// ```
///
/// date               = date-value
pub fn date(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "DATE",
        recognize(date_value),
    )(input)
}

/// Parse date_value chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date::date_value;
///
/// assert!(date_value("19970714".into()).is_ok());
///
/// assert!(date_value("1997071".into()).is_err());
/// assert!(date_value("c1997071/=".into()).is_err());
/// assert!(date_value(":".into()).is_err());
/// ```
///
/// date-value         = date-fullyear date-month date-mday
pub fn date_value(input: ParserInput) -> ParserResult<(i32, u32, u32)> {
    map_err_message!(
        tuple(
            (
                date_fullyear,
                date_month,
                date_mday,
            )
        ),
        "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
    )(input)
}

/// Parse date-fullyear chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date::date_fullyear;
///
/// assert!(date_fullyear("2007".into()).is_ok());
/// assert!(date_fullyear("0000".into()).is_ok());
/// assert!(date_fullyear("9999".into()).is_ok());
///
/// assert!(date_fullyear("0".into()).is_err());
/// assert!(date_fullyear("000".into()).is_err());
/// assert!(date_fullyear(":".into()).is_err());
/// ```
///
/// date-fullyear      = 4DIGIT
pub fn date_fullyear(input: ParserInput) -> ParserResult<i32> {
    let (remaining, year) = take_while_m_n(4, 4, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_year) = year.to_string().parse::<i32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid year"), input)
            )
        );
    };

    Ok((remaining, parsed_year))
}

/// Parse date_month chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date::date_month;
///
/// assert!(date_month("01".into()).is_ok());
/// assert!(date_month("06".into()).is_ok());
/// assert!(date_month("12".into()).is_ok());
///
/// assert!(date_month("00".into()).is_err());
/// assert!(date_month("13".into()).is_err());
/// assert!(date_month("A".into()).is_err());
/// assert!(date_month(":".into()).is_err());
/// ```
///
/// date-month         = 2DIGIT        ;01-12
pub fn date_month(input: ParserInput) -> ParserResult<u32> {
    let (remaining, month) = take_while_m_n(2, 2, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_month) = month.to_string().parse::<u32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid month"), input)
            )
        );
    };

    if !(1..=12).contains(&parsed_month) {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("expected month between 01-12"), input)
            )
        );
    }

    Ok((remaining, parsed_month))
}

/// Parse date chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date::date_mday;
///
/// assert!(date_mday("01".into()).is_ok());
/// assert!(date_mday("15".into()).is_ok());
/// assert!(date_mday("31".into()).is_ok());
///
/// assert!(date_mday("00".into()).is_err());
/// assert!(date_mday("32".into()).is_err());
/// assert!(date_mday("A".into()).is_err());
/// assert!(date_mday(":".into()).is_err());
/// ```
///
/// date-mday          = 2DIGIT        ;01-28, 01-29, 01-30, 01-31
///                                    ;based on month/year
pub fn date_mday(input: ParserInput) -> ParserResult<u32> {
    let (remaining, mday) = take_while_m_n(2, 2, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_mday) = mday.to_string().parse::<u32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid mday"), input)
            )
        );
    };

    if !(1..=31).contains(&parsed_mday) {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("expected mday between 01-31"), input)
            )
        );
    }

    Ok((remaining, parsed_mday))
}

// Value Name:  DATE
//
//    Purpose:  This value type is used to identify values that contain a
//       calendar date.
//
//    Format Definition:  This value type is defined by the following
//       notation:
//
//        date               = date-value
//
//        date-value         = date-fullyear date-month date-mday
//        date-fullyear      = 4DIGIT
//        date-month         = 2DIGIT        ;01-12
//        date-mday          = 2DIGIT        ;01-28, 01-29, 01-30, 01-31
//                                           ;based on month/year
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl ICalendarEntity for Date {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "DATE",
            map_res(
                date_value,
                |(year, month, day)| {
                    let date = Self {
                        year,
                        month,
                        day,
                    };

                    if let Err(error) = date.validate() {
                        Err(error)
                    } else {
                        Ok(date)
                    }
                }
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day)
    }

    fn validate(&self) -> Result<(), String> {
        if chrono::NaiveDate::from_ymd_opt(self.year, self.month, self.day).is_none() {
            Err(String::from("invalid date"))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<Date> for chrono::NaiveDate {
    type Error = String;

    fn try_from(date: Date) -> Result<chrono::NaiveDate, Self::Error> {
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(date.year, date.month, date.day) {
            Ok(date)
        } else {
            Err(String::from("invalid date"))
        }
    }
}

impl_icalendar_entity_traits!(Date);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Date::parse_ical("19970714 TESTING".into()),
            (
                " TESTING",
                Date {
                    year: 1997_i32,
                    month: 7_u32,
                    day: 14_u32
                },
            ),
        );

        assert!(Date::parse_ical("Abc".into()).is_err());
        assert!(Date::parse_ical("cB+/=".into()).is_err());
        assert!(Date::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Date {
                year: 1997_i32,
                month: 7_u32,
                day: 14_u32,
            }.render_ical(),
            String::from("19970714"),
        );
    }

    #[test]
    fn parse_ical_error() {
        assert_parser_error!(
            Date::parse_ical(":".into()),
            nom::Err::Error(
                span: ":",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE"],
            ),
        );

        assert_parser_error!(
            Date::parse_ical("20240".into()),
            nom::Err::Error(
                span: "0",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE"],
            ),
        );

        assert_parser_error!(
            Date::parse_ical("2024020".into()),
            nom::Err::Error(
                span: "0",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE"],
            ),
        );
    }

    #[test]
    fn validate() {
        assert_eq!(
            Date {
                year: 1997_i32,
                month: 1_u32,
                day: 14_u32,
            }.validate(),
            Ok(()),
        );

        assert_eq!(
            Date {
                year: 1997_i32,
                month: 7_u32,
                day: 14_u32,
            }.validate(),
            Ok(()),
        );

        assert_eq!(
            Date {
                year: 1997_i32,
                month: 12_u32,
                day: 14_u32,
            }.validate(),
            Ok(()),
        );

        assert_eq!(
            Date {
                year: 1997_i32,
                month: 13_u32,
                day: 14_u32,
            }.validate(),
            Err(String::from("invalid date")),
        );

        assert_eq!(
            Date {
                year: 1997_i32,
                month: 2_u32,
                day: 31_u32,
            }.validate(),
            Err(String::from("invalid date")),
        );
    }
}
