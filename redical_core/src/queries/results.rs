use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

use crate::EventInstance;

use super::results_ordering::{OrderingCondition, QueryResultOrdering};

#[derive(Debug)]
pub struct QueryResults {
    pub ordering_condition: OrderingCondition,
    pub results: BTreeSet<QueryResult>,
    pub distinct_uid_lookup: Option<HashSet<String>>,
    pub count: usize,
    pub offset: usize,
}

impl QueryResults {
    pub fn new(
        ordering_condition: OrderingCondition,
        offset: usize,
        distinct_uids: bool,
    ) -> QueryResults {
        let distinct_uid_lookup = if distinct_uids {
            Some(HashSet::new())
        } else {
            None
        };

        QueryResults {
            ordering_condition,
            offset,
            distinct_uid_lookup,
            count: 1,
            results: BTreeSet::new(),
        }
    }

    pub fn truncate(&mut self, length: usize) {
        while self.len() > length {
            self.results.pop_last();
        }
    }

    pub fn len(&self) -> usize {
        self.results.len()
    }

    pub fn push(&mut self, event_instance: EventInstance) {
        let event_instance_uid = event_instance.uid.uid.clone();

        if self.is_event_instance_included(&event_instance) {
            let result_ordering = self
                .ordering_condition
                .build_result_ordering_for_event_instance(&event_instance);

            let result = QueryResult {
                result_ordering,
                event_instance,
            };

            self.results.insert(result);
        }

        // If only distinct UIDs are to be returned, we add the UID of the current
        // EventInstance to the lookup set so that any future EventInstances sharing the same
        // UID are excluded.
        if let Some(distinct_uid_lookup) = &mut self.distinct_uid_lookup {
            distinct_uid_lookup.insert(event_instance_uid.to_string());
        }

        self.count += 1;
    }

    fn is_event_instance_included(&self, event_instance: &EventInstance) -> bool {
        // This ensures that only EventInstances within the offset window are included. Those
        // before the window are counted by excluded.
        if self.count <= self.offset {
            return false;
        }

        // If only distinct UIDs are to be returned, we check the lookup set for the presence of
        // the UID of the proposed EventInstance, and if its present, we exclude it.
        if self
            .distinct_uid_lookup
            .as_ref()
            .is_some_and(|distinct_uid_lookup| {
                distinct_uid_lookup.contains(&event_instance.uid.uid.to_string())
            })
        {
            return false;
        }

        true
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueryResult {
    pub result_ordering: QueryResultOrdering,
    pub event_instance: EventInstance,
}

impl PartialOrd for QueryResult {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let partial_ordering = self.result_ordering.partial_cmp(&other.result_ordering);

        if partial_ordering.is_some_and(|partial_ordering| partial_ordering.is_eq()) {
            self.event_instance
                .uid
                .partial_cmp(&other.event_instance.uid)
        } else {
            partial_ordering
        }
    }
}

impl Ord for QueryResult {
    fn cmp(&self, other: &Self) -> Ordering {
        let ordering = self.result_ordering.cmp(&other.result_ordering);

        if ordering.is_eq() {
            self.event_instance.uid.cmp(&other.event_instance.uid)
        } else {
            ordering
        }
    }
}

pub trait QueryResultItem: Ord + Eq + PartialEq {}

impl QueryResultItem for QueryResult {}

#[cfg(test)]
mod test {
    use super::*;

    use crate::{GeoDistance, GeoPoint, IndexedProperties, PassiveProperties};

    use pretty_assertions_sorted::assert_eq;

    use crate::testing::macros::build_property_from_ical;

    use redical_ical::properties::{
        DTEndProperty,
        DTStartProperty,
        DurationProperty,
        GeoProperty,
        UIDProperty,
        PassiveProperty,
    };

    use std::str::FromStr;

    fn build_event_instance_one() -> EventInstance {
        EventInstance {
            uid: build_property_from_ical!(UIDProperty, "UID:UID_ONE"),
            dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000140Z"),
            dtend: build_property_from_ical!(DTEndProperty, "DTEND:19700101T000150Z"),
            duration: build_property_from_ical!(DurationProperty, "DURATION:PT10S"),
            indexed_properties: IndexedProperties::new(),
            passive_properties: PassiveProperties::new(),
        }
    }

    fn build_event_instance_two() -> EventInstance {
        EventInstance {
            uid: build_property_from_ical!(UIDProperty, "UID:UID_TWO"),
            dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000320Z"),
            dtend: build_property_from_ical!(DTEndProperty, "DTEND:19700101T000330Z"),
            duration: build_property_from_ical!(DurationProperty, "DURATION:PT10S"),
            indexed_properties: IndexedProperties {
                class: None,
                geo: Some(build_property_from_ical!(
                    GeoProperty,
                    "GEO:51.899779;-2.0760367"
                )), // Cheltenham
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
        }
    }

    fn build_event_instance_three() -> EventInstance {
        EventInstance {
            uid: build_property_from_ical!(UIDProperty, "UID:UID_THREE"),
            dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000500Z"),
            dtend: build_property_from_ical!(DTEndProperty, "DTEND:19700101T000510Z"),
            duration: build_property_from_ical!(DurationProperty, "DURATION:PT10S"),
            indexed_properties: IndexedProperties {
                class: None,
                geo: Some(build_property_from_ical!(
                    GeoProperty,
                    "GEO:51.7504163;-1.2475878"
                )), // Oxford
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
        }
    }

    fn build_event_instance_four() -> EventInstance {
        EventInstance {
            uid: build_property_from_ical!(UIDProperty, "UID:UID_FOUR"),
            dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000640Z"),
            dtend: build_property_from_ical!(DTEndProperty, "DTEND:19700101T000650Z"),
            duration: build_property_from_ical!(DurationProperty, "DURATION:PT10S"),
            indexed_properties: IndexedProperties {
                class: None,
                geo: Some(build_property_from_ical!(
                    GeoProperty,
                    "GEO:51.4517446;-1.004574"
                )), // Reading
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
        }
    }

    #[test]
    fn test_query_results_offset() {
        let all_expected_query_results = vec![
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(100),
                event_instance: build_event_instance_one(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(200),
                event_instance: build_event_instance_two(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(300),
                event_instance: build_event_instance_three(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(400),
                event_instance: build_event_instance_four(),
            },
        ];

        let mut query_results: QueryResults =
            QueryResults::new(OrderingCondition::DtStart, 0, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_one());
        query_results.push(build_event_instance_two());
        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    event_instance: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(200),
                    event_instance: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    event_instance: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    event_instance: build_event_instance_four(),
                },
            ],
        );

        let mut query_results: QueryResults =
            QueryResults::new(OrderingCondition::DtStart, 2, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_one());
        query_results.push(build_event_instance_two());
        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    event_instance: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    event_instance: build_event_instance_four(),
                },
            ],
        );

        let mut query_results: QueryResults =
            QueryResults::new(OrderingCondition::DtStart, 4, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_one());
        query_results.push(build_event_instance_two());
        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![],
        );
    }

    #[test]
    fn test_query_results_truncate() {
        let mut query_results: QueryResults =
            QueryResults::new(OrderingCondition::DtStart, 0, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_one());
        query_results.push(build_event_instance_two());
        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_four());

        let all_expected_query_results = vec![
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(100),
                event_instance: build_event_instance_one(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(200),
                event_instance: build_event_instance_two(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(300),
                event_instance: build_event_instance_three(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(400),
                event_instance: build_event_instance_four(),
            },
        ];

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            all_expected_query_results,
        );

        query_results.truncate(6);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            all_expected_query_results,
        );

        query_results.truncate(4);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            all_expected_query_results,
        );

        query_results.truncate(3);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            all_expected_query_results[0..=2],
        );

        query_results.truncate(1);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            all_expected_query_results[0..=0],
        );

        query_results.truncate(0);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![],
        );
    }

    #[test]
    fn test_query_results_dtstart_ordering() {
        let mut query_results: QueryResults =
            QueryResults::new(OrderingCondition::DtStart, 0, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::DtStart(400),
                event_instance: build_event_instance_four(),
            },]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    event_instance: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    event_instance: build_event_instance_four(),
                },
            ]
        );

        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_two());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    event_instance: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(200),
                    event_instance: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    event_instance: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    event_instance: build_event_instance_four(),
                },
            ]
        );
    }

    #[test]
    fn test_query_results_dtstart_geo_dist_ordering() {
        let mut query_results: QueryResults = QueryResults::new(
            OrderingCondition::DtStartGeoDist(
                GeoPoint::new(51.5055296_f64, -0.0758252_f64), // London
            ),
            0,
            false,
        );

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::DtStartGeoDist(
                    400,
                    Some(GeoDistance::Kilometers((64, 595658)))
                ),
                event_instance: build_event_instance_four(),
            },]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(100, None),
                    event_instance: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        400,
                        Some(GeoDistance::Kilometers((64, 595658)))
                    ),
                    event_instance: build_event_instance_four(),
                },
            ]
        );

        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_two());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(100, None),
                    event_instance: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        200,
                        Some(GeoDistance::Kilometers((144, 636981)))
                    ),
                    event_instance: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        300,
                        Some(GeoDistance::Kilometers((85, 341678)))
                    ),
                    event_instance: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        400,
                        Some(GeoDistance::Kilometers((64, 595658)))
                    ),
                    event_instance: build_event_instance_four(),
                },
            ]
        );
    }

    #[test]
    fn test_query_results_geo_dist_dtstart_ordering() {
        let mut query_results: QueryResults = QueryResults::new(
            OrderingCondition::GeoDistDtStart(
                GeoPoint::new(51.5055296_f64, -0.0758252_f64), // London
            ),
            0,
            false,
        );

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::GeoDistDtStart(
                    Some(GeoDistance::Kilometers((64, 595658))),
                    400
                ),
                event_instance: build_event_instance_four(),
            },]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((64, 595658))),
                        400
                    ),
                    event_instance: build_event_instance_four(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(None, 100),
                    event_instance: build_event_instance_one(),
                },
            ]
        );

        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_two());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((64, 595658))),
                        400
                    ),
                    event_instance: build_event_instance_four(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((85, 341678))),
                        300
                    ),
                    event_instance: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((144, 636981))),
                        200
                    ),
                    event_instance: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(None, 100),
                    event_instance: build_event_instance_one(),
                },
            ]
        );
    }
}
