use chrono::prelude::TimeZone;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime, LocalResult};
use chrono_tz::Tz;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::combinator::{recognize, map, map_res, opt, cut};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, map_err_message};

use crate::grammar::latin_capital_letter_t;

use crate::values::{
    date::{date, Date},
    time::{time, Time},
};

// VALUE = ("DATE-TIME" / "DATE")
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValueType {
    DateTime,
    Date,
}

impl ICalendarEntity for ValueType {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "VALUE",
            map_err_message!(
                alt((
                    map(tag("DATE-TIME"), |_| ValueType::DateTime),
                    map(tag("DATE"), |_| ValueType::Date),
                )),
                "expected iCalendar RFC-5545 VALUE (\"DATE-TIME\" or \"DATE\")",
            ),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::DateTime => String::from("DATE-TIME"),
            Self::Date => String::from("DATE"),
        }
    }
}

impl ValueType {
    pub fn validate_against_date_time(&self, date_time: &DateTime) -> Result<(), String> {
        match (self, date_time) {
            (ValueType::DateTime, DateTime::UtcDateTime(_))   => Ok(()),
            (ValueType::DateTime, DateTime::LocalDateTime(_)) => Ok(()),
            (ValueType::Date,     DateTime::LocalDate(_))     => Ok(()),
            _ => Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        }
    }

    pub fn new_from_date_time(date_time: &DateTime) -> Self {
        match date_time {
            DateTime::UtcDateTime(_) => ValueType::DateTime,
            DateTime::LocalDateTime(_) => ValueType::DateTime,
            DateTime::LocalDate(_) => ValueType::Date,
        }
    }
}

impl_icalendar_entity_traits!(ValueType);

/// Parse date-time chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::values::date_time::date_time;
///
/// assert!(date_time("19970714".into()).is_ok());
/// assert!(date_time("19980118T230000".into()).is_ok());
///
/// assert!(date_time("1997071".into()).is_err());
/// assert!(date_time("19970714T".into()).is_err());
/// assert!(date_time("19980118T2300".into()).is_err());
/// assert!(date_time("c1997071/=".into()).is_err());
/// assert!(date_time(":".into()).is_err());
/// ```
///
/// date-time  = date "T" time ;As specified in the DATE and TIME
///                            ;value definitions
pub fn date_time(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "DATE-TIME",
        recognize(
            pair(
                date,
                opt(
                    preceded(
                        latin_capital_letter_t,
                        cut(time),
                    )
                ),
            )
        )
    )(input)
}

// Value Name:  DATE-TIME
//
// Purpose:  This value type is used to identify values that specify a
//    precise calendar date and time of day.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     date-time  = date "T" time ;As specified in the DATE and TIME
//                                ;value definitions
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum DateTime {
    LocalDate(NaiveDate),
    LocalDateTime(NaiveDateTime),
    UtcDateTime(NaiveDateTime),
}

impl ICalendarEntity for DateTime {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "DATE-TIME",
            map_res(
                pair(
                    Date::parse_ical,
                    opt(
                        preceded(
                            latin_capital_letter_t,
                            cut(Time::parse_ical),
                        )
                    ),
                ),
                |(date, time): (Date, Option<Time>)| -> Result<Self, String> {
                    let date = NaiveDate::try_from(date)?;

                    if let Some(time) = time {
                        if time.is_utc {
                            Ok(
                                Self::UtcDateTime(
                                    NaiveDateTime::new(
                                        date,
                                        NaiveTime::try_from(time)?,
                                    )
                                )
                            )
                        } else {
                            Ok(
                                Self::LocalDateTime(
                                    NaiveDateTime::new(
                                        date,
                                        NaiveTime::try_from(time)?,
                                    )
                                )
                            )
                        }
                    } else {
                        Ok(
                            Self::LocalDate(date)
                        )
                    }
                },
            )
        )(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        let tz = context.and_then(|context| context.tz.as_ref());

        self.render_formatted_date_time(tz)
    }
}

impl DateTime {
    /// Converts the `DateTime` to the provided timezone.
    /// If `current_tz` is `None` then it is assumed to be UTC.
    /// If `DateTime::UtcDateTime` and `current_tz` is specified to not be UTC, then it will
    /// silently ignore `current_tz` and be converted from UTC.
    pub fn with_timezone(&self, current_tz: Option<&Tz>, new_tz: &Tz) -> Self {
        let current_tz = current_tz.cloned().unwrap_or(Tz::UTC);

        match self {
            Self::LocalDate(date) => {
                let date_time: NaiveDateTime = date.to_owned().into();
                let utc_timestamp = current_tz.from_local_datetime(&date_time).unwrap().timestamp();
                let tz_adjusted_naive_date_time = new_tz.timestamp_opt(utc_timestamp, 0_u32).unwrap().naive_local();

                Self::LocalDate(tz_adjusted_naive_date_time.into())
            },

            Self::LocalDateTime(date_time) => {
                let utc_timestamp = current_tz.from_local_datetime(date_time).unwrap().timestamp();
                let tz_adjusted_naive_date_time = new_tz.timestamp_opt(utc_timestamp, 0_u32).unwrap().naive_local();

                Self::LocalDateTime(tz_adjusted_naive_date_time)
            },

            Self::UtcDateTime(date_time) => {
                if new_tz == &Tz::UTC {
                    self.clone()
                } else {
                    let utc_timestamp = Tz::UTC.from_local_datetime(date_time).unwrap().timestamp();
                    let tz_adjusted_naive_date_time = new_tz.timestamp_opt(utc_timestamp, 0_u32).unwrap().naive_local();

                    Self::LocalDateTime(tz_adjusted_naive_date_time)
                }
            },
        }
    }

    /// Returns the timestamp of the `DateTime` (adjusted to UTC from provided current timezone).
    /// If `current_tz` is `None` then it is assumed to be UTC.
    /// If `DateTime::UtcDateTime` and `current_tz` is specified to not be UTC, then it will
    /// silently ignore `current_tz` and be presumed UTC.
    pub fn get_utc_timestamp(&self, current_tz: Option<&Tz>) -> i64 {
        let current_tz = current_tz.cloned().unwrap_or(Tz::UTC);

        let date_time_result =
            match self {
                Self::LocalDate(date) => {
                    let date_time: NaiveDateTime = date.to_owned().into();

                    current_tz.from_local_datetime(&date_time)
                },

                Self::LocalDateTime(date_time) => {
                    current_tz.from_local_datetime(date_time)
                },

                Self::UtcDateTime(date_time) => {
                    Tz::UTC.from_local_datetime(date_time)
                },
            };

        date_time_result.unwrap()
                        .timestamp()
    }

    pub fn render_formatted_date_time(&self, tz: Option<&Tz>) -> String {
        let tz = tz.cloned().unwrap_or(Tz::UTC);

        match self {
            Self::LocalDate(date) => {
                Self::serialize_date(date, &tz)
            },

            Self::LocalDateTime(date_time) => {
                Self::serialize_date_time(date_time, &tz)
            },

            Self::UtcDateTime(date_time) => {
                if tz == Tz::UTC {
                    Self::serialize_date_time(date_time, &tz)
                } else {
                    self.with_timezone(Some(&Tz::UTC), &tz).render_formatted_date_time(Some(&tz))
                }
            },
        }
    }

    fn serialize_date_time(naive_date_time: &NaiveDateTime, tz: &Tz) -> String {
        let local_date_time = tz.from_local_datetime(naive_date_time).unwrap();

        if matches!(tz, &Tz::UTC) {
            local_date_time.format("%Y%m%dT%H%M%SZ").to_string()
        } else {
            local_date_time.format("%Y%m%dT%H%M%S").to_string()
        }
    }

    fn serialize_date(naive_date: &NaiveDate, tz: &Tz) -> String {
        let naive_date_time = NaiveDateTime::new(naive_date.to_owned(), NaiveTime::default());

        tz.from_local_datetime(&naive_date_time)
          .unwrap()
          .format("%Y%m%d")
          .to_string()
    }

}

impl From<i64> for DateTime {
    fn from(timestamp: i64) -> Self {
        match Tz::UTC.timestamp_opt(timestamp, 0) {
            LocalResult::Single(local_date_time) => {
                DateTime::UtcDateTime(local_date_time.naive_utc())
            },

            LocalResult::None => {
                // TODO: Consider handling this better
                panic!("Unable to parse ICalendar DateTime String from UTC timestamp: {timestamp} - none determined");
            },

            LocalResult::Ambiguous(earliest, latest) => {
                // TODO: Consider handling this better
                panic!("Unable to parse ICalendar DateTime String from UTC timestamp: {timestamp} - multiple determined - earliest: {earliest} latest: {latest}");
            },
        }
    }
}

impl_icalendar_entity_traits!(DateTime);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn date_time_parse_ical() {
        assert_parser_output!(
            DateTime::parse_ical("19970714 TESTING".into()),
            (
                " TESTING",
                DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
                ),
            ),
        );

        assert_parser_output!(
            DateTime::parse_ical("19980118T230000 TESTING".into()),
            (
                " TESTING",
                DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            ),
        );

        assert_parser_output!(
            DateTime::parse_ical("19980118T230000Z TESTING".into()),
            (
                " TESTING",
                DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            ),
        );

        assert!(DateTime::parse_ical("1997071".into()).is_err());
        assert!(DateTime::parse_ical("19970714T".into()).is_err());
        assert!(DateTime::parse_ical("19980118T2300".into()).is_err());
        assert!(DateTime::parse_ical("c1997071/=".into()).is_err());
        assert!(DateTime::parse_ical(":".into()).is_err());
    }

    #[test]
    fn value_type_parse_ical_error() {
        assert_parser_error!(
            ValueType::parse_ical(":".into()),
            nom::Err::Error(
                span: ":",
                message: "expected iCalendar RFC-5545 VALUE (\"DATE-TIME\" or \"DATE\")",
                context: ["VALUE"],
            ),
        );
    }

    #[test]
    fn date_time_parse_ical_error() {
        assert_parser_error!(
            DateTime::parse_ical(":".into()),
            nom::Err::Error(
                span: ":",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE-TIME", "DATE"],
            ),
        );

        assert_parser_error!(
            DateTime::parse_ical("20240".into()),
            nom::Err::Error(
                span: "0",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE-TIME", "DATE"],
            ),
        );

        assert_parser_error!(
            DateTime::parse_ical("2024020".into()),
            nom::Err::Error(
                span: "0",
                message: "expected iCalendar RFC-5545 DATE-VALUE (DATE-FULLYEAR DATE-MONTH DATE-MDAY)",
                context: ["DATE-TIME", "DATE"],
            ),
        );

        assert_parser_error!(
            DateTime::parse_ical("20240202T0".into()),
            nom::Err::Failure(
                span: "0",
                message: "expected iCalendar RFC-5545 TIME (TIME-HOUR TIME-MINUTE TIME-SECOND [TIME-UTC])",
                context: ["DATE-TIME", "TIME"],
            ),
        );

        assert_parser_error!(
            DateTime::parse_ical("20240202T0202d".into()),
            nom::Err::Failure(
                span: "d",
                message: "expected iCalendar RFC-5545 TIME (TIME-HOUR TIME-MINUTE TIME-SECOND [TIME-UTC])",
                context: ["DATE-TIME", "TIME"],
            ),
        );
    }

    #[test]
    fn date_time_render_ical() {
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).render_ical(),
            String::from("19970714"),
        );

        assert_eq!(
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical(),
            String::from("19980118T230000Z"),
        );

        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical(),
            String::from("19980118T230000Z"),
        );
    }

    #[test]
    fn date_time_render_ical_with_context_tz_override() {
        // UTC +02:00
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Vilnius), distance_unit: None })),
            String::from("19970714"),
        );

        // UTC -07:00
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("19970714"),
        );

        // UTC +02:00
        assert_eq!(
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Vilnius), distance_unit: None })),
            String::from("19980118T230000"),
        );

        // UTC -07:00
        assert_eq!(
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("19980118T230000"),
        );

        // UTC +02:00
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Vilnius), distance_unit: None })),
            String::from("19980119T010000"),
        );

        // UTC -07:00
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("19980118T160000"),
        );

        // UTC +00:00
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::UTC), distance_unit: None })),
            String::from("19980118T230000Z"),
        );
    }

    #[test]
    fn date_time_with_tz() {
        // Current tz is not provided so assume UTC
        // UTC -> UTC +02:00
        // Remains the same
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).with_timezone(None, &Tz::Europe__Vilnius),
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ),
        );

        // UTC +01:00 -> UTC +02:00
        // Remains the same
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).with_timezone(Some(&Tz::Europe__Warsaw), &Tz::Europe__Vilnius),
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ),
        );

        // UTC -> UTC -07:00
        // Changes to previous day (midnight - 7 hours)
        assert_eq!(
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
            ).with_timezone(Some(&Tz::UTC), &Tz::America__Phoenix),
            DateTime::LocalDate(
                NaiveDate::from_ymd_opt(1997_i32, 7_u32, 13_u32).unwrap()
            ),
        );

        // UTC -> UTC +02:00
        // Changes to 01:00:00 the next day (23:00:00 + 2 hours)
        assert_eq!(
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::UTC), &Tz::Europe__Vilnius),
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 19_u32).unwrap(),
                    NaiveTime::from_hms_opt(1_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );

        // UTC -> UTC -07:00
        // Stays on the same day but 7 hours earlier
        assert_eq!(
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::UTC), &Tz::America__Phoenix),
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(16_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );

        // UTC -> UTC +02:00
        // Changes to LocalDateTime at 01:00:00 the next day (23:00:00 + 2 hours)
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::UTC), &Tz::Europe__Vilnius),
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 19_u32).unwrap(),
                    NaiveTime::from_hms_opt(1_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );

        // UTC -> UTC -07:00
        // Stays on the same day but 7 hours earlier
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::UTC), &Tz::America__Phoenix),
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(16_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );

        // Misinformed of current TZ being -07:00 when UTC.
        // We just ignore it as we know it is UTC.
        // UTC -> UTC -07:00
        // Changes to LocalDateTime but staying on the same day but 7 hours earlier
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::America__Phoenix), &Tz::America__Phoenix),
            DateTime::LocalDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(16_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );

        // Misinformed of current TZ being -07:00 when UTC.
        // We just ignore it as we know it is UTC.
        // UTC -> UTC
        // Stays on UtcDateTime with the same time (essentially just clone)
        assert_eq!(
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ).with_timezone(Some(&Tz::America__Phoenix), &Tz::UTC),
            DateTime::UtcDateTime(
                NaiveDateTime::new(
                    NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                    NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                )
            ),
        );
    }

    #[test]
    fn value_type_parse_ical() {
        assert_parser_output!(
            ValueType::parse_ical(r#"DATE-TIME TESTING"#.into()),
            (
                " TESTING",
                ValueType::DateTime,
            ),
        );

        assert_parser_output!(
            ValueType::parse_ical(r#"DATE TESTING"#.into()),
            (
                " TESTING",
                ValueType::Date,
            ),
        );

        assert!(ValueType::parse_ical(":".into()).is_err());
    }

    #[test]
    fn value_type_validate_against_date_time() {
        assert_eq!(
            ValueType::Date.validate_against_date_time(
                &DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
                )
            ),
            Ok(()),
        );

        assert_eq!(
            ValueType::DateTime.validate_against_date_time(
                &DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1997_i32, 7_u32, 14_u32).unwrap()
                )
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );

        assert_eq!(
            ValueType::DateTime.validate_against_date_time(
                &DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                )
            ),
            Ok(()),
        );

        assert_eq!(
            ValueType::Date.validate_against_date_time(
                &DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                )
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );

        assert_eq!(
            ValueType::DateTime.validate_against_date_time(
                &DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                )
            ),
            Ok(()),
        );

        assert_eq!(
            ValueType::Date.validate_against_date_time(
                &DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
                        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
                    )
                )
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );
    }

    #[test]
    fn value_type_render_ical() {
        assert_eq!(
            ValueType::DateTime.render_ical(),
            String::from("DATE-TIME"),
        );

        assert_eq!(
            ValueType::Date.render_ical(),
            String::from("DATE"),
        );
    }
}
