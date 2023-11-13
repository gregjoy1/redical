use chrono::TimeZone;
use rrule::Tz;

/// Generates an iCalendar date-time string format with the prefix symbols.
/// Like: `:19970714T173000Z` or `;TZID=America/New_York:19970714T133000`
/// ref: <https://tools.ietf.org/html/rfc5545#section-3.3.5>
pub fn serialize_timestamp_to_ical_datetime(utc_timestamp: i64, timezone: Tz) -> String {
    let mut timezone_prefix  = String::new();
    let mut timezone_postfix = String::new();

    let local_datetime = timezone.timestamp_opt(utc_timestamp, 0).unwrap();

    if let Tz::Tz(timezone) = timezone {
        match timezone {
            chrono_tz::UTC => {
                timezone_postfix = "Z".to_string();
            }
            timezone => {
                timezone_prefix = format!(";TZID={}", timezone.name());
            }
        }
    }

    let serialized_datetime = local_datetime.format("%Y%m%dT%H%M%S");

    format!("{}:{}{}", timezone_prefix, serialized_datetime, timezone_postfix)
}

pub fn serialize_timestamp_to_ical_utc_datetime(timestamp: i64) -> String {
    Tz::UTC.timestamp_opt(timestamp, 0)
           .unwrap()
           .format("%Y%m%dT%H%M%SZ")
           .to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_serialize_timestamp_to_ical_datetime() {
        let timestamp: i64 = 1699833600; // 2023-11-13 00:00:00 +0000

        assert_eq!(
            serialize_timestamp_to_ical_datetime(timestamp, Tz::UTC),
            String::from(":20231113T000000Z"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(timestamp, Tz::Europe__London),
            String::from(";TZID=Europe/London:20231113T000000"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(timestamp, Tz::Europe__Vilnius),
            String::from(";TZID=Europe/Vilnius:20231113T020000"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(timestamp, Tz::Europe__Amsterdam),
            String::from(";TZID=Europe/Amsterdam:20231113T010000"),
        );
    }

    #[test]
    fn test_serialize_timestamp_to_utc_ical_datetime() {
        assert_eq!(
            serialize_timestamp_to_ical_utc_datetime(1699833600), // 2023-11-13 00:00:00 +0000
            String::from("20231113T000000Z"),
        );
    }
}
