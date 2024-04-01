use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::property_value_data_types::date_time::DateTime;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// Unofficial parameter for describing DateTime value types:
//
// VALUE = ("DATE-TIME" / "DATE")
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    DateTime,
    Date,
}

impl ICalendarEntity for Value {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "VALUE",
            alt((
                map(tag("DATE-TIME"), |_| Value::DateTime),
                map(tag("DATE"), |_| Value::Date),
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

impl Value {
    pub fn validate_against_date_time(&self, date_time: &DateTime) -> Result<(), String> {
        match (self, date_time) {
            (Value::DateTime, DateTime { date: _, time: Some(_) }) => Ok(()),
            (Value::Date,     DateTime { date: _, time: None })    => Ok(()),
            _ => Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        }
    }
}

impl_icalendar_entity_traits!(Value);

// Unofficial parameter for describing DateTime value types:
//
// VALUE = ("DATE-TIME" / "DATE")
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ValueParam(pub Value);

impl ICalendarEntity for ValueParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "VALUEPARAM",
            map(
                pair(
                    tag("VALUE"),
                    preceded(tag("="), cut(Value::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("VALUE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(ValueParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::{
        date::Date,
        time::Time,
    };

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ValueParam::parse_ical(r#"VALUE=DATE-TIME TESTING"#.into()),
            (
                " TESTING",
                ValueParam(Value::DateTime),
            ),
        );

        assert_parser_output!(
            ValueParam::parse_ical(r#"VALUE=DATE TESTING"#.into()),
            (
                " TESTING",
                ValueParam(Value::Date),
            ),
        );

        assert!(ValueParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn validate_against_date_time() {
        assert_eq!(
            Value::DateTime.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }),
                },
            ),
            Ok(()),
        );

        assert_eq!(
            Value::DateTime.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: None,
                },
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );

        assert_eq!(
            Value::Date.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: None,
                },
            ),
            Ok(()),
        );

        assert_eq!(
            Value::Date.validate_against_date_time(
                &DateTime {
                    date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 },
                    time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }),
                },
            ),
            Err(String::from("VALUE incompatible with parsed DATE-TIME/DATE value")),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ValueParam(Value::DateTime).render_ical(),
            String::from("VALUE=DATE-TIME"),
        );

        assert_eq!(
            ValueParam(Value::Date).render_ical(),
            String::from("VALUE=DATE"),
        );
    }
}
