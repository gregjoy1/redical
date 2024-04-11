use std::cmp::Ordering;
use std::collections::BTreeSet;

use geo::HaversineDistance;

use crate::{EventInstance, GeoDistance, GeoPoint, KeyValuePair};

use redical_ical::{
    ICalendarComponent,
    RenderingContext,
    DistanceUnit,
    content_line::ContentLine,
    properties::{
        ICalendarProperty,
        DTStartProperty,
    },
};

use redical_ical::properties::ICalendarDateTimeProperty;

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
                QueryResultOrdering::DtStart(event_instance.dtstart.get_utc_timestamp().clone())
            }

            OrderingCondition::DtStartGeoDist(ordering_geo_point) => {
                let dtstart_timestamp = event_instance.dtstart.get_utc_timestamp().clone();
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
                let dtstart_timestamp = event_instance.dtstart.get_utc_timestamp().clone();
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

impl ICalendarComponent for QueryResultOrdering {
    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine> {
        let timezone =
            context.and_then(|context| context.tz)
                   .map_or(rrule::Tz::UTC, |tz| rrule::Tz::Tz(tz));

        let mut serialized_ical_set = BTreeSet::new();

        match self {
            QueryResultOrdering::DtStart(dtstart_timestamp) => {
                let dtstart_property = DTStartProperty::new_from_utc_timestamp(dtstart_timestamp);

                serialized_ical_set.insert(dtstart_property.to_content_line_with_context(context));
            }

            QueryResultOrdering::DtStartGeoDist(dtstart_timestamp, geo_distance) => {
                let dtstart_property = DTStartProperty::new_from_utc_timestamp(dtstart_timestamp);

                serialized_ical_set.insert(dtstart_property.to_content_line_with_context(context));

                if let Some(geo_distance) = geo_distance {
                    let geo_distance = match context
                        .cloned()
                        .and_then(|context| context.distance_unit)
                        .unwrap_or(DistanceUnit::Kilometers)
                    {
                        DistanceUnit::Kilometers => geo_distance.to_kilometers(),
                        DistanceUnit::Miles => geo_distance.to_miles(),
                    };

                    let x_geo_dist_property = ContentLine::from((String::from("X-GEO-DIST"), Vec::new(), geo_distance.to_string()));

                    serialized_ical_set.insert(x_geo_dist_property);
                }
            }

            QueryResultOrdering::GeoDistDtStart(geo_distance, dtstart_timestamp) => {
                if let Some(geo_distance) = geo_distance {
                    let geo_distance = match context
                        .cloned()
                        .and_then(|context| context.distance_unit)
                        .unwrap_or(DistanceUnit::Kilometers)
                    {
                        DistanceUnit::Kilometers => geo_distance.to_kilometers(),
                        DistanceUnit::Miles => geo_distance.to_miles(),
                    };

                    let x_geo_dist_property = ContentLine::from((String::from("X-GEO-DIST"), Vec::new(), geo_distance.to_string()));

                    serialized_ical_set.insert(x_geo_dist_property);
                }

                let dtstart_property = DTStartProperty::new_from_utc_timestamp(dtstart_timestamp);

                serialized_ical_set.insert(dtstart_property.to_content_line_with_context(context));
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
