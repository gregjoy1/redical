use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::combinator::{recognize, map, opt, cut};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::grammar::latin_capital_letter_t;

use crate::property_value_data_types::{
    date::{date, Date},
    time::{time, Time},
};

/// Parse date-time chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::property_value_data_types::date_time::date_time;
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
    fn parse_ical() {
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
    fn render_ical() {
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
}
