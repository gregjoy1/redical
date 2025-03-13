use crate::{GeoDistance, GeoPoint, InvertedCalendarIndexTerm, KeyValuePair};

use redical_ical::values::where_operator as ical_where_operator;

use crate::queries::query::QueryIndexAccessor;

#[derive(Debug, PartialEq, Clone)]
pub enum WhereOperator {
    Or,
    And,
}

impl From<ical_where_operator::WhereOperator> for WhereOperator {
    fn from(where_operator: ical_where_operator::WhereOperator) -> Self {
        match where_operator {
            ical_where_operator::WhereOperator::Or => WhereOperator::Or,
            ical_where_operator::WhereOperator::And => WhereOperator::And,
        }
    }
}

impl WhereOperator {
    pub fn execute<'cal>(
        &self,
        where_conditional_a: &WhereConditional,
        where_conditional_b: &WhereConditional,
        query_index_accessor: &impl QueryIndexAccessor<'cal>,
    ) -> Result<InvertedCalendarIndexTerm, String> {
        let inverted_calendar_index_term_a = &where_conditional_a.execute(query_index_accessor)?;
        let inverted_calendar_index_term_b = &where_conditional_b.execute(query_index_accessor)?;

        let merged_inverted_calendar_index_term = match &self {
            WhereOperator::Or => InvertedCalendarIndexTerm::merge_or(
                inverted_calendar_index_term_a,
                inverted_calendar_index_term_b,
            ),

            WhereOperator::And => InvertedCalendarIndexTerm::merge_and(
                inverted_calendar_index_term_a,
                inverted_calendar_index_term_b,
            ),
        };

        Ok(merged_inverted_calendar_index_term)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereConditional {
    Property(WhereConditionalProperty),
    NegatedProperty(WhereConditionalProperty),
    Operator(
        Box<WhereConditional>,
        Box<WhereConditional>,
        WhereOperator,
    ),
    Group(Box<WhereConditional>),
}

impl WhereConditional {
    pub fn execute<'cal>(
        &self,
        query_index_accessor: &impl QueryIndexAccessor<'cal>
    ) -> Result<InvertedCalendarIndexTerm, String> {
        match self {
            WhereConditional::Property(where_conditional_property) => {
                let inverted_calendar_index_term = where_conditional_property.execute(
                    query_index_accessor
                )?;

                Ok(inverted_calendar_index_term)
            }

            WhereConditional::NegatedProperty(where_conditional_property) => {
                let inverted_calendar_index_term = where_conditional_property.execute_not(
                    query_index_accessor
                )?;

                Ok(inverted_calendar_index_term)
            }

            WhereConditional::Operator(
                where_conditional_a,
                where_conditional_b,
                where_operator,
            ) => {
                let inverted_calendar_index_term = where_operator.execute(
                    where_conditional_a,
                    where_conditional_b,
                    query_index_accessor
                )?;

                Ok(inverted_calendar_index_term)
            }

            WhereConditional::Group(where_conditional) => {
                let inverted_calendar_index_term = where_conditional.execute(
                    query_index_accessor
                )?;

                Ok(inverted_calendar_index_term)
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereConditionalProperty {
    UID(String),
    Categories(String),
    LocationType(String),
    RelatedTo(KeyValuePair),
    Geo(GeoDistance, GeoPoint),
    Class(String),
}

impl WhereConditionalProperty {
    pub fn execute<'cal>(
        &self,
        query_index_accessor: &impl QueryIndexAccessor<'cal>
    ) -> Result<InvertedCalendarIndexTerm, String> {
        match &self {
            // For UID, we just return an "include all" consensus for that event UID.
            WhereConditionalProperty::UID(uid) => {
                Ok(query_index_accessor.search_uid_index(uid))
            },

            WhereConditionalProperty::LocationType(location_type) => {
                Ok(query_index_accessor.search_location_type_index(location_type))
            },

            WhereConditionalProperty::Categories(category) => {
                Ok(query_index_accessor.search_categories_index(category))
            },

            WhereConditionalProperty::RelatedTo(reltype_uids) => {
                Ok(query_index_accessor.search_related_to_index(reltype_uids))
            },

            WhereConditionalProperty::Geo(distance, long_lat) => {
                Ok(query_index_accessor.search_geo_index(distance, long_lat))
            },

            WhereConditionalProperty::Class(classification) => {
                Ok(query_index_accessor.search_class_index(classification))
            },
        }
    }

    pub fn execute_not<'cal>(
        &self,
        query_index_accessor: &impl QueryIndexAccessor<'cal>
    ) -> Result<InvertedCalendarIndexTerm, String> {
        match &self {
            WhereConditionalProperty::UID(uid) => {
                Ok(query_index_accessor.search_not_uid_index(uid))
            },

            WhereConditionalProperty::LocationType(location_type) => {
                Ok(query_index_accessor.search_not_location_type_index(location_type))
            },

            WhereConditionalProperty::Categories(category) => {
                Ok(query_index_accessor.search_not_categories_index(category))
            },

            WhereConditionalProperty::RelatedTo(reltype_uids) => {
                Ok(query_index_accessor.search_not_related_to_index(reltype_uids))
            },

            WhereConditionalProperty::Geo(distance, long_lat) => {
                Ok(query_index_accessor.search_not_geo_index(distance, long_lat))
            },

            WhereConditionalProperty::Class(classification) => {
                Ok(query_index_accessor.search_not_class_index(classification))
            },
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::assert_eq;

    use crate::{IndexedConclusion, Calendar, Event};
    use crate::queries::event_query::EventQueryIndexAccessor;
    use crate::queries::event_instance_query::EventInstanceQueryIndexAccessor;
    use std::collections::{HashMap, HashSet};

    const LONDON: GeoPoint = GeoPoint { lat: 51.5074_f64, long: -0.1278_f64 };
    const OXFORD: GeoPoint = GeoPoint { lat: 51.8773_f64, long: -1.2475878_f64 };
    const NEW_YORK_CITY: GeoPoint = GeoPoint { lat: 40.7128_f64, long: -74.006_f64 };

    macro_rules! assert_event_query_results {
        ($calendar:expr, $conditional:expr, $expected:expr) => {
            let accessor = EventQueryIndexAccessor::new($calendar);
            let actual = $conditional.execute(&accessor).unwrap();

            assert_eq!(actual, $expected);
        }
    }

    macro_rules! assert_event_instance_query_results {
        ($calendar:expr, $conditional:expr, $expected:expr) => {
            let accessor = EventInstanceQueryIndexAccessor::new($calendar);
            let actual = $conditional.execute(&accessor).unwrap();

            assert_eq!(actual, $expected);
        }
    }

    fn calendar_with_events() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let event_one = Event::parse_ical("EVENT_ONE", "").unwrap();
        let event_two = Event::parse_ical("EVENT_TWO", "").unwrap();
        let event_three = Event::parse_ical("EVENT_THREE", "").unwrap();

        calendar.insert_event(event_one);
        calendar.insert_event(event_two);
        calendar.insert_event(event_three);
        calendar.rebuild_indexes().unwrap();

        calendar
    }

    fn calendar_with_indexed_location_types() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All in person"),
            String::from("All online"),
            String::from("Mostly in person"),
            String::from("Mostly online"),
            String::from("Unindexed event 1"),
            String::from("Unindexed event 2"),
        ];

        for event_uid in event_uids.iter() {
            calendar.insert_event(Event::new(event_uid.to_owned()));
        }

        let indexed_location_types = [
            (
                String::from("ONLINE"),
                [
                    (
                        String::from("All online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
            (
                String::from("IN-PERSON"),
                [
                    (
                        String::from("All in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            )
        ];

        for (location_type, events) in indexed_location_types.iter() {
            for (event_uid, conclusion) in events.iter() {
                calendar.indexed_location_type.insert(
                    event_uid.to_string(),
                    location_type.to_string(),
                    conclusion
                ).unwrap();
            }
        }

        calendar
    }

    fn calendar_with_indexed_categories() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All adults"),
            String::from("All kids"),
            String::from("Mostly adults"),
            String::from("Mostly kids"),
            String::from("Unindexed event 1"),
            String::from("Unindexed event 2"),
        ];

        for event_uid in event_uids.iter() {
            calendar.insert_event(Event::new(event_uid.to_owned()));
        }

        let indexed_categories = [
            (
                String::from("Adults"),
                [
                    (
                        String::from("All adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
            (
                String::from("Kids"),
                [
                    (
                        String::from("All kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
        ];

        for (category, events) in indexed_categories.iter() {
            for (event_uid, conclusion) in events.iter() {
                calendar.indexed_categories.insert(
                    event_uid.to_string(),
                    category.to_string(),
                    conclusion
                ).unwrap();
            }
        }

        calendar
    }

    fn calendar_with_indexed_relations() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All account-1"),
            String::from("All account-2"),
            String::from("Mostly account-1"),
            String::from("Mostly account-2"),
            String::from("Unindexed event 1"),
            String::from("Unindexed event 2"),
        ];

        for event_uid in event_uids.iter() {
            calendar.insert_event(Event::new(event_uid.to_owned()));
        }

        let indexed_related_to = [
            (
                KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-1"),
                ),
                [
                    (
                        String::from("All account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
            (
                KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-2"),
                ),
                [
                    (
                        String::from("All account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
        ];

        for (related_to, events) in indexed_related_to.iter() {
            for (event_uid, conclusion) in events.iter() {
                calendar.indexed_related_to.insert(
                    event_uid.to_string(),
                    related_to.clone(),
                    conclusion
                ).unwrap();
            }
        }

        calendar
    }

    fn calendar_with_indexed_geo_points() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All in London"),
            String::from("All in Oxford"),
            String::from("Mostly in London"),
            String::from("Mostly in Oxford"),
            String::from("Unindexed event 1"),
            String::from("Unindexed event 2"),
        ];

        for event_uid in event_uids.iter() {
            calendar.insert_event(Event::new(event_uid.to_owned()));
        }

        let indexed_geo = [
            (
                LONDON,
                [
                    (
                        String::from("All in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
            (
                OXFORD,
                [
                    (
                        String::from("All in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
        ];

        for (geo_point, events) in indexed_geo.iter() {
            for (event_uid, conclusion) in events.iter() {
                calendar.indexed_geo.insert(
                    event_uid.to_string(),
                    geo_point,
                    conclusion
                ).unwrap();
            }
        }

        calendar
    }

    fn calendar_with_indexed_classes() -> Calendar {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All public"),
            String::from("All private"),
            String::from("Mostly public"),
            String::from("Mostly private"),
            String::from("Unindexed event 1"),
            String::from("Unindexed event 2"),
        ];

        for event_uid in event_uids.iter() {
            calendar.insert_event(Event::new(event_uid.to_owned()));
        }

        let indexed_class = [
            (
                String::from("PUBLIC"),
                [
                    (
                        String::from("All public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
            (
                String::from("PRIVATE"),
                [
                    (
                        String::from("All private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]
            ),
        ];

        for (class, events) in indexed_class.iter() {
            for (event_uid, conclusion) in events.iter() {
                calendar.indexed_class.insert(
                    event_uid.to_string(),
                    class.to_string(),
                    conclusion
                ).unwrap();
            }
        }

        calendar
    }

    fn calendar_with_composite_indexes() -> Calendar {
        let mut calendar = Calendar::new(String::from("Calendar_UID"));

        calendar.indexed_categories.terms.insert(
            String::from("CATEGORY_ONE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("CATEGORY_ONE_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("CATEGORY_ONE_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("CATEGORY_ONE_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                ]),
            },
        );

        calendar.indexed_categories.terms.insert(
            String::from("CATEGORY_TWO"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("CATEGORY_TWO_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("CATEGORY_TWO_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("CATEGORY_TWO_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                ]),
            },
        );

        calendar.indexed_related_to.terms.insert(
            KeyValuePair::new(String::from("PARENT"), String::from("PARENT_UID")),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("RELATED_TO_PARENT_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("RELATED_TO_PARENT_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_PARENT_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                ]),
            },
        );

        calendar.indexed_related_to.terms.insert(
            KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UID")),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("RELATED_TO_CHILD_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("RELATED_TO_CHILD_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_CHILD_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                    ),
                ]),
            },
        );

        calendar
    }

    #[test]
    fn test_event_uid_querying_with_indexed_term() {
        let calendar = calendar_with_events();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::UID(
                    String::from("EVENT_ONE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_ONE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_uid_querying_with_unindexed_term() {
        let calendar = calendar_with_events();

        // TODO: fix this. Should return an empty event set
        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::UID(
                    String::from("EVENT_FOUR")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_FOUR"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_location_type_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::LocationType(
                    String::from("IN-PERSON")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_location_type_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::LocationType(
                    String::from("ON-HORSEBACK")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_categories_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Categories(
                    String::from("Kids")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_categories_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Categories(
                    String::from("Teenagers")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_related_to_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-1"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_related_to_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-4"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_geo_querying_with_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    OXFORD,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_geo_querying_without_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    NEW_YORK_CITY,
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_class_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Class(
                    String::from("PRIVATE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_class_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Class(
                    String::from("UNKNOWN")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_composite_conditional_event_querying() {
        let calendar = calendar_with_composite_indexes();

        // Where (CATEGORIES = CATEGORY_ONE OR PARENT_UID = PARENT) AND
        //       (CATEGORIES = CATEGORY_TWO OR CHILD = CHILD_UID)
        let query_where_conditional = WhereConditional::Group(
            Box::new(WhereConditional::Operator(
                Box::new(WhereConditional::Group(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UID"),
                            )),
                        )),
                        WhereOperator::Or,
                    )),
                )),
                Box::new(WhereConditional::Group(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_TWO")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("CHILD"),
                                String::from("CHILD_UID"),
                            )),
                        )),
                        WhereOperator::Or,
                    )),
                )),
                WhereOperator::And,
            )),
        );

        assert_event_query_results!(
            &calendar,
            query_where_conditional,
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_event_instance_uid_querying_with_indexed_term() {
        let calendar = calendar_with_events();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::UID(
                    String::from("EVENT_ONE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_ONE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_uid_querying_with_unindexed_term() {
        let calendar = calendar_with_events();

        // TODO: fix this. Should return an empty event set
        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::UID(
                    String::from("EVENT_FOUR")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_FOUR"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_location_type_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::LocationType(
                    String::from("IN-PERSON")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_location_type_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::LocationType(
                    String::from("ON-HORSEBACK")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_instance_categories_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Categories(
                    String::from("Kids")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_categories_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Categories(
                    String::from("Teenagers")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_instance_related_to_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-1"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_related_to_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-4"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_instance_geo_querying_with_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    OXFORD,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_geo_querying_without_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    NEW_YORK_CITY,
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_event_instance_class_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Class(
                    String::from("PRIVATE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_class_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::Property(
                WhereConditionalProperty::Class(
                    String::from("UNKNOWN")
                )
            ),
            InvertedCalendarIndexTerm {
                events: [].into(),
            }
        );
    }

    #[test]
    fn test_negated_event_uid_querying_with_indexed_term() {
        let calendar = calendar_with_events();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::UID(
                    String::from("EVENT_ONE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_TWO"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_THREE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_uid_querying_with_unindexed_term() {
        let calendar = calendar_with_events();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::UID(
                    String::from("EVENT_FOUR")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_ONE"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_TWO"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_THREE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_location_type_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::LocationType(
                    String::from("IN-PERSON")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_location_type_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::LocationType(
                    String::from("ON-HORSEBACK")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All online"), 
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_negated_event_categories_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Categories(
                    String::from("Kids")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_categories_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Categories(
                    String::from("Teenagers")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All adults"), 
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_negated_event_related_to_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-1"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_related_to_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-4"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-1"), 
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_negated_event_geo_querying_with_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    OXFORD,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_geo_querying_without_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    NEW_YORK_CITY,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in Oxford"), 
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_negated_event_class_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Class(
                    String::from("PRIVATE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_class_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Class(
                    String::from("UNKNOWN")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_uid_querying_with_indexed_term() {
        let calendar = calendar_with_events();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::UID(
                    String::from("EVENT_ONE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_TWO"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_THREE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_uid_querying_with_unindexed_term() {
        let calendar = calendar_with_events();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::UID(
                    String::from("EVENT_FOUR")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("EVENT_ONE"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_TWO"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("EVENT_THREE"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_location_type_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::LocationType(
                    String::from("IN-PERSON")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_location_type_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_location_types();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::LocationType(
                    String::from("ON-HORSEBACK")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_categories_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Categories(
                    String::from("Kids")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_categories_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_categories();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Categories(
                    String::from("Teenagers")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly adults"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly kids"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_related_to_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-1"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_related_to_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_relations();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::RelatedTo(
                    KeyValuePair::new(
                        String::from("X-ACCOUNT"),
                        String::from("account-4"),
                    )
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly account-2"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_geo_querying_with_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    OXFORD,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_geo_querying_without_events_in_radius() {
        let calendar = calendar_with_indexed_geo_points();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Geo(
                    GeoDistance::new_from_miles_float(10.0_f64),
                    NEW_YORK_CITY,
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in London"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly in Oxford"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_class_querying_with_indexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Class(
                    String::from("PRIVATE")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Include(Some([100].into()))
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Exclude(Some([100].into()))
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_negated_event_instance_class_querying_with_unindexed_term() {
        let calendar = calendar_with_indexed_classes();

        assert_event_instance_query_results!(
            &calendar,
            WhereConditional::NegatedProperty(
                WhereConditionalProperty::Class(
                    String::from("UNKNOWN")
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("All public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly public"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("All private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Mostly private"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 1"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("Unindexed event 2"),
                        IndexedConclusion::Include(None)
                    ),
                ]),
            }
        );
    }

    #[test]
    fn test_composite_conditional_event_instance_querying() {
        let calendar = calendar_with_composite_indexes();

        // Where (CATEGORIES = CATEGORY_ONE OR PARENT_UID = PARENT) AND
        //       (CATEGORIES = CATEGORY_TWO OR CHILD = CHILD_UID)
        let query_where_conditional = WhereConditional::Group(
            Box::new(WhereConditional::Operator(
                Box::new(WhereConditional::Group(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UID"),
                            )),
                        )),
                        WhereOperator::Or,
                    )),
                )),
                Box::new(WhereConditional::Group(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_TWO")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("CHILD"),
                                String::from("CHILD_UID"),
                            )),
                        )),
                        WhereOperator::Or,
                    )),
                )),
                WhereOperator::And,
            )),
        );

        assert_event_instance_query_results!(
            &calendar,
            query_where_conditional,
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some([100, 200].into()))
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some([100, 200].into()))
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some([100, 200].into()))
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some([100, 200].into()))
                    ),
                ])
            }
        );
    }
}
