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
    Operator(
        Box<WhereConditional>,
        Box<WhereConditional>,
        WhereOperator,
    ),
    Group(Box<WhereConditional>),
}

impl WhereConditional {
    pub fn execute<'cal>(&self, query_index_accessor: &impl QueryIndexAccessor<'cal>) -> Result<InvertedCalendarIndexTerm, String> {
        match self {
            WhereConditional::Property(where_conditional_property) => {
                let inverted_calendar_index_term = where_conditional_property.execute(query_index_accessor)?;

                Ok(inverted_calendar_index_term)
            }

            WhereConditional::Operator(
                where_conditional_a,
                where_conditional_b,
                where_operator,
            ) => {
                let inverted_calendar_index_term =
                    where_operator.execute(where_conditional_a, where_conditional_b, query_index_accessor)?;

                Ok(inverted_calendar_index_term)
            }

            WhereConditional::Group(where_conditional) => {
                let inverted_calendar_index_term = where_conditional.execute(query_index_accessor)?;

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
    pub fn get_details(&self) -> String {
        match &self {
            WhereConditionalProperty::Categories(category) => {
                format!("CATEGORIES:{category}")
            }

            WhereConditionalProperty::UID(uid) => {
                format!("UID:{uid}")
            }

            WhereConditionalProperty::LocationType(location_type) => {
                format!("LOCATION-TYPE:{location_type}")
            }

            WhereConditionalProperty::RelatedTo(reltype_uids) => {
                format!(
                    "RELATED-TO;RELTYPE={}:{}",
                    reltype_uids.key, reltype_uids.value
                )
            }

            WhereConditionalProperty::Geo(distance, long_lat) => {
                format!("GEO;DIST={}:{}", distance, long_lat)
            }

            WhereConditionalProperty::Class(classification) => {
                format!("CLASS:{}", classification)
            }
        }
    }

    pub fn execute<'cal>(&self, query_index_accessor: &impl QueryIndexAccessor<'cal>) -> Result<InvertedCalendarIndexTerm, String> {
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

    pub fn merge_and<'cal>(
        &self,
        inverted_index_term_a: &InvertedCalendarIndexTerm,
        query_index_accessor: &impl QueryIndexAccessor<'cal>,
    ) -> Result<InvertedCalendarIndexTerm, String> {
        let inverted_index_term_b = self.execute(query_index_accessor)?;

        Ok(InvertedCalendarIndexTerm::merge_and(
            inverted_index_term_a,
            &inverted_index_term_b,
        ))
    }

    pub fn merge_or<'cal>(
        &self,
        inverted_index_term_a: &InvertedCalendarIndexTerm,
        query_index_accessor: &impl QueryIndexAccessor<'cal>,
    ) -> Result<InvertedCalendarIndexTerm, String> {
        let inverted_index_term_b = self.execute(query_index_accessor)?;

        Ok(InvertedCalendarIndexTerm::merge_or(
            inverted_index_term_a,
            &inverted_index_term_b,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::assert_eq;

    use crate::{IndexedConclusion, Calendar};
    use crate::queries::event_instance_query::EventInstanceQueryIndexAccessor;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_query_where_conditional() {
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

        // TODO: Test GEO where params...

        // (
        //      ( CATEGORIES:CATEGORY_ONE OR RELATED-TO;RELTYPE=PARENT:PARENT_UID )
        //      AND
        //      ( CATEGORIES:CATEGORY_TWO OR RELATED-TO;RELTYPE=CHILD:CHILD_UID )
        // )
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

        let query_index_accessor = EventInstanceQueryIndexAccessor::new(&calendar);

        assert_eq!(
            query_where_conditional.execute(&query_index_accessor).unwrap(),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                ])
            }
        );
    }
}
