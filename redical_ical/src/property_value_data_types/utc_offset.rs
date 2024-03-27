use nom::combinator::map;
use nom::sequence::tuple;
use nom::combinator::opt;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::grammar::PositiveNegative;

use crate::property_value_data_types::integer::Integer;
use crate::property_value_data_types::time::{time_hour, time_minute, time_second};

// Value Name:  UTC-OFFSET
//
// Purpose:  This value type is used to identify properties that contain
//    an offset from UTC to local time.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     utc-offset = time-numzone
//
//     time-numzone = ("+" / "-") time-hour time-minute [time-second]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct UtcOffset {
    pub positive_negative: PositiveNegative,
    pub time_hour: Integer,
    pub time_minute: Integer,
    pub time_second: Option<Integer>,
}

impl ICalendarEntity for UtcOffset {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(
            tuple((
                PositiveNegative::parse_ical,
                time_hour,
                time_minute,
                opt(time_second),
            )),
            |(positive_negative, time_hour, time_minute, opt_time_second)| {
                UtcOffset {
                    positive_negative,
                    time_hour: time_hour.into(),
                    time_minute: time_minute.into(),
                    time_second: opt_time_second.and_then(|time_second| Some(Integer::from(time_second))),
                }
            }
        )(input)
    }

    fn render_ical(&self) -> String {
        if let Some(time_second) = self.time_second.to_owned() {
            format!(
                "{}{:02}{:02}{:02}",
                self.positive_negative.render_ical(),
                *self.time_hour,
                *self.time_minute,
                *time_second,
            )
        } else {
            format!(
                "{}{:02}{:02}",
                self.positive_negative.render_ical(),
                *self.time_hour,
                *self.time_minute,
            )
        }
    }
}

impl_icalendar_entity_traits!(UtcOffset);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            UtcOffset::parse_ical("-0500 TESTING".into()),
            (
                " TESTING",
                UtcOffset {
                    positive_negative: PositiveNegative::Negative,
                    time_hour: Integer(5),
                    time_minute: Integer(0),
                    time_second: None,
                },
            ),
        );

        assert_parser_output!(
            UtcOffset::parse_ical("+0100 TESTING".into()),
            (
                " TESTING",
                UtcOffset {
                    positive_negative: PositiveNegative::Positive,
                    time_hour: Integer(1),
                    time_minute: Integer(0),
                    time_second: None,
                },
            ),
        );

        assert_parser_output!(
            UtcOffset::parse_ical("+112050 TESTING".into()),
            (
                " TESTING",
                UtcOffset {
                    positive_negative: PositiveNegative::Positive,
                    time_hour: Integer(11),
                    time_minute: Integer(20),
                    time_second: Some(Integer(50)),
                },
            ),
        );

        assert!(UtcOffset::parse_ical("0505".into()).is_err());
        assert!(UtcOffset::parse_ical("2525".into()).is_err());
        assert!(UtcOffset::parse_ical("++".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            UtcOffset {
                positive_negative: PositiveNegative::Negative,
                time_hour: Integer(5),
                time_minute: Integer(0),
                time_second: None,
            }.render_ical(),
            String::from("-0500"),
        );

        assert_eq!(
            UtcOffset {
                positive_negative: PositiveNegative::Positive,
                time_hour: Integer(1),
                time_minute: Integer(0),
                time_second: None,
            }.render_ical(),
            String::from("+0100"),
        );

        assert_eq!(
            UtcOffset {
                positive_negative: PositiveNegative::Positive,
                time_hour: Integer(11),
                time_minute: Integer(20),
                time_second: Some(Integer(50)),
            }.render_ical(),
            String::from("+112050"),
        );
    }
}

