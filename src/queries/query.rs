use std::time::Duration;

use crate::data_types::{KeyValuePair, InvertedCalendarIndexTerm, Calendar};

use crate::queries::results::QueryResults;
use crate::queries::ordering::OrderingCondition;

#[derive(Debug, PartialEq, Clone)]
pub struct Query {
    where_conditional:  Option<WhereConditional>,
    ordering_condition: OrderingCondition,
    limit:              i64,
}

impl Query {
    pub fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults, String> {
        let where_conditional_result = 
            if let Some(where_conditional) = &mut self.where_conditional {
                Some(where_conditional.execute(calendar)?)
            } else {
                None
            };

        let mut query_results = QueryResults::new(self.ordering_condition.clone());

        // TODO: implement EventInstance extrapolation...

        Ok(query_results)
    }
}

impl Default for Query {

    fn default() -> Self {
        Query {
            where_conditional:  None,
            ordering_condition: OrderingCondition::DtStart,
            limit:              50,
        }
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereOperator {
    Or,
    And,
}

impl WhereOperator {

    pub fn execute(&self, where_conditional_a: &mut WhereConditional, where_conditional_b: &mut WhereConditional, calendar: &Calendar) -> Result<InvertedCalendarIndexTerm, String> {
        let inverted_calendar_index_term_a = &where_conditional_a.execute(calendar)?;
        let inverted_calendar_index_term_b = &where_conditional_b.execute(calendar)?;

        let merged_inverted_calendar_index_term = match &self {
            WhereOperator::Or => {
                InvertedCalendarIndexTerm::merge_or(
                    inverted_calendar_index_term_a,
                    inverted_calendar_index_term_b,
                )
            },

            WhereOperator::And => {
                InvertedCalendarIndexTerm::merge_and(
                    inverted_calendar_index_term_a,
                    inverted_calendar_index_term_b,
                )
            },
        };

        Ok(merged_inverted_calendar_index_term)
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereConditional {
    Property(WhereConditionalProperty, Option<WhereConditionalAnalysis>),
    Operator(Box<WhereConditional>, Box<WhereConditional>, WhereOperator, Option<WhereConditionalAnalysis>),
    Group(Box<WhereConditional>, Option<WhereConditionalAnalysis>),
}

impl WhereConditional {

    pub fn execute(&mut self, calendar: &Calendar) -> Result<InvertedCalendarIndexTerm, String> {
        let start = std::time::Instant::now();

        match self {
            WhereConditional::Property(
                where_conditional_property,
                where_conditional_analysis
            ) => {
                let inverted_calendar_index_term = where_conditional_property.execute(calendar)?;

                let _ = where_conditional_analysis.insert(
                    WhereConditionalAnalysis {
                        elapsed_duration: start.elapsed(),
                        output_count:     inverted_calendar_index_term.events.len(),
                    }
                );

                Ok(inverted_calendar_index_term)
            },

            WhereConditional::Operator(
                where_conditional_a,
                where_conditional_b,
                where_operator,
                where_conditional_analysis
            ) => {
                let inverted_calendar_index_term = where_operator.execute(
                    where_conditional_a,
                    where_conditional_b,
                    calendar
                )?;

                let _ = where_conditional_analysis.insert(
                    WhereConditionalAnalysis {
                        elapsed_duration: start.elapsed(),
                        output_count:     inverted_calendar_index_term.events.len(),
                    }
                );

                Ok(inverted_calendar_index_term)
            },

            WhereConditional::Group(
                where_conditional,
                where_conditional_analysis
            ) => {
                let inverted_calendar_index_term = where_conditional.execute(calendar)?;

                let _ = where_conditional_analysis.insert(
                    WhereConditionalAnalysis {
                        elapsed_duration: start.elapsed(),
                        output_count:     inverted_calendar_index_term.events.len(),
                    }
                );

                Ok(inverted_calendar_index_term)
            }
        }
    }

    pub fn get_where_conditional_analyses(&self, depth: i32) -> Result<Vec<(i32, String, WhereConditionalAnalysis)>, String> {
        match self {
            WhereConditional::Property(
                where_conditional_property,
                where_conditional_analysis
            ) => {
                let details = format!("Property: {:#?}", where_conditional_property.get_details());

                if let Some(where_conditional_analysis) = where_conditional_analysis {
                    Ok(vec![(depth, details, where_conditional_analysis.clone())])
                } else {
                    Err(format!("None WhereConditionalAnalysis at {details}"))
                }
            },

            WhereConditional::Operator(
                where_conditional_a,
                where_conditional_b,
                where_operator,
                where_conditional_analysis
            ) => {
                let details = format!("Operator: {:#?}", where_operator);

                if let Some(where_conditional_analysis) = where_conditional_analysis {
                    Ok(
                        vec![
                            vec![(depth, details, where_conditional_analysis.clone())],
                            where_conditional_a.get_where_conditional_analyses(depth + 1)?,
                            where_conditional_b.get_where_conditional_analyses(depth + 1)?,
                        ].concat()
                    )
                } else {
                    Err(format!("None WhereConditionalAnalysis at {details}"))
                }
            },

            WhereConditional::Group(
                where_conditional,
                where_conditional_analysis
            ) => {
                if let Some(where_conditional_analysis) = where_conditional_analysis {
                    Ok(
                        vec![
                            vec![(depth, String::from("Group"), where_conditional_analysis.clone())],
                            where_conditional.get_where_conditional_analyses(depth + 1)?,
                        ].concat()
                    )
                } else {
                    Err(format!("None WhereConditionalAnalysis at Group"))
                }
            }
        }
    }

}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereConditionalProperty {
    Categories(String),
    RelatedTo(KeyValuePair),
}

impl WhereConditionalProperty {

    pub fn get_details(&self) -> String {
        match &self {
            WhereConditionalProperty::Categories(category) => {
                format!("CATEGORIES:{category}")
            },

            WhereConditionalProperty::RelatedTo(reltype_uuids) => {
                format!("RELATED-TO;RELTYPE={}:{}", reltype_uuids.key, reltype_uuids.value)
            },
        }
    }

    pub fn execute(&self, calendar: &Calendar) -> Result<InvertedCalendarIndexTerm, String> {
        match &self {
            WhereConditionalProperty::Categories(category) => {
                Ok(
                    calendar.indexed_categories
                            .terms
                            .get(category)
                            .unwrap_or(
                                &InvertedCalendarIndexTerm::new()
                            )
                            .clone()
                )
            },

            WhereConditionalProperty::RelatedTo(reltype_uuids) => {
                Ok(
                    calendar.indexed_related_to
                            .terms
                            .get(reltype_uuids)
                            .unwrap_or(
                                &InvertedCalendarIndexTerm::new()
                            )
                            .clone()
                )
            },
        }
    }

    pub fn merge_and(&self, inverted_index_term_a: &InvertedCalendarIndexTerm, calendar: &Calendar) -> Result<InvertedCalendarIndexTerm, String> {
        let empty_calendar_index_term = InvertedCalendarIndexTerm::new();

        let inverted_index_term_b = match &self {
            WhereConditionalProperty::Categories(category) => {
                calendar.indexed_categories
                        .terms
                        .get(category)
                        .unwrap_or(&empty_calendar_index_term)
            },

            WhereConditionalProperty::RelatedTo(reltype_uuids) => {
                calendar.indexed_related_to
                        .terms
                        .get(reltype_uuids)
                        .unwrap_or(&empty_calendar_index_term)
            },
        };

        Ok(
            InvertedCalendarIndexTerm::merge_and(
                inverted_index_term_a,
                inverted_index_term_b
            )
        )
    }

    pub fn merge_or(&self, inverted_index_term_a: &InvertedCalendarIndexTerm, calendar: &Calendar) -> Result<InvertedCalendarIndexTerm, String> {
        let empty_calendar_index_term = InvertedCalendarIndexTerm::new();

        let inverted_index_term_b = match &self {
            WhereConditionalProperty::Categories(category) => {
                calendar.indexed_categories
                        .terms
                        .get(category)
                        .unwrap_or(&empty_calendar_index_term)
            },

            WhereConditionalProperty::RelatedTo(reltype_uuids) => {
                calendar.indexed_related_to
                        .terms
                        .get(reltype_uuids)
                        .unwrap_or(&empty_calendar_index_term)
            },
        };

        Ok(
            InvertedCalendarIndexTerm::merge_or(
                inverted_index_term_a,
                inverted_index_term_b
            )
        )
    }

}

#[derive(Debug, PartialEq, Clone)]
pub struct WhereConditionalAnalysis {
    elapsed_duration: Duration,
    output_count:     usize,
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    use std::collections::{HashSet, HashMap};
    use crate::data_types::IndexedConclusion;

    #[test]
    fn test_query_where_conditional() {
        let mut calendar = Calendar::new(String::from("Calendar_UUID"));

        calendar.indexed_categories.terms.insert(
            String::from("CATEGORY_ONE"),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("CATEGORY_ONE_EVENT_INCLUDE_ALL"),    IndexedConclusion::Include(None)),
                        (String::from("CATEGORY_ONE_EVENT_INCLUDE_MOST"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("CATEGORY_ONE_EVENT_INCLUDE_FEW"),    IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),

                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                    ])
            }
        );

        calendar.indexed_categories.terms.insert(
            String::from("CATEGORY_TWO"),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("CATEGORY_TWO_EVENT_INCLUDE_ALL"),    IndexedConclusion::Include(None)),
                        (String::from("CATEGORY_TWO_EVENT_INCLUDE_MOST"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("CATEGORY_TWO_EVENT_INCLUDE_FEW"),    IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),

                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                    ])
            }
        );

        calendar.indexed_related_to.terms.insert(
            KeyValuePair::new(String::from("PARENT"), String::from("PARENT_UUID")),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("RELATED_TO_PARENT_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("RELATED_TO_PARENT_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("RELATED_TO_PARENT_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),

                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),     IndexedConclusion::Include(None)),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),    IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),     IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                    ])
            }
        );

        calendar.indexed_related_to.terms.insert(
            KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UUID")),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("RELATED_TO_CHILD_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("RELATED_TO_CHILD_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("RELATED_TO_CHILD_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),

                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),    IndexedConclusion::Include(None)),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),    IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                    ])
            }
        );

        // (
        //      ( CATEGORIES:CATEGORY_ONE OR RELATED-TO;RELTYPE=PARENT:PARENT_UUID )
        //      AND
        //      ( CATEGORIES:CATEGORY_TWO OR RELATED-TO;RELTYPE=CHILD:CHILD_UUID )
        // )
        let mut query_where_conditional = WhereConditional::Group(
            Box::new(
                WhereConditional::Operator(
                    Box::new(
                        WhereConditional::Group(
                            Box::new(
                                WhereConditional::Operator(
                                    Box::new(
                                        WhereConditional::Property(
                                            WhereConditionalProperty::Categories(
                                                String::from("CATEGORY_ONE")
                                            ),
                                            None,
                                        )
                                    ),
                                    Box::new(
                                        WhereConditional::Property(
                                            WhereConditionalProperty::RelatedTo(
                                                KeyValuePair::new(
                                                    String::from("PARENT"),
                                                    String::from("PARENT_UUID"),
                                                )
                                            ),
                                            None
                                        )
                                    ),
                                    WhereOperator::Or,
                                    None,
                                )
                            ),
                            None
                        )
                    ),
                    Box::new(
                        WhereConditional::Group(
                            Box::new(
                                WhereConditional::Operator(
                                    Box::new(
                                        WhereConditional::Property(
                                            WhereConditionalProperty::Categories(
                                                String::from("CATEGORY_TWO")
                                            ),
                                            None,
                                        )
                                    ),
                                    Box::new(
                                        WhereConditional::Property(
                                            WhereConditionalProperty::RelatedTo(
                                                KeyValuePair::new(
                                                    String::from("CHILD"),
                                                    String::from("CHILD_UUID"),
                                                )
                                            ),
                                            None,
                                        )
                                    ),
                                    WhereOperator::Or,
                                    None,
                                )
                            ),
                            None
                        )
                    ),
                    WhereOperator::And,
                    None,
                )
            ),
            None
        );

        assert_eq!(
            query_where_conditional.get_where_conditional_analyses(0),
            Err(format!("None WhereConditionalAnalysis at Group")),
        );

        assert_eq!(
            query_where_conditional.execute(&calendar).unwrap(),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("ALL_CATEGORIES_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),

                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_ALL"),  IndexedConclusion::Include(None)),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_MOST"), IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("RELATED_TO_ALL_EVENT_INCLUDE_FEW"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                    ])
            }
        );

        let where_conditional_analyses = query_where_conditional.get_where_conditional_analyses(0).unwrap();

        assert_eq!(where_conditional_analyses.len(), 10);

        macro_rules! assert_where_conditional_analysis {
            ($array: ident, $index: expr, $depth: expr, $output_count: expr, $details: expr) => {
                let (depth, details, where_conditional_analysis) = &$array[$index];

                assert_eq!(depth, &$depth);
                assert_eq!(details, &$details);
                assert_eq!(where_conditional_analysis.output_count, $output_count);
                assert_eq!(where_conditional_analysis.elapsed_duration.is_zero(), false);
            }
        }

        assert_where_conditional_analysis!(where_conditional_analyses, 0, 0, 6usize,  String::from("Group"));
        assert_where_conditional_analysis!(where_conditional_analyses, 1, 1, 6usize,  String::from("Operator: And"));
        assert_where_conditional_analysis!(where_conditional_analyses, 2, 2, 12usize, String::from("Group"));
        assert_where_conditional_analysis!(where_conditional_analyses, 3, 3, 12usize, String::from("Operator: Or"));
        assert_where_conditional_analysis!(where_conditional_analyses, 4, 4, 6usize,  String::from("Property: \"CATEGORIES:CATEGORY_ONE\""));
        assert_where_conditional_analysis!(where_conditional_analyses, 5, 4, 6usize,  String::from("Property: \"RELATED-TO;RELTYPE=PARENT:PARENT_UUID\""));
        assert_where_conditional_analysis!(where_conditional_analyses, 6, 2, 12usize, String::from("Group"));
        assert_where_conditional_analysis!(where_conditional_analyses, 7, 3, 12usize, String::from("Operator: Or"));
        assert_where_conditional_analysis!(where_conditional_analyses, 8, 4, 6usize,  String::from("Property: \"CATEGORIES:CATEGORY_TWO\""));
        assert_where_conditional_analysis!(where_conditional_analyses, 9, 4, 6usize,  String::from("Property: \"RELATED-TO;RELTYPE=CHILD:CHILD_UUID\""));
    }

}
