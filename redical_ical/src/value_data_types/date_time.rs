use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::combinator::{recognize, map, opt, cut};

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
            (ValueType::DateTime, DateTime { date: _, time: Some(_) }) => Ok(()),
            (ValueType::Date,     DateTime { date: _, time: None })    => Ok(()),
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
pub struct DateTime {
    pub date: Date,
    pub time: Option<Time>,
}

impl ICalendarEntity for DateTime {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "DATE-TIME",
            map(
                pair(
                    Date::parse_ical,
                    opt(
                        preceded(
                            latin_capital_letter_t,
                            cut(Time::parse_ical),
                        )
                    ),
                ),
                |(date, time)| {
                    Self {
                        date,
                        time,
                    }
                },
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        if let Some(time) = self.time.as_ref() {
            format!("{}T{}", self.date.render_ical(), time.render_ical())
        } else {
            self.date.render_ical()
        }
    }

    fn validate(&self) -> Result<(), String> {
        self.date.validate()?;

        if let Some(time) = self.time.as_ref() {
            time.validate()?;
        }

        Ok(())
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
                DateTime {
                    date: Date {
                        year: 1997_i32,
                        month: 7_u32,
                        day: 14_u32,
                    },
                    time: None,
                },
            ),
        );

        assert_parser_output!(
            DateTime::parse_ical("19980118T230000Z TESTING".into()),
            (
                " TESTING",
                DateTime {
                    date: Date {
                        year: 1998_i32,
                        month: 1_u32,
                        day: 18_u32,
                    },
                    time: Some(
                        Time {
                            hour: 23_u32,
                            minute: 0_u32,
                            second: 0_u32,
                            is_utc: true,
                        }
                    )
                },
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
            DateTime {
                date: Date {
                    year: 1997_i32,
                    month: 7_u32,
                    day: 14_u32,
                },
                time: None,
            }.render_ical(),
            String::from("19970714"),
        );

        assert_eq!(
            DateTime {
                date: Date {
                    year: 1998_i32,
                    month: 1_u32,
                    day: 18_u32,
                },
                time: Some(
                    Time{
                        hour: 23_u32,
                        minute: 0_u32,
                        second: 0_u32,
                        is_utc: true
                    }
                )
            }.render_ical(),
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
            ValueType::DateTime.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }),
                },
            ),
            Ok(()),
        );

        assert_eq!(
            ValueType::DateTime.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: None,
                },
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );

        assert_eq!(
            ValueType::Date.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: None,
                },
            ),
            Ok(()),
        );

        assert_eq!(
            ValueType::Date.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }),
                },
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
