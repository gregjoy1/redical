use std::time::Duration;

use crate::data_types::{KeyValuePair, InvertedCalendarIndexTerm, Calendar};

pub struct Query {
    result_aggregate:  ResultAggregate,
    where_conditional: Option<WhereConditional>,
    result_ordering:   ResultOrdering,
    limit:             i64,
}

impl Default for Query {

    fn default() -> Self {
        Query {
            result_aggregate:  ResultAggregate::All,
            where_conditional: None,
            result_ordering:   ResultOrdering::OrderByDtstart,
            limit:             50,
        }
    }

}

pub enum GroupValue {
    UUID,
    RelatedTo(String),
}

pub enum ResultAggregate {
    All,
    First,
    AllOfGroup(GroupValue),
    FirstOfGroup(GroupValue),
}

pub enum ResultOrdering {
    OrderByDtstart,
    OrderByDtstartGeoDist,
    OrderByGeoDistDtstart,
}

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

}

pub enum WhereConditionalProperty {
    Categories(String),
    RelatedTo(KeyValuePair),
}

impl WhereConditionalProperty {

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

pub struct WhereConditionalAnalysis {
    elapsed_duration: Duration,
    output_count:     usize,
}
