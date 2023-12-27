use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::core::EventInstance;

use super::results_ordering::{OrderingCondition, QueryResultOrdering};

#[derive(Debug)]
pub struct QueryResults {
    pub ordering_condition: OrderingCondition,
    pub results: BTreeSet<QueryResult>,
    pub count: usize,
    pub offset: usize,
}

impl QueryResults {
    pub fn new(ordering_condition: OrderingCondition, offset: usize) -> QueryResults {
        QueryResults {
            ordering_condition,
            offset,
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
        if self.count > self.offset {
            let result_ordering = self
                .ordering_condition
                .build_result_ordering_for_event_instance(&event_instance);

            let result = QueryResult {
                result_ordering,
                event_instance,
            };

            self.results.insert(result);
        }

        self.count += 1;
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
                .uuid
                .partial_cmp(&other.event_instance.uuid)
        } else {
            partial_ordering
        }
    }
}

impl Ord for QueryResult {
    fn cmp(&self, other: &Self) -> Ordering {
        let ordering = self.result_ordering.cmp(&other.result_ordering);

        if ordering.is_eq() {
            self.event_instance.uuid.cmp(&other.event_instance.uuid)
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

    use crate::core::{GeoDistance, GeoPoint};

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    use std::collections::BTreeSet;

    fn build_event_instance_one() -> EventInstance {
        EventInstance {
            uuid: String::from("UUID_ONE"),
            dtstart_timestamp: 100,
            dtend_timestamp: 110,
            duration: 10,
            geo: None,
            categories: None,
            related_to: None,
            passive_properties: BTreeSet::new(),
        }
    }

    fn build_event_instance_two() -> EventInstance {
        EventInstance {
            uuid: String::from("UUID_TWO"),
            dtstart_timestamp: 200,
            dtend_timestamp: 210,
            duration: 10,
            geo: Some(GeoPoint::new(-2.0760367, 51.899779)), // Cheltenham
            categories: None,
            related_to: None,
            passive_properties: BTreeSet::new(),
        }
    }

    fn build_event_instance_three() -> EventInstance {
        EventInstance {
            uuid: String::from("UUID_THREE"),
            dtstart_timestamp: 300,
            dtend_timestamp: 310,
            duration: 10,
            geo: Some(GeoPoint::new(-1.2475878, 51.7504163)), // Oxford
            categories: None,
            related_to: None,
            passive_properties: BTreeSet::new(),
        }
    }

    fn build_event_instance_four() -> EventInstance {
        EventInstance {
            uuid: String::from("UUID_FOUR"),
            dtstart_timestamp: 400,
            dtend_timestamp: 410,
            duration: 10,
            geo: Some(GeoPoint::new(-1.004574, 51.4517446)), // Reading
            categories: None,
            related_to: None,
            passive_properties: BTreeSet::new(),
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

        let mut query_results: QueryResults = QueryResults::new(OrderingCondition::DtStart, 0);

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

        let mut query_results: QueryResults = QueryResults::new(OrderingCondition::DtStart, 2);

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

        let mut query_results: QueryResults = QueryResults::new(OrderingCondition::DtStart, 4);

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
        let mut query_results: QueryResults = QueryResults::new(OrderingCondition::DtStart, 0);

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
        let mut query_results: QueryResults = QueryResults::new(OrderingCondition::DtStart, 0);

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
                GeoPoint::new(-0.0758252, 51.5055296), // London
            ),
            0,
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
                GeoPoint::new(-0.0758252, 51.5055296), // London
            ),
            0,
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
