use serde::{Deserialize, Serialize};

use nom::{
    bytes::complete::tag,
    character::complete::digit1,
    combinator::opt,
    error::{context, VerboseError},
    sequence::{preceded, terminated, tuple},
    IResult,
};

pub type ParserResult<T, U> = IResult<T, U, VerboseError<T>>;

const SECONDS_IN_MINUTE: i64 = 60;
const SECONDS_IN_HOUR: i64 = SECONDS_IN_MINUTE * 60;
const SECONDS_IN_DAY: i64 = SECONDS_IN_HOUR * 24;
const SECONDS_IN_WEEK: i64 = SECONDS_IN_DAY * 7;

pub fn parse_duration_string_components(
    input: &str,
) -> ParserResult<
    &str,
    (
        Option<&str>,
        Option<&str>,
        Option<(&str, Option<&str>, Option<&str>, Option<&str>)>,
    ),
> {
    context(
        "parsed duration",
        preceded(
            tag("P"),
            tuple((
                opt(terminated(digit1, tag("W"))),
                opt(terminated(digit1, tag("D"))),
                opt(tuple((
                    tag("T"),
                    opt(terminated(digit1, tag("H"))),
                    opt(terminated(digit1, tag("M"))),
                    opt(terminated(digit1, tag("S"))),
                ))),
            )),
        ),
    )(input)
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct ParsedDuration {
    pub weeks: Option<i64>,
    pub days: Option<i64>,
    pub hours: Option<i64>,
    pub minutes: Option<i64>,
    pub seconds: Option<i64>,
}

impl ParsedDuration {
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

        duration_in_seconds
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }

    pub fn to_ical(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut output = String::from("P");

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

        Some(output)
    }
}

impl Default for ParsedDuration {
    fn default() -> Self {
        ParsedDuration {
            weeks: None,
            days: None,
            hours: None,
            minutes: None,
            seconds: None,
        }
    }
}

impl TryFrom<&str> for ParsedDuration {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match parse_duration_string_components(value) {
            Ok((remaining, parsed_duration_string_components)) => {
                if remaining.is_empty() {
                    let mut parsed_duration = ParsedDuration::default();

                    let (weeks, days, time_component) = parsed_duration_string_components;

                    if let Some(weeks) = weeks {
                        let parsed_weeks = str::parse::<i64>(weeks).map_err(|_error| {
                            format!("Could not parse numeric duration weeks value: {weeks}")
                        })?;

                        parsed_duration.weeks = Some(parsed_weeks);
                    }

                    if let Some(days) = days {
                        let parsed_days = str::parse::<i64>(days).map_err(|_error| {
                            format!("Could not parse numeric duration days value: {days}")
                        })?;

                        parsed_duration.days = Some(parsed_days);
                    }

                    if let Some((_time_delim, hours, minutes, seconds)) = time_component {
                        if let Some(hours) = hours {
                            let parsed_hours = str::parse::<i64>(hours).map_err(|_error| {
                                format!("Could not parse numeric duration hours value: {hours}")
                            })?;

                            parsed_duration.hours = Some(parsed_hours);
                        }

                        if let Some(minutes) = minutes {
                            let parsed_minutes = str::parse::<i64>(minutes).map_err(|_error| {
                                format!("Could not parse numeric duration minutes value: {minutes}")
                            })?;

                            parsed_duration.minutes = Some(parsed_minutes);
                        }

                        if let Some(seconds) = seconds {
                            let parsed_seconds = str::parse::<i64>(seconds).map_err(|_error| {
                                format!("Could not parse numeric duration seconds value: {seconds}")
                            })?;

                            parsed_duration.seconds = Some(parsed_seconds);
                        }
                    }

                    Ok(parsed_duration)
                } else {
                    Err(format!("Unexpected values: '{remaining}'"))
                }
            }

            Err(error) => Err(error.to_string()),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_duration() {
        assert_eq!(
            ParsedDuration::try_from("P15--INVALID20S"),
            Err(String::from("Unexpected values: '15--INVALID20S'"))
        );

        assert_eq!(
            ParsedDuration::try_from("P7W SOMETHING ELSE"),
            Err(String::from("Unexpected values: ' SOMETHING ELSE'"))
        );

        assert_eq!(
            ParsedDuration::try_from("P15DT5H0M20S"),
            Ok(ParsedDuration {
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
            })
        );

        assert_eq!(
            ParsedDuration::try_from("P7W"),
            Ok(ParsedDuration {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            })
        );

        assert_eq!(
            ParsedDuration::try_from("PT25S"),
            Ok(ParsedDuration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
            })
        );
    }

    #[test]
    fn test_get_duration_in_seconds() {
        assert_eq!(ParsedDuration::default().get_duration_in_seconds(), 0);

        assert_eq!(
            ParsedDuration {
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
            ParsedDuration {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            }
            .get_duration_in_seconds(),
            (((60 * 60) * 24) * 7) * 7,
        );

        assert_eq!(
            ParsedDuration {
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
    fn test_to_ical() {
        assert_eq!(ParsedDuration::default().to_ical(), None);

        assert_eq!(
            ParsedDuration {
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
            }
            .to_ical(),
            Some(String::from("P15DT5H0M20S")),
        );

        assert_eq!(
            ParsedDuration {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            }
            .to_ical(),
            Some(String::from("P7W")),
        );

        assert_eq!(
            ParsedDuration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
            }
            .to_ical(),
            Some(String::from("PT25S")),
        );
    }
}
