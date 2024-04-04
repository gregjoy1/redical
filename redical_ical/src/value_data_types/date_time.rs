use chrono::prelude::TimeZone;
use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
use chrono_tz::Tz;

use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::combinator::{recognize, map, map_res, opt, cut};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::grammar::latin_capital_letter_t;

use crate::value_data_types::{
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
            alt((
                map(tag("DATE-TIME"), |_| ValueType::DateTime),
                map(tag("DATE"), |_| ValueType::Date),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
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
}

impl_icalendar_entity_traits!(ValueType);

/// Parse date-time chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::value_data_types::date_time::date_time;
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

    fn render_ical(&self) -> String {
        self.serialize_ical(None)
    }
}

impl DateTime {
    pub fn serialize_ical(&self, tz: Option<&Tz>) -> String {
        let tz = tz.cloned().unwrap_or(Tz::UTC);

        match self {
            Self::LocalDate(date) => {
                // TODO: Render with context of property.
                Self::serialize_date(date, &tz)
            },

            Self::LocalDateTime(date_time) => {
                // TODO: Render with context of property.
                Self::serialize_date_time(date_time, &tz)
            },

            Self::UtcDateTime(date_time) => {
                if tz == Tz::UTC {
                    Self::serialize_date_time(date_time, &tz)
                } else {
                    let utc_timestamp = Tz::UTC.from_local_datetime(date_time).unwrap().timestamp();
                    let tz_adjusted_naive_date_time = tz.timestamp_opt(utc_timestamp, 0_u32).unwrap().naive_local();

                    Self::serialize_date_time(&tz_adjusted_naive_date_time, &tz)
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

impl_icalendar_entity_traits!(DateTime);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

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
            String::from("19980118T230000Z"), // TODO: Update to without Z suffix with render_ical_with_context change.
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
