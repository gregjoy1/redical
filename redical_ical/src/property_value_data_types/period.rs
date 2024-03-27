use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::error::context;
use nom::combinator::{recognize, map};
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::date_time::DateTime;
use crate::property_value_data_types::duration::Duration;

/// Parse period chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::property_value_data_types::period::period;
///
/// assert!(period("19970101T180000Z/19970102T070000Z".into()).is_ok());
/// assert!(period("19970101T180000Z/PT5H30M".into()).is_ok());
///
/// assert!(period("1997071".into()).is_err());
/// assert!(period("19970714T".into()).is_err());
/// assert!(period("19980118T2300".into()).is_err());
/// assert!(period("c1997071/=".into()).is_err());
/// assert!(period(":".into()).is_err());
/// ```
///
/// period     = period-explicit / period-start
pub fn period(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "PERIOD",
        alt((
            recognize(period_explicit),
            recognize(period_start),
        ))
    )(input)
}

/// period-explicit = date-time "/" date-time
/// ; [ISO.8601.2004] complete representation basic format for a
/// ; period of time consisting of a start and end.  The start MUST
/// ; be before the end.
pub fn period_explicit(input: ParserInput) -> ParserResult<(DateTime, DateTime)> {
    pair(
        DateTime::parse_ical,
        preceded(
            tag("/"),
            DateTime::parse_ical,
        ),
    )(input)
}

/// period-start = date-time "/" dur-value
/// ; [ISO.8601.2004] complete representation basic format for a
/// ; period of time consisting of a start and positive duration
/// ; of time.
pub fn period_start(input: ParserInput) -> ParserResult<(DateTime, Duration)> {
    pair(
        DateTime::parse_ical,
        preceded(
            tag("/"),
            Duration::parse_ical,
        ),
    )(input)
}

// Value Name:  PERIOD
//
// Purpose:  This value type is used to identify values that contain a
//    precise period of time.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     period     = period-explicit / period-start
//
//     period-explicit = date-time "/" date-time
//     ; [ISO.8601.2004] complete representation basic format for a
//     ; period of time consisting of a start and end.  The start MUST
//     ; be before the end.
//
//     period-start = date-time "/" dur-value
//     ; [ISO.8601.2004] complete representation basic format for a
//     ; period of time consisting of a start and positive duration
//     ; of time.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Period {
    Explicit(DateTime, DateTime),
    Start(DateTime, Duration),
}

impl ICalendarEntity for Period {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        context(
            "PERIOD",
            alt((
                map(period_explicit, |(start, end)| Self::Explicit(start, end)),
                map(period_start, |(start, duration)| Self::Start(start, duration)),
            ))
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Explicit(start, end) => {
                format!("{}/{}", start.render_ical(), end.render_ical())
            },

            Self::Start(start, duration) => {
                format!("{}/{}", start.render_ical(), duration.render_ical())
            },
        }
    }
}

impl_icalendar_entity_traits!(Period);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Period::parse_ical("19970101T180000Z/19970102T070000Z TESTING".into()),
            (
                " TESTING",
                Period::Explicit(
                    DateTime::from_str("19970101T180000Z").unwrap(),
                    DateTime::from_str("19970102T070000Z").unwrap(),
                ),
            ),
        );

        assert_parser_output!(
            Period::parse_ical("19970101T180000Z/PT5H30M TESTING".into()),
            (
                " TESTING",
                Period::Start(
                    DateTime::from_str("19970101T180000Z").unwrap(),
                    Duration::from_str("PT5H30M").unwrap(),
                ),
            ),
        );

        assert!(Period::parse_ical("1997071".into()).is_err());
        assert!(Period::parse_ical("19970714T".into()).is_err());
        assert!(Period::parse_ical("19980118T2300".into()).is_err());
        assert!(Period::parse_ical("c1997071/=".into()).is_err());
        assert!(Period::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Period::Explicit(
                DateTime::from_str("19970101T180000Z").unwrap(),
                DateTime::from_str("19970102T070000Z").unwrap(),
            ).render_ical(),
            String::from("19970101T180000Z/19970102T070000Z"),
        );

        assert_eq!(
            Period::Start(
                DateTime::from_str("19970101T180000Z").unwrap(),
                Duration::from_str("PT5H30M").unwrap(),
            ).render_ical(),
            String::from("19970101T180000Z/PT5H30M"),
        );
    }
}
