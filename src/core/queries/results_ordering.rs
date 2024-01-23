use std::cmp::Ordering;
use std::collections::BTreeSet;

use chrono_tz::Tz;

use crate::core::ical::serializer::{
    DistanceUnit, SerializableICalComponent, SerializableICalProperty, SerializationPreferences,
};

use geo::HaversineDistance;

use crate::core::{EventInstance, GeoDistance, GeoPoint, KeyValuePair};

use crate::core::ical::properties::{DTStartProperty, XProperty};

#[derive(Debug, PartialEq, Clone)]
pub enum OrderingCondition {
    DtStart,
    DtStartGeoDist(GeoPoint),
    GeoDistDtStart(GeoPoint),
}

impl OrderingCondition {
    pub fn build_result_ordering_for_event_instance(
        &self,
        event_instance: &EventInstance,
    ) -> QueryResultOrdering {
        match &self {
            OrderingCondition::DtStart => {
                QueryResultOrdering::DtStart(event_instance.dtstart.utc_timestamp.clone())
            }

            OrderingCondition::DtStartGeoDist(ordering_geo_point) => {
                let dtstart_timestamp = event_instance.dtstart.utc_timestamp.clone();
                let geo_distance =
                    event_instance
                        .indexed_properties
                        .geo
                        .clone()
                        .and_then(|event_instance_geo| {
                            let event_instance_geo_point = GeoPoint::from(event_instance_geo);

                            Some(GeoDistance::new_from_meters_float(
                                event_instance_geo_point.haversine_distance(&ordering_geo_point),
                            ))
                        });

                QueryResultOrdering::DtStartGeoDist(dtstart_timestamp, geo_distance)
            }

            OrderingCondition::GeoDistDtStart(ordering_geo_point) => {
                let dtstart_timestamp = event_instance.dtstart.utc_timestamp.clone();
                let geo_distance =
                    event_instance
                        .indexed_properties
                        .geo
                        .clone()
                        .and_then(|event_instance_geo| {
                            let event_instance_geo_point = GeoPoint::from(event_instance_geo);

                            Some(GeoDistance::new_from_meters_float(
                                event_instance_geo_point.haversine_distance(&ordering_geo_point),
                            ))
                        });

                QueryResultOrdering::GeoDistDtStart(geo_distance, dtstart_timestamp)
            }
        }
    }
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Clone)]
pub enum QueryResultOrdering {
    DtStart(i64),
    DtStartGeoDist(i64, Option<GeoDistance>),
    GeoDistDtStart(Option<GeoDistance>, i64),
}

impl SerializableICalComponent for QueryResultOrdering {
    fn serialize_to_ical_set(
        &self,
        preferences: Option<&SerializationPreferences>,
    ) -> BTreeSet<String> {
        let timezone = if let Some(preferences) = preferences {
            rrule::Tz::Tz(preferences.get_timezone())
        } else {
            rrule::Tz::UTC
        };

        let mut serialized_ical_set = BTreeSet::new();

        match self {
            QueryResultOrdering::DtStart(dtstart_timestamp) => {
                let dtstart_property = DTStartProperty {
                    timezone: None,
                    value_type: None,
                    utc_timestamp: dtstart_timestamp.to_owned(),
                    x_params: None,
                };

                serialized_ical_set.insert(dtstart_property.serialize_to_ical(preferences));
            }

            QueryResultOrdering::DtStartGeoDist(dtstart_timestamp, geo_distance) => {
                let dtstart_property = DTStartProperty {
                    timezone: None,
                    value_type: None,
                    utc_timestamp: dtstart_timestamp.to_owned(),
                    x_params: None,
                };

                serialized_ical_set.insert(dtstart_property.serialize_to_ical(preferences));

                if let Some(geo_distance) = geo_distance {
                    let geo_distance = match preferences
                        .cloned()
                        .and_then(|preferences| preferences.distance_unit)
                        .unwrap_or(DistanceUnit::Kilometers)
                    {
                        DistanceUnit::Kilometers => geo_distance.to_kilometers(),
                        DistanceUnit::Miles => geo_distance.to_miles(),
                    };

                    let x_geo_dist_property = XProperty {
                        language: None,
                        name: String::from("X-GEO-DIST"),
                        value: geo_distance.to_string(),
                        x_params: None,
                    };

                    serialized_ical_set.insert(x_geo_dist_property.serialize_to_ical(preferences));
                }
            }

            QueryResultOrdering::GeoDistDtStart(geo_distance, dtstart_timestamp) => {
                if let Some(geo_distance) = geo_distance {
                    let geo_distance = match preferences
                        .cloned()
                        .and_then(|preferences| preferences.distance_unit)
                        .unwrap_or(DistanceUnit::Kilometers)
                    {
                        DistanceUnit::Kilometers => geo_distance.to_kilometers(),
                        DistanceUnit::Miles => geo_distance.to_miles(),
                    };

                    let x_geo_dist_property = XProperty {
                        language: None,
                        name: String::from("X-GEO-DIST"),
                        value: geo_distance.to_string(),
                        x_params: None,
                    };

                    serialized_ical_set.insert(x_geo_dist_property.serialize_to_ical(preferences));
                }

                let dtstart_property = DTStartProperty {
                    timezone: None,
                    value_type: None,
                    utc_timestamp: dtstart_timestamp.to_owned(),
                    x_params: None,
                };

                serialized_ical_set.insert(dtstart_property.serialize_to_ical(preferences));
            }
        }

        serialized_ical_set
    }
}

impl Ord for QueryResultOrdering {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self, &other) {
            (
                QueryResultOrdering::DtStart(self_dtstart_timestamp),
                QueryResultOrdering::DtStart(other_dtstart_timestamp),
            ) => self_dtstart_timestamp.cmp(&other_dtstart_timestamp),

            (
                QueryResultOrdering::DtStartGeoDist(self_dtstart_timestamp, self_geo_distance),
                QueryResultOrdering::DtStartGeoDist(other_dtstart_timestamp, other_geo_distance),
            ) => {
                let dtstart_timestamp_comparison =
                    self_dtstart_timestamp.cmp(&other_dtstart_timestamp);

                if dtstart_timestamp_comparison.is_eq() {
                    self_geo_distance.cmp(&other_geo_distance)
                } else {
                    dtstart_timestamp_comparison
                }
            }

            (
                QueryResultOrdering::GeoDistDtStart(self_geo_distance, self_dtstart_timestamp),
                QueryResultOrdering::GeoDistDtStart(other_geo_distance, other_dtstart_timestamp),
            ) => {
                // Ensure that None is always Greater than Some(...)
                let geo_distance_comparison = match (self_geo_distance, other_geo_distance) {
                    (Some(self_geo_distance), Some(other_geo_distance)) => {
                        self_geo_distance.cmp(&other_geo_distance)
                    }

                    (Some(_), None) => Ordering::Less,

                    (None, Some(_)) => Ordering::Greater,

                    (None, None) => Ordering::Equal,
                };

                if geo_distance_comparison.is_eq() {
                    self_dtstart_timestamp.cmp(&other_dtstart_timestamp)
                } else {
                    geo_distance_comparison
                }
            }

            _ => {
                panic!("Unexpected comparison between disparate QueryResultOrdering variants, self: {:#?} other: {:#?}", self, other);
            }
        }
    }
}
