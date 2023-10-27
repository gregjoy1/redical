use crate::data_types::{EventInstance, GeoDistance};
use std::cmp::Ordering;

pub trait QueryResultOrdering: PartialOrd + PartialEq + Eq + Ord {
    fn new_from_event_instance(event_instance: &EventInstance) -> Self;
}

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Clone)]
pub struct QueryResultDtstartOrdering(pub i64);

impl QueryResultOrdering for QueryResultDtstartOrdering {
    fn new_from_event_instance(event_instance: &EventInstance) -> Self {
        QueryResultDtstartOrdering(event_instance.dtstart_timestamp.clone())
    }
}

#[derive(Debug, PartialEq, Eq, Ord, Clone)]
pub struct QueryResultDtstartGeoDistOrdering(i64, Option<GeoDistance>);

impl PartialOrd for QueryResultDtstartGeoDistOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let dtstart_comparison = self.0.partial_cmp(&other.0);

        if dtstart_comparison.is_some_and(|ordering| ordering.is_eq()) {
            self.1.partial_cmp(&other.1)
        } else {
            dtstart_comparison
        }
    }
}

#[derive(Debug, PartialEq, Eq, Ord, Clone)]
pub struct QueryResultGeoDistDtstartOrdering(Option<GeoDistance>, i64);

impl PartialOrd for QueryResultGeoDistDtstartOrdering {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let distance_comparison = self.0.partial_cmp(&other.0);

        if distance_comparison.is_some_and(|ordering| ordering.is_eq()) {
            self.1.partial_cmp(&other.1)
        } else {
            distance_comparison
        }
    }
}
