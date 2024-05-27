use nom::sequence::tuple;
use nom::error::context;
use nom::combinator::{recognize, map_res, opt};
use nom::bytes::complete::{tag, take_while_m_n};
use nom::character::is_digit;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits, map_err_message};

/// Parse time chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::time::time;
///
/// assert!(time("083000".into()).is_ok());
/// assert!(time("133000Z".into()).is_ok());
///
/// assert!(time("0830".into()).is_err());
/// assert!(time("13300=".into()).is_err());
/// assert!(time(":".into()).is_err());
/// ```
///
/// time         = time-hour time-minute time-second [time-utc]
pub fn time(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "TIME",
        recognize(
            map_err_message!(
                tuple(
                    (
                        time_hour,
                        time_minute,
                        time_second,
                        opt(time_utc),
                    )
                ),
                "expected iCalendar RFC-5545 TIME (TIME-HOUR TIME-MINUTE TIME-SECOND [TIME-UTC])",
            )
        ),
    )(input)
}

/// Parse time-hour chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::time::time_hour;
///
/// assert!(time_hour("00".into()).is_ok());
/// assert!(time_hour("06".into()).is_ok());
/// assert!(time_hour("23".into()).is_ok());
///
/// assert!(time_hour("24".into()).is_err());
/// assert!(time_hour("0".into()).is_err());
/// assert!(time_hour(":".into()).is_err());
/// ```
///
/// time-hour      = 4DIGIT
pub fn time_hour(input: ParserInput) -> ParserResult<u32> {
    let (remaining, hour) = take_while_m_n(2, 2, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_hour) = hour.to_string().parse::<u32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid hour"), input)
            )
        );
    };

    if parsed_hour > 23 {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("expected hour between 00-23"), input)
            )
        );
    }

    Ok((remaining, parsed_hour))
}

/// Parse time-minute chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::time::time_minute;
///
/// assert!(time_minute("00".into()).is_ok());
/// assert!(time_minute("06".into()).is_ok());
/// assert!(time_minute("59".into()).is_ok());
///
/// assert!(time_minute("60".into()).is_err());
/// assert!(time_minute("0".into()).is_err());
/// assert!(time_minute(":".into()).is_err());
/// ```
///
/// time-minute      = 4DIGIT
pub fn time_minute(input: ParserInput) -> ParserResult<u32> {
    let (remaining, minute) = take_while_m_n(2, 2, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_minute) = minute.to_string().parse::<u32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid minute"), input)
            )
        );
    };

    if parsed_minute > 59 {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("expected minute between 00-59"), input)
            )
        );
    }

    Ok((remaining, parsed_minute))
}

/// Parse time-second chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::time::time_second;
///
/// assert!(time_second("00".into()).is_ok());
/// assert!(time_second("06".into()).is_ok());
/// assert!(time_second("60".into()).is_ok());
///
/// assert!(time_second("61".into()).is_err());
/// assert!(time_second("0".into()).is_err());
/// assert!(time_second(":".into()).is_err());
/// ```
///
/// time-second      = 4DIGIT
pub fn time_second(input: ParserInput) -> ParserResult<u32> {
    let (remaining, second) = take_while_m_n(2, 2, |value| is_digit(value as u8))(input)?;

    let Ok(parsed_second) = second.to_string().parse::<u32>() else {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("invalid second"), input)
            )
        );
    };

    if parsed_second > 60 {
        return Err(
            nom::Err::Error(
                ParserError::new(String::from("expected second between 00-60"), input)
            )
        );
    }

    Ok((remaining, parsed_second))
}

// time-utc     = "Z"
pub fn time_utc(input: ParserInput) -> ParserResult<ParserInput> {
    tag("Z")(input)
}

// Value Name:  TIME
//
// Purpose:  This value type is used to identify values that contain a
//    time of day.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     time         = time-hour time-minute time-second [time-utc]
//
//     time-hour    = 2DIGIT        ;00-23
//     time-minute  = 2DIGIT        ;00-59
//     time-second  = 2DIGIT        ;00-60
//     ;The "60" value is used to account for positive "leap" seconds.
//
//     time-utc     = "Z"
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Time {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub is_utc: bool,
}

impl ICalendarEntity for Time {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "TIME",
            map_res(
                map_err_message!(
                    tuple(
                        (
                            time_hour,
                            time_minute,
                            time_second,
                            opt(time_utc),
                        )
                    ),
                    "expected iCalendar RFC-5545 TIME (TIME-HOUR TIME-MINUTE TIME-SECOND [TIME-UTC])",
                ),
                |(hour, minute, second, utc)| {
                    let time = Self {
                        hour,
                        minute,
                        second,
                        is_utc: utc.is_some(),
                    };

                    if let Err(error) = time.validate() {
                        Err(error)
                    } else {
                        Ok(time)
                    }
                }
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        if self.is_utc {
            format!("{:02}{:02}{:02}Z", self.hour, self.minute, self.second)
        } else {
            format!("{:02}{:02}{:02}", self.hour, self.minute, self.second)
        }
    }

    fn validate(&self) -> Result<(), String> {
        if chrono::NaiveTime::from_hms_opt(self.hour, self.minute, self.second).is_none() {
            Err(String::from("time is invalid"))
        } else {
            Ok(())
        }
    }
}

impl TryFrom<Time> for chrono::NaiveTime {
    type Error = String;

    fn try_from(time: Time) -> Result<chrono::NaiveTime, Self::Error> {
        if let Some(time) = chrono::NaiveTime::from_hms_opt(time.hour, time.minute, time.second) {
            Ok(time)
        } else {
            Err(String::from("time is invalid"))
        }
    }
}

impl_icalendar_entity_traits!(Time);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical_error() {
        assert_parser_error!(
            Time::parse_ical(":::: TESTING".into()),
            nom::Err::Error(
                span: ":::: TESTING",
                message: "expected iCalendar RFC-5545 TIME (TIME-HOUR TIME-MINUTE TIME-SECOND [TIME-UTC])",
                context: ["TIME"],
            ),
        );
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Time::parse_ical("083000 TESTING".into()),
            (
                " TESTING",
                Time{
                    hour: 8_u32,
                    minute: 30_u32,
                    second: 0_u32,
                    is_utc: false,
                },
            ),
        );

        assert_parser_output!(
            Time::parse_ical("133000Z TESTING".into()),
            (
                " TESTING",
                Time{
                    hour: 13_u32,
                    minute: 30_u32,
                    second: 0_u32,
                    is_utc: true,
                },
            ),
        );

        assert!(Time::parse_ical("Abc".into()).is_err());
        assert!(Time::parse_ical("cB+/=".into()).is_err());
        assert!(Time::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Time{
                hour: 8_u32,
                minute: 30_u32,
                second: 0_u32,
                is_utc: false,
            }.render_ical(),
            String::from("083000"),
        );

        assert_eq!(
            Time{
                hour: 13_u32,
                minute: 30_u32,
                second: 0_u32,
                is_utc: true,
            }.render_ical(),
            String::from("133000Z"),
        );
    }
}
