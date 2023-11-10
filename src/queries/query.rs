use crate::data_types::{KeyValuePair, InvertedCalendarIndexTerm, Calendar, EventInstance, EventInstanceIterator, LowerBoundFilterCondition, UpperBoundFilterCondition};

use crate::queries::results::QueryResults;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{LowerBoundRangeCondition, UpperBoundRangeCondition};
use crate::queries::indexed_property_filters::WhereConditional;

use crate::data_types::MergedIterator;

#[derive(Debug, PartialEq, Clone)]
pub struct Query {
    where_conditional:           Option<WhereConditional>,
    ordering_condition:          OrderingCondition,
    lower_bound_range_condition: Option<LowerBoundRangeCondition>,
    upper_bound_range_condition: Option<UpperBoundRangeCondition>,
    limit:                       usize,
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


        /*
        TODO: DtStartGeoDist and GeoDistDtStart
        match self.ordering_condition {
            OrderingCondition::DtStart => {
            },

            OrderingCondition::DtStartGeoDist(geo_point) => {
            },

            OrderingCondition::GeoDistDtStart(geo_point) => {
            },
        }
        */

        self.execute_for_dtstart_ordering(calendar, &mut query_results, &where_conditional_result);

        Ok(query_results)
    }

    fn execute_for_dtstart_ordering(&self, calendar: &Calendar, query_results: &mut QueryResults, where_conditional_result: &Option<InvertedCalendarIndexTerm>) {
        let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> = MergedIterator::new();

        let lower_bound_filter_condition: Option<LowerBoundFilterCondition> = self.lower_bound_range_condition.clone().and_then(|lower_bound_range_condition| {
            let lower_bound_filter_condition: LowerBoundFilterCondition = lower_bound_range_condition.into();

            Some(lower_bound_filter_condition)
        });

        let upper_bound_filter_condition: Option<UpperBoundFilterCondition> = self.upper_bound_range_condition.clone().and_then(|upper_bound_range_condition| {
            let upper_bound_filter_condition: UpperBoundFilterCondition = upper_bound_range_condition.into();

            Some(upper_bound_filter_condition)
        });


        match where_conditional_result {
            Some(inverted_calendar_index_term) => {
                for (event_uuid, indexed_conclusion) in &inverted_calendar_index_term.events {
                    let Some(event) = calendar.events.get(event_uuid) else {
                        // TODO: handle missing indexed event...

                        continue;
                    };

                    let event_instance_iterator =
                        EventInstanceIterator::new(
                            event,
                            None,
                            lower_bound_filter_condition.clone(),
                            upper_bound_filter_condition.clone(),
                            Some(indexed_conclusion.clone()),
                        ).unwrap(); // TODO: handle this properly...

                    let _result = merged_iterator.add_iter(event_uuid.clone(), event_instance_iterator);
                }
            },

            None => {
                for (event_uuid, event) in &calendar.events {
                    let event_instance_iterator =
                        EventInstanceIterator::new(
                            event,
                            None,
                            lower_bound_filter_condition.clone(),
                            upper_bound_filter_condition.clone(),
                            None,
                        ).unwrap(); // TODO: handle this properly...

                    let _result = merged_iterator.add_iter(event_uuid.clone(), event_instance_iterator);
                }
            },
        }

        for (_, event_instance) in merged_iterator {
            query_results.push(event_instance);

            if query_results.len() > self.limit {
                break;
            }
        }

    }
}

impl Default for Query {

    fn default() -> Self {
        Query {
            where_conditional:           None,
            ordering_condition:          OrderingCondition::DtStart,
            lower_bound_range_condition: None,
            upper_bound_range_condition: None,
            limit:                       50,
        }
    }

}
