use crate::data_types::{KeyValuePair, InvertedCalendarIndexTerm, Calendar};

use crate::queries::results::QueryResults;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::indexed_property_filters::WhereConditional;

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
