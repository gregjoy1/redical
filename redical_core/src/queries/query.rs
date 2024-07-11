use std::str::FromStr;

use chrono_tz::Tz;

use crate::Calendar;
use crate::queries::results::QueryResults;

use crate::queries::indexed_property_filters::WhereConditional;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{
    LowerBoundRangeCondition, UpperBoundRangeCondition,
};

pub trait Query: FromStr + PartialEq + Clone + Default {
    fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults, String>;
    fn set_where_conditional(&mut self, where_conditional: Option<WhereConditional>);
    fn set_ordering_condition(&mut self, ordering_condition: OrderingCondition);
    fn set_lower_bound_range_condition(&mut self, lower_bound_range_condition: Option<LowerBoundRangeCondition>);
    fn set_upper_bound_range_condition(&mut self, upper_bound_range_condition: Option<UpperBoundRangeCondition>);
    fn set_in_timezone(&mut self, in_timezone: Tz);
    fn set_distinct_uids(&mut self, distinct_uids: bool);
    fn set_offset(&mut self, offset: usize);
    fn set_limit(&mut self, limit: usize);
}
