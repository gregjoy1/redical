use std::collections::{BTreeSet, HashMap, HashSet};

use crate::core::parsers::duration::ParsedDuration;
use crate::core::{GeoPoint, KeyValuePair};

use chrono::TimeZone;
use rrule::Tz;

/// Generates an iCalendar date-time string format with the prefix symbols.
/// Like: `:19970714T173000Z` or `;TZID=America/New_York:19970714T133000`
/// ref: <https://tools.ietf.org/html/rfc5545#section-3.3.5>
pub fn serialize_timestamp_to_ical_datetime(utc_timestamp: &i64, timezone: &Tz) -> String {
    let mut timezone_prefix = String::new();
    let mut timezone_postfix = String::new();

    let local_datetime = timezone.timestamp_opt(utc_timestamp.clone(), 0).unwrap();

    if let Tz::Tz(timezone) = timezone {
        match timezone {
            &chrono_tz::UTC => {
                timezone_postfix = "Z".to_string();
            }

            &timezone => {
                timezone_prefix = format!(";TZID={}", timezone.name());
            }
        }
    }

    let serialized_datetime = local_datetime.format("%Y%m%dT%H%M%S");

    format!(
        "{}:{}{}",
        timezone_prefix, serialized_datetime, timezone_postfix
    )
}

pub fn serialize_timestamp_to_ical_utc_datetime(timestamp: &i64) -> String {
    Tz::UTC
        .timestamp_opt(timestamp.clone(), 0)
        .unwrap()
        .format("%Y%m%dT%H%M%SZ")
        .to_string()
}

pub fn serialize_indexed_categories_to_ical_set(
    categories: &Option<HashSet<String>>,
) -> BTreeSet<KeyValuePair> {
    let mut categories_ical_set = BTreeSet::new();

    let Some(categories) = categories else {
        return categories_ical_set;
    };

    let mut categories: Vec<String> =
        Vec::from_iter(categories.iter().map(|element| element.to_owned()));

    categories.sort();

    if categories.len() > 0 {
        categories_ical_set.insert(KeyValuePair::new(
            String::from("CATEGORIES"),
            format!(":{}", categories.join(",")),
        ));
    }

    categories_ical_set
}

pub fn serialize_indexed_related_to_ical_set(
    related_to: &Option<HashMap<String, HashSet<String>>>,
) -> BTreeSet<KeyValuePair> {
    let mut related_to_ical_set = BTreeSet::new();

    let Some(related_to) = related_to else {
        return related_to_ical_set;
    };

    for (reltype, reltype_uuids) in related_to {
        if reltype_uuids.is_empty() {
            continue;
        }

        let mut reltype_uuids: Vec<String> =
            Vec::from_iter(reltype_uuids.iter().map(|element| element.to_owned()));

        reltype_uuids.sort();

        reltype_uuids.iter().for_each(|reltype_uuid| {
            related_to_ical_set.insert(KeyValuePair::new(
                String::from("RELATED-TO"),
                format!(";RELTYPE={}:{}", reltype, reltype_uuid),
            ));
        });
    }

    related_to_ical_set
}

pub fn serialize_indexed_geo_to_ical(geo_point: &GeoPoint) -> KeyValuePair {
    KeyValuePair::new(
        String::from("GEO"),
        format!(":{};{}", geo_point.lat, geo_point.long),
    )
}

pub fn serialize_duration_to_ical(duration: &ParsedDuration) -> Option<KeyValuePair> {
    if let Some(duration_ical) = duration.to_ical() {
        Some(KeyValuePair::new(
            String::from("DURATION"),
            format!(":{}", duration_ical),
        ))
    } else {
        None
    }
}

pub fn serialize_dtstart_timestamp_to_ical(dtstart_timestamp: &i64, timezone: &Tz) -> KeyValuePair {
    KeyValuePair::new(
        String::from("DTSTART"),
        serialize_timestamp_to_ical_datetime(dtstart_timestamp, timezone),
    )
}

pub fn serialize_dtend_timestamp_to_ical(dtend_timestamp: &i64, timezone: &Tz) -> KeyValuePair {
    KeyValuePair::new(
        String::from("DTEND"),
        serialize_timestamp_to_ical_datetime(dtend_timestamp, timezone),
    )
}

pub fn serialize_uuid_to_ical(uuid: &String) -> KeyValuePair {
    KeyValuePair::new(String::from("UUID"), format!(":{}", uuid))
}

pub trait ICalSerializer {
    fn serialize_to_ical(&self, timezone: &Tz) -> Vec<String> {
        self.serialize_to_ical_set(timezone)
            .iter()
            .map(|key_value_pair| key_value_pair.to_string())
            .collect()
    }

    fn serialize_to_ical_set(&self, timezone: &Tz) -> BTreeSet<KeyValuePair>;
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_serialize_timestamp_to_ical_datetime() {
        let timestamp: i64 = 1699833600; // 2023-11-13 00:00:00 +0000

        assert_eq!(
            serialize_timestamp_to_ical_datetime(&timestamp, &Tz::UTC),
            String::from(":20231113T000000Z"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(&timestamp, &Tz::Europe__London),
            String::from(";TZID=Europe/London:20231113T000000"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(&timestamp, &Tz::Europe__Vilnius),
            String::from(";TZID=Europe/Vilnius:20231113T020000"),
        );

        assert_eq!(
            serialize_timestamp_to_ical_datetime(&timestamp, &Tz::Europe__Amsterdam),
            String::from(";TZID=Europe/Amsterdam:20231113T010000"),
        );
    }

    #[test]
    fn test_serialize_timestamp_to_utc_ical_datetime() {
        assert_eq!(
            serialize_timestamp_to_ical_utc_datetime(&1699833600), // 2023-11-13 00:00:00 +0000
            String::from("20231113T000000Z"),
        );
    }

    #[test]
    fn test_serialize_duration_to_ical() {
        assert_eq!(serialize_duration_to_ical(&ParsedDuration::default()), None,);

        assert_eq!(
            serialize_duration_to_ical(&ParsedDuration {
                weeks: None,
                days: Some(15),
                hours: Some(5),
                minutes: Some(0),
                seconds: Some(20),
            }),
            Some(KeyValuePair::new(
                String::from("DURATION"),
                String::from(":P15DT5H0M20S"),
            )),
        );

        assert_eq!(
            serialize_duration_to_ical(&ParsedDuration {
                weeks: Some(7),
                days: None,
                hours: None,
                minutes: None,
                seconds: None,
            }),
            Some(KeyValuePair::new(
                String::from("DURATION"),
                String::from(":P7W")
            )),
        );

        assert_eq!(
            serialize_duration_to_ical(&ParsedDuration {
                weeks: None,
                days: None,
                hours: None,
                minutes: None,
                seconds: Some(25),
            }),
            Some(KeyValuePair::new(
                String::from("DURATION"),
                String::from(":PT25S"),
            )),
        );
    }
}
