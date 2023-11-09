use geo::HaversineDistance;
use crate::data_types::{EventInstance, GeoDistance, GeoPoint};
use std::cmp::Ordering;

#[derive(Debug, PartialEq, Clone)]
pub enum QueryOrderingCondition {
    DtStart,
    DtStartGeoDist(GeoPoint),
    GeoDistDtStart(GeoPoint),
}

impl QueryOrderingCondition {
    pub fn build_result_ordering_for_event_instance(&self, event_instance: &EventInstance) -> QueryResultOrdering {
        match &self {
            QueryOrderingCondition::DtStart => {
                QueryResultOrdering::DtStart(
                    event_instance.dtstart_timestamp.clone()
                )
            },

            QueryOrderingCondition::DtStartGeoDist(ordering_geo_point) => {
                let dtstart_timestamp = event_instance.dtstart_timestamp.clone();
                let geo_distance = event_instance.geo.clone().and_then(|event_instance_geo_point| {
                    Some(
                        GeoDistance::new_from_kilometers_float(
                            event_instance_geo_point.haversine_distance(&ordering_geo_point)
                        )
                    )
                });

                QueryResultOrdering::DtStartGeoDist(
                    dtstart_timestamp,
                    geo_distance
                )
            },

            QueryOrderingCondition::GeoDistDtStart(ordering_geo_point) => {
                let dtstart_timestamp = event_instance.dtstart_timestamp.clone();
                let geo_distance = event_instance.geo.clone().and_then(|event_instance_geo_point| {
                    Some(
                        GeoDistance::new_from_kilometers_float(
                            event_instance_geo_point.haversine_distance(&ordering_geo_point)
                        )
                    )
                });

                QueryResultOrdering::GeoDistDtStart(
                    geo_distance,
                    dtstart_timestamp
                )
            },
        }
    }
}

#[derive(Debug, PartialEq, Eq, Ord, Clone)]
pub enum QueryResultOrdering {
    DtStart(i64),
    DtStartGeoDist(i64, Option<GeoDistance>),
    GeoDistDtStart(Option<GeoDistance>, i64),
}

impl PartialOrd for QueryResultOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (&self, &other) {
            (
                QueryResultOrdering::DtStart(self_dtstart_timestamp),
                QueryResultOrdering::DtStart(other_dtstart_timestamp),
            ) => {
                self_dtstart_timestamp.partial_cmp(&other_dtstart_timestamp)
            },

            (
                QueryResultOrdering::DtStartGeoDist(self_dtstart_timestamp,  self_geo_distance),
                QueryResultOrdering::DtStartGeoDist(other_dtstart_timestamp, other_geo_distance),
            ) => {
                let dtstart_timestamp_comparison = self_dtstart_timestamp.partial_cmp(&other_dtstart_timestamp);

                if dtstart_timestamp_comparison.is_some_and(|ordering| ordering.is_eq()) {
                    self_geo_distance.partial_cmp(&other_geo_distance)
                } else {
                    dtstart_timestamp_comparison
                }
            },

            (
                QueryResultOrdering::GeoDistDtStart(self_geo_distance,  self_dtstart_timestamp),
                QueryResultOrdering::GeoDistDtStart(other_geo_distance, other_dtstart_timestamp),
            ) => {
                // Ensure that None is always Greater than Some(...)
                let geo_distance_comparison =
                    match (self_geo_distance, other_geo_distance) {
                        (
                            Some(self_geo_distance),
                            Some(other_geo_distance),
                        ) => {
                            self_geo_distance.partial_cmp(&other_geo_distance)
                        },

                        (Some(_), None) => {
                            Some(Ordering::Less)
                        },

                        (None, Some(_)) => {
                            Some(Ordering::Greater)
                        },

                        (None, None) => {
                            Some(Ordering::Equal)
                        },
                    };

                if geo_distance_comparison.is_some_and(|ordering| ordering.is_eq()) {
                    self_dtstart_timestamp.partial_cmp(&other_dtstart_timestamp)
                } else {
                    geo_distance_comparison
                }
            },

            _ => {
                panic!("Unexpected comparison between disparate QueryResultOrdering variants, self: {:#?} other: {:#?}", self, other);
            },
        }
    }
}
