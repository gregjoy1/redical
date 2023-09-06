//! DateTime parsing utility function (and all dependencies) extracted and copied out of the rust-rrule crate.
// As the rust-rrule crate only accepts and parses iCal DTSTART, RDATE, and EXDATE properties, we are unable
// to parse DTEND, and DURATION properties (either used in determining a duration). Unfortunately the parser
// used is a private submodule in the rust-rrule crate and inaccessible to us, so we had to copy and extract
// all code in here so that it can be used freely by us.
//
// Files copied/extracted from:
// * https://github.com/fmeringdal/rust-rrule/blob/v0.10.0/rrule/src/parser/datetime.rs
// * https://github.com/fmeringdal/rust-rrule/blob/v0.10.0/rrule/src/parser/regex.rs
// * https://github.com/fmeringdal/rust-rrule/blob/v0.10.0/rrule/src/core/datetime.rs

use std::str::FromStr;

use lazy_static::lazy_static;
use regex::{Captures, Regex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidTimezone(String),
    InvalidDateTime { value: String, property: String },
    InvalidDateTimeFormat(String),
    InvalidDateTimeInLocalTimezone { value: String, property: String },
    DateTimeInLocalTimezoneIsAmbiguous {
        value: String,
        property: String,
        date1: String,
        date2: String,
    },
}

lazy_static! {
    static ref DATESTR_RE: Regex =
        Regex::new(r"(?m)^([0-9]{4})([0-9]{2})([0-9]{2})(T([0-9]{2})([0-9]{2})([0-9]{2})(Z?))?$")
            .expect("DATESTR_RE regex failed");
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedDateString {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub time: Option<ParsedDateStringTime>,
    pub flags: ParsedDateStringFlags,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedDateStringFlags {
    pub zulu_timezone_set: bool,
}

#[derive(Debug, PartialEq)]
pub(crate) struct ParsedDateStringTime {
    pub hour: u32,
    pub min: u32,
    pub sec: u32,
}

fn get_datetime_captures<T: FromStr>(
    captures: &Captures,
    idx: usize,
    val: &str,
) -> Result<T, ParseError> {
    captures
        .get(idx)
        .ok_or_else(|| ParseError::InvalidDateTimeFormat(val.into()))?
        .as_str()
        .parse()
        .map_err(|_| ParseError::InvalidDateTimeFormat(val.into()))
}

impl ParsedDateString {
    /// Parses a date string with format `YYYYMMDD(THHMMSSZ)` where the part in parentheses
    /// is optional. It returns [`ParsedDateString`].
    pub(crate) fn from_ical_datetime(val: &str) -> Result<Self, ParseError> {
        let captures = DATESTR_RE
            .captures(val)
            .ok_or_else(|| ParseError::InvalidDateTimeFormat(val.into()))?;

        let year = get_datetime_captures(&captures, 1, val)?;
        let month = get_datetime_captures(&captures, 2, val)?;
        let day = get_datetime_captures(&captures, 3, val)?;

        // Check if time part is captured
        let time = if captures.get(4).is_some() {
            let hour = get_datetime_captures(&captures, 5, val)?;
            let min = get_datetime_captures(&captures, 6, val)?;
            let sec = get_datetime_captures(&captures, 7, val)?;
            Some(ParsedDateStringTime { hour, min, sec })
        } else {
            None
        };

        let zulu_timezone_set = match captures.get(8) {
            Some(part) => part.as_str() == "Z",
            None => false,
        };
        let flags = ParsedDateStringFlags { zulu_timezone_set };

        Ok(Self {
            year,
            month,
            day,
            time,
            flags,
        })
    }
}

// =========

use rrule::Tz;

// use crate::core::{DateTime, Tz};

pub type DateTime = chrono::DateTime<Tz>;

use chrono::{NaiveDate, TimeZone};

/// Attempts to convert a `str` to a `chrono_tz::Tz`.
pub(crate) fn parse_timezone(tz: &str) -> Result<Tz, ParseError> {
    chrono_tz::Tz::from_str(tz)
        .map_err(|_| ParseError::InvalidTimezone(tz.into()))
        .map(Tz::Tz)
}

/// Convert a datetime string and a timezone to a `chrono::DateTime<Tz>`.
/// If the string specifies a zulu timezone with `Z`, then the timezone
/// argument will be ignored.
pub(crate) fn datestring_to_date(
    dt: &str,
    tz: Option<Tz>,
    property: &str,
) -> Result<DateTime, ParseError> {
    let ParsedDateString {
        year,
        month,
        day,
        time,
        flags,
    } = ParsedDateString::from_ical_datetime(dt).map_err(|_| ParseError::InvalidDateTime {
        value: dt.into(),
        property: property.into(),
    })?;

    // Combine parts to create data time.
    let date =
        NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| ParseError::InvalidDateTime {
            value: dt.into(),
            property: property.into(),
        })?;

    // Spec defines this is a date-time OR date
    // So the time can will be set to 0:0:0 if only a date is given.
    // https://icalendar.org/iCalendar-RFC-5545/3-8-2-4-date-time-start.html
    let (hour, min, sec) = if let Some(time) = time {
        (time.hour, time.min, time.sec)
    } else {
        (0, 0, 0)
    };
    let datetime = date
        .and_hms_opt(hour, min, sec)
        .ok_or_else(|| ParseError::InvalidDateTime {
            value: dt.into(),
            property: property.into(),
        })?;

    // Apply timezone appended to the datetime before converting to UTC.
    // For more info https://icalendar.org/iCalendar-RFC-5545/3-3-5-date-time.html
    let datetime: chrono::DateTime<Tz> = if flags.zulu_timezone_set {
        // If a `Z` is present, UTC should be used.
        chrono::DateTime::<chrono::Utc>::from_utc(datetime, chrono::Utc).with_timezone(&Tz::UTC)
    } else {
        // If no `Z` is present, local time should be used.
        use chrono::offset::LocalResult;
        // Get datetime in local time or machine local time.
        // So this also takes into account daylight or standard time (summer/winter).
        if let Some(tz) = tz {
            // Use the timezone specified in the `tz`
            match tz.from_local_datetime(&datetime) {
                LocalResult::None => Err(ParseError::InvalidDateTimeInLocalTimezone {
                    value: dt.into(),
                    property: property.into(),
                }),
                LocalResult::Single(date) => Ok(date),
                LocalResult::Ambiguous(date1, date2) => {
                    Err(ParseError::DateTimeInLocalTimezoneIsAmbiguous {
                        value: dt.into(),
                        property: property.into(),
                        date1: date1.to_rfc3339(),
                        date2: date2.to_rfc3339(),
                    })
                }
            }?
        } else {
            // Use current system timezone
            // TODO Add option to always use UTC when this is executed on a server.
            let local = Tz::LOCAL;
            match local.from_local_datetime(&datetime) {
                LocalResult::None => {
                    return Err(ParseError::InvalidDateTimeInLocalTimezone {
                        value: dt.into(),
                        property: property.into(),
                    })
                }
                LocalResult::Single(date) => date,
                LocalResult::Ambiguous(date1, date2) => {
                    return Err(ParseError::DateTimeInLocalTimezoneIsAmbiguous {
                        value: dt.into(),
                        property: property.into(),
                        date1: date1.to_rfc3339(),
                        date2: date2.to_rfc3339(),
                    })
                }
            }
        }
    };

    Ok(datetime)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GARBAGE_INPUTS: [&str; 4] = ["", "  ", "fasfa!2414", "-20101017T120000Z"];

    #[test]
    fn parses_valid_datestrings_correctly() {
        let tests = [
            (
                "20101017T120000Z",
                ParsedDateString {
                    year: 2010,
                    month: 10,
                    day: 17,
                    time: Some(ParsedDateStringTime {
                        hour: 12,
                        min: 0,
                        sec: 0,
                    }),
                    flags: ParsedDateStringFlags {
                        zulu_timezone_set: true,
                    },
                },
            ),
            (
                "20101017",
                ParsedDateString {
                    year: 2010,
                    month: 10,
                    day: 17,
                    time: None,
                    flags: ParsedDateStringFlags {
                        zulu_timezone_set: false,
                    },
                },
            ),
            (
                "20220101T121049Z",
                ParsedDateString {
                    year: 2022,
                    month: 1,
                    day: 1,
                    time: Some(ParsedDateStringTime {
                        hour: 12,
                        min: 10,
                        sec: 49,
                    }),
                    flags: ParsedDateStringFlags {
                        zulu_timezone_set: true,
                    },
                },
            ),
            (
                "20220101",
                ParsedDateString {
                    year: 2022,
                    month: 1,
                    day: 1,
                    time: None,
                    flags: ParsedDateStringFlags {
                        zulu_timezone_set: false,
                    },
                },
            ),
        ];
        for (input, expected_output) in tests {
            let output = ParsedDateString::from_ical_datetime(input);
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_datestrings() {
        let tests = [
            GARBAGE_INPUTS.to_vec(),
            [
                "-20101017T120000Z",
                "20101017T",
                "201010177",
                "20101017T1200",
                "210101017T1200",
            ]
            .to_vec(),
        ]
        .concat();
        for input in tests {
            let res = ParsedDateString::from_ical_datetime(input);
            assert!(res.is_err());
        }
    }

    const US_PACIFIC: Tz = Tz::US__Pacific;

    #[test]
    fn parses_valid_datestime_str() {
        let tests = [
            (
                "19970902T090000Z",
                None,
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000",
                Some(Tz::UTC),
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000",
                Some(US_PACIFIC),
                US_PACIFIC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
            (
                "19970902T090000Z",
                Some(US_PACIFIC),
                // Timezone is overwritten by the zulu specified in the datetime string
                Tz::UTC.with_ymd_and_hms(1997, 9, 2, 9, 0, 0).unwrap(),
            ),
        ];

        for (datetime_str, timezone, expected_output) in tests {
            let output = datestring_to_date(datetime_str, timezone, "DTSTART");
            assert_eq!(output, Ok(expected_output));
        }
    }

    #[test]
    fn rejects_invalid_datetime_str() {
        let tests = [
            ("", None),
            ("TZID=America/New_York:19970902T090000", None),
            ("19970902T09", None),
            ("19970902T09", Some(US_PACIFIC)),
        ];

        for (datetime_str, timezone) in tests {
            let res = datestring_to_date(datetime_str, timezone, "DTSTART");
            assert!(res.is_err());
        }
    }
}
