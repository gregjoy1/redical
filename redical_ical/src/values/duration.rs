use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::digit1,
    combinator::{opt, map, map_res, cut, recognize},
    error::context,
    sequence::{preceded, terminated, tuple, pair},
};

use crate::grammar::PositiveNegative;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

const SECONDS_IN_MINUTE: i64 = 60;
const SECONDS_IN_HOUR: i64 = SECONDS_IN_MINUTE * 60;
const SECONDS_IN_DAY: i64 = SECONDS_IN_HOUR * 24;
const SECONDS_IN_WEEK: i64 = SECONDS_IN_DAY * 7;

/// Parse duration chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::duration::duration;
///
/// assert!(duration("P15DT5H0M20S".into()).is_ok());
/// assert!(duration("-P7W".into()).is_ok());
/// assert!(duration("PT25S".into()).is_ok());
///
/// assert!(duration("Abc".into()).is_err());
/// assert!(duration("cB+/=".into()).is_err());
/// assert!(duration(":".into()).is_err());
/// ```
///
/// dur-value  = (["+"] / "-") "P" (dur-date / dur-time / dur-week)
pub fn duration(input: ParserInput) -> ParserResult<ParserInput> {
    context("DURATION", recognize(dur_value))(input)
}

/// Parse dur_value chars.
///
/// dur-value  = (["+"] / "-") "P" (dur-date / dur-time / dur-week)
pub fn dur_value(input: ParserInput) -> ParserResult<(Option<PositiveNegative>, (Option<i64>, Option<i64>, Option<(Option<i64>, Option<i64>, Option<i64>)>))> {
    tuple((
        opt(PositiveNegative::parse_ical),
        preceded(
            tag("P"),
            cut(
                map_err_message!(
                    alt((
                        map(dur_week, |week| (Some(week), None, None)),
                        map(dur_date, |(day, time)| (None, Some(day), time)),
                        map(dur_time, |time| (None, None, Some(time))),
                    )),
                    "expected either iCalendar RFC-5545 DUR-DATE, DUR-TIME, or DUR-WEEK",
                )
            )
        ),
    ))(input)
}

/// Parse dur_week chars.
///
/// dur-week   = 1*DIGIT "W"
pub fn dur_week(input: ParserInput) -> ParserResult<i64> {
    map_res(
        terminated(
            digit1,
            tag("W"),
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}

/// Parse dur_date chars.
///
/// dur-date   = dur-day [dur-time]
pub fn dur_date(input: ParserInput) -> ParserResult<(i64, Option<(Option<i64>, Option<i64>, Option<i64>)>)> {
    pair(dur_day, opt(dur_time))(input)
}

/// Parse dur_day chars.
///
/// dur-day    = 1*DIGIT "D"
pub fn dur_day(input: ParserInput) -> ParserResult<i64> {
    map_res(
        terminated(
            digit1,
            tag("D"),
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}

/// Parse dur_time chars.
///
/// dur-time   = "T" (dur-hour / dur-minute / dur-second)
pub fn dur_time(input: ParserInput) -> ParserResult<(Option<i64>, Option<i64>, Option<i64>)> {
    preceded(
        tag("T"),
        cut(
            map_err_message!(
                tuple((
                    opt(dur_hour),
                    opt(dur_minute),
                    opt(dur_second),
                )),
                "expected either iCalendar RFC-5545 DUR-DATE, DUR-TIME, or DUR-WEEK",
            )
        ),
    )(input)
}

/// Parse dur_hour chars.
///
/// dur-hour   = 1*DIGIT "H" [dur-minute]
pub fn dur_hour(input: ParserInput) -> ParserResult<i64> {
    map_res(
        terminated(
            digit1,
            tag("H"),
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}

/// Parse dur_minute chars.
///
/// dur-minute = 1*DIGIT "M" [dur-second]

pub fn dur_minute(input: ParserInput) -> ParserResult<i64> {
    map_res(
        terminated(
            digit1,
            tag("M"),
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}

/// Parse dur_value chars.
///
/// dur-second = 1*DIGIT "S"
pub fn dur_second(input: ParserInput) -> ParserResult<i64> {
    map_res(
        terminated(
            digit1,
            tag("S"),
        ),
        |value: ParserInput| value.parse::<i64>(),
    )(input)
}


#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Duration {
    pub positive_negative: Option<PositiveNegative>,
    pub weeks: Option<i64>,
    pub days: Option<i64>,
    pub hours: Option<i64>,
    pub minutes: Option<i64>,
    pub seconds: Option<i64>,
}

impl ICalendarEntity for Duration {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
where
        Self: Sized
    {
        context(
            "DURATION",
            map(
                dur_value,
                |(positive_negative, (weeks, days, time)): (Option<PositiveNegative>, (Option<i64>, Option<i64>, Option<(Option<i64>, Option<i64>, Option<i64>)>))| {
                    let hours = time.and_then(|time| time.0);
                    let minutes = time.and_then(|time| time.1);
                    let seconds = time.and_then(|time| time.2);

                    Self {
                        positive_negative,
                        weeks,
                        days,
                        hours,
                        minutes,
                        seconds,
                    }
                }
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        let mut output = String::new();

        if self.is_empty() {
            return output;
        }

        if let Some(positive_negative) = &self.positive_negative {
            output.push_str(positive_negative.render_ical().as_str());
        }

        output.push_str("P");

        if let Some(weeks) = self.weeks {
            output.push_str(&format!("{weeks}W"));
        }

        if let Some(days) = self.days {
            output.push_str(&format!("{days}D"));
        }

        if self.hours.is_some() || self.minutes.is_some() || self.seconds.is_some() {
            output.push_str("T");
        }

        if let Some(hours) = self.hours {
            output.push_str(&format!("{hours}H"));
        }

        if let Some(minutes) = self.minutes {
            output.push_str(&format!("{minutes}M"));
        }

        if let Some(seconds) = self.seconds {
            output.push_str(&format!("{seconds}S"));
        }

        output
    }
}

impl Duration {
    pub fn get_duration_in_seconds(&self) -> i64 {
        let mut duration_in_seconds = 0;

        if let Some(weeks) = self.weeks {
            duration_in_seconds += weeks * SECONDS_IN_WEEK;
        }

        if let Some(days) = self.days {
            duration_in_seconds += days * SECONDS_IN_DAY;
        }

        if let Some(hours) = self.hours {
            duration_in_seconds += hours * SECONDS_IN_HOUR;
        }

        if let Some(minutes) = self.minutes {
            duration_in_seconds += minutes * SECONDS_IN_MINUTE;
        }

        if let Some(seconds) = self.seconds {
            duration_in_seconds += seconds
        }

        if let Some(PositiveNegative::Negative) = self.positive_negative {
            duration_in_seconds = -duration_in_seconds;
        }

        duration_in_seconds
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

impl Default for Duration {
    fn default() -> Self {
        Duration {
            positive_negative: None,
            weeks: None,
            days: None,
            hours: None,
            minutes: None,
            seconds: None,
        }
    }
}

impl From<i64> for Duration {
    fn from(duration_in_seconds: i64) -> Self {
        let mut remaining_seconds = duration_in_seconds;

        let mut weeks = None;
        let mut days = None;
        let mut hours = None;
        let mut minutes = None;
        let mut seconds = None;

        if remaining_seconds >= SECONDS_IN_WEEK {
            weeks = Some(remaining_seconds / SECONDS_IN_WEEK);

            remaining_seconds = remaining_seconds % SECONDS_IN_WEEK;
        }

        if remaining_seconds >= SECONDS_IN_DAY {
            days = Some(remaining_seconds / SECONDS_IN_DAY);

            remaining_seconds = remaining_seconds % SECONDS_IN_DAY;
        }

        if remaining_seconds >= SECONDS_IN_HOUR {
            hours = Some(remaining_seconds / SECONDS_IN_HOUR);

            remaining_seconds = remaining_seconds % SECONDS_IN_HOUR;
        }

        if remaining_seconds >= SECONDS_IN_MINUTE {
            minutes = Some(remaining_seconds / SECONDS_IN_MINUTE);

            remaining_seconds = remaining_seconds % SECONDS_IN_MINUTE;
        }

        if remaining_seconds > 0 || duration_in_seconds == 0 {
            seconds = Some(remaining_seconds);
        }

        let mut positive_negative = None;

        if duration_in_seconds < 0 {
            positive_negative = Some(PositiveNegative::Negative)
        }

        Duration {
            weeks,
            days,
            hours,
            minutes,
            seconds,
            positive_negative,
        }
    }
}

impl_icalendar_entity_traits!(Duration);

#[cfg(test)]
mod test {

    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn test_from_seconds_int() {
        assert_eq!(
            Duration::from(1483506),
            Duration {
                weeks: Some(2),
                days: Some(3),
                hours: Some(4),
                minutes: Some(5),
                seconds: Some(6),
                positive_negative: None,
            }
        );

        assert_eq!(
            Duration::from(25),
            Duration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
                positive_negative: None,
            }
        );

        assert_eq!(
            Duration::from(0),
            Duration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(0),
                positive_negative: None,
            }
        );


        assert_eq!(
            Duration::from(-100),
            Duration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
                positive_negative: Some(PositiveNegative::Negative),
            }
        );
    }

    #[test]
    fn test_parse_ical() {
        assert_parser_output!(
            Duration::parse_ical("P7W SOMETHING ELSE".into()),
            (
                " SOMETHING ELSE",
                Duration {
                    positive_negative: None,
                    weeks: Some(7),
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: None,
                }
            )
        );

        assert_parser_output!(
            Duration::parse_ical("P15DT5H0M20S".into()),
            (
                "",
                Duration {
                    positive_negative: None,
                    weeks: None,
                    days: Some(15),
                    hours: Some(5),
                    minutes: Some(0),
                    seconds: Some(20),
                },
            )
        );

        assert_parser_output!(
            Duration::parse_ical("P7W".into()),
            (
                "",
                Duration {
                    positive_negative: None,
                    weeks: Some(7),
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: None,
                },
            )
        );

        assert_parser_output!(
            Duration::parse_ical("PT25S".into()),
            (
                "",
                Duration {
                    positive_negative: None,
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: Some(25),
                },
            )
        );

        assert_parser_output!(
            Duration::parse_ical("-PT25S".into()),
            (
                "",
                Duration {
                    positive_negative: Some(PositiveNegative::Negative),
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: None,
                    seconds: Some(25),
                },
            )
        );
    }

    #[test]
    fn test_parse_ical_error() {
        assert_parser_error!(
            Duration::parse_ical("P15--INVALID20S".into()),
            nom::Err::Failure(
                span: "15--INVALID20S",
                message: "expected either iCalendar RFC-5545 DUR-DATE, DUR-TIME, or DUR-WEEK",
                context: ["DURATION"],
            ),
        );
    }

    #[test]
    fn test_get_duration_in_seconds() {
        assert_eq!(Duration::default().get_duration_in_seconds(), 0);

        assert_eq!(
            Duration {
                positive_negative: None,
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
            }
            .get_duration_in_seconds(),
            20 + ((60 * 60) * 5) + (((60 * 60) * 24) * 15),
        );

        assert_eq!(
            Duration {
                positive_negative: Some(PositiveNegative::Negative),
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            }
            .get_duration_in_seconds(),
            -(((60 * 60) * 24) * 7) * 7,
        );

        assert_eq!(
            Duration {
                positive_negative: None,
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
            }
            .get_duration_in_seconds(),
            25,
        );
    }

    #[test]
    fn test_render_ical() {
        assert_eq!(Duration::default().render_ical(), String::from(""));

        assert_eq!(
            Duration {
                positive_negative: None,
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
            }.render_ical(),
            String::from("P15DT5H0M20S"),
        );

        assert_eq!(
            Duration {
                positive_negative: Some(PositiveNegative::Negative),
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            }.render_ical(),
            String::from("-P7W"),
        );

        assert_eq!(
            Duration {
                positive_negative: None,
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
            }.render_ical(),
            String::from("PT25S"),
        );
    }
}
