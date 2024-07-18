use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};

use redical_ical::ICalendarComponent;

use super::results_ordering::{OrderingCondition, QueryResultOrdering};

pub trait QueryableEntity: ICalendarComponent + Eq {
    fn build_result_ordering(&self, ordering_condition: &OrderingCondition) -> QueryResultOrdering;

    fn get_uid(&self) -> String;
}

#[derive(Debug)]
pub struct QueryResults<T: QueryableEntity> {
    pub ordering_condition: OrderingCondition,
    pub results: BTreeSet<QueryResult<T>>,
    pub distinct_uid_lookup: Option<HashSet<String>>,
    pub count: usize,
    pub offset: usize,
}

impl<T> QueryResults<T>
where
    T: QueryableEntity,
{
    pub fn new(
        ordering_condition: OrderingCondition,
        offset: usize,
        distinct_uids: bool,
    ) -> QueryResults<T> {
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

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn push(&mut self, entity: T) {
        let uid = entity.get_uid();

        if self.is_queryable_entity_included(&entity) {
            let result_ordering = entity.build_result_ordering(&self.ordering_condition);

            let result = QueryResult {
                result_ordering,
                result: entity,
            };

            self.results.insert(result);
        }

        // If only distinct UIDs are to be returned, we add the UID of the current
        // EventInstance to the lookup set so that any future EventInstances sharing the same
        // UID are excluded.
        if let Some(distinct_uid_lookup) = &mut self.distinct_uid_lookup {
            distinct_uid_lookup.insert(uid.to_string());
        }

        self.count += 1;
    }

    fn is_queryable_entity_included(&self, entity: &T) -> bool {
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
                distinct_uid_lookup.contains(&entity.get_uid())
            })
        {
            return false;
        }

        true
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueryResult<T: ICalendarComponent> {
    pub result_ordering: QueryResultOrdering,
    pub result: T,
}

impl<T> PartialOrd for QueryResult<T>
where
    T: QueryableEntity,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for QueryResult<T>
where
    T: QueryableEntity,
{
    fn cmp(&self, other: &Self) -> Ordering {
        let ordering = self.result_ordering.cmp(&other.result_ordering);

        if ordering.is_eq() {
            self.result.get_uid().cmp(&other.result.get_uid())
        } else {
            ordering
        }
    }
}

pub trait QueryResultItem: Ord + Eq + PartialEq {}

impl<T> QueryResultItem for QueryResult<T> where T: QueryableEntity {}

#[cfg(test)]
mod test {
    use super::*;

    use crate::{EventInstance, GeoDistance, GeoPoint, IndexedProperties, PassiveProperties};

    use pretty_assertions_sorted::assert_eq;

    use crate::testing::macros::build_property_from_ical;

    use redical_ical::properties::{
        DTEndProperty,
        DTStartProperty,
        DurationProperty,
        GeoProperty,
        UIDProperty,
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
                location_type: None,
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
                location_type: None,
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
                location_type: None,
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
        }
    }

    #[test]
    fn test_query_results_offset() {
        let mut query_results: QueryResults<EventInstance> =
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    result: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(200),
                    result: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    result: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    result: build_event_instance_four(),
                },
            ],
        );

        let mut query_results: QueryResults<EventInstance> =
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    result: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    result: build_event_instance_four(),
                },
            ],
        );

        let mut query_results: QueryResults<EventInstance> =
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![],
        );
    }

    #[test]
    fn test_query_results_truncate() {
        let mut query_results: QueryResults<EventInstance> =
            QueryResults::new(OrderingCondition::DtStart, 0, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_one());
        query_results.push(build_event_instance_two());
        query_results.push(build_event_instance_three());
        query_results.push(build_event_instance_four());

        let all_expected_query_results = vec![
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(100),
                result: build_event_instance_one(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(200),
                result: build_event_instance_two(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(300),
                result: build_event_instance_three(),
            },
            QueryResult {
                result_ordering: QueryResultOrdering::DtStart(400),
                result: build_event_instance_four(),
            },
        ];

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            all_expected_query_results,
        );

        query_results.truncate(6);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            all_expected_query_results,
        );

        query_results.truncate(4);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            all_expected_query_results,
        );

        query_results.truncate(3);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            all_expected_query_results[0..=2],
        );

        query_results.truncate(1);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            all_expected_query_results[0..=0],
        );

        query_results.truncate(0);

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![],
        );
    }

    #[test]
    fn test_query_results_dtstart_ordering() {
        let mut query_results: QueryResults<EventInstance> =
            QueryResults::new(OrderingCondition::DtStart, 0, false);

        assert!(query_results.results.is_empty());

        query_results.push(build_event_instance_four());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::DtStart(400),
                result: build_event_instance_four(),
            }]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    result: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    result: build_event_instance_four(),
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(100),
                    result: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(200),
                    result: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(300),
                    result: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStart(400),
                    result: build_event_instance_four(),
                },
            ]
        );
    }

    #[test]
    fn test_query_results_dtstart_geo_dist_ordering() {
        let mut query_results: QueryResults<EventInstance> = QueryResults::new(
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::DtStartGeoDist(
                    400,
                    Some(GeoDistance::Kilometers((64, 595658)))
                ),
                result: build_event_instance_four(),
            },]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(100, None),
                    result: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        400,
                        Some(GeoDistance::Kilometers((64, 595658)))
                    ),
                    result: build_event_instance_four(),
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(100, None),
                    result: build_event_instance_one(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        200,
                        Some(GeoDistance::Kilometers((144, 636981)))
                    ),
                    result: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        300,
                        Some(GeoDistance::Kilometers((85, 341678)))
                    ),
                    result: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::DtStartGeoDist(
                        400,
                        Some(GeoDistance::Kilometers((64, 595658)))
                    ),
                    result: build_event_instance_four(),
                },
            ]
        );
    }

    #[test]
    fn test_query_results_geo_dist_dtstart_ordering() {
        let mut query_results: QueryResults<EventInstance> = QueryResults::new(
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![QueryResult {
                result_ordering: QueryResultOrdering::GeoDistDtStart(
                    Some(GeoDistance::Kilometers((64, 595658))),
                    400
                ),
                result: build_event_instance_four(),
            }]
        );

        query_results.push(build_event_instance_one());

        assert_eq!(
            query_results
                .results
                .clone()
                .into_iter()
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((64, 595658))),
                        400
                    ),
                    result: build_event_instance_four(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(None, 100),
                    result: build_event_instance_one(),
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
                .collect::<Vec<QueryResult<EventInstance>>>(),
            vec![
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((64, 595658))),
                        400
                    ),
                    result: build_event_instance_four(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((85, 341678))),
                        300
                    ),
                    result: build_event_instance_three(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(
                        Some(GeoDistance::Kilometers((144, 636981))),
                        200
                    ),
                    result: build_event_instance_two(),
                },
                QueryResult {
                    result_ordering: QueryResultOrdering::GeoDistDtStart(None, 100),
                    result: build_event_instance_one(),
                },
            ]
        );
    }
}
