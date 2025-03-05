use std::str::FromStr;

use chrono_tz::Tz;

use crate::{Calendar, InvertedCalendarIndexTerm, KeyValuePair, GeoDistance, GeoPoint};
use crate::queries::results::{QueryableEntity, QueryResults};

use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{
    LowerBoundRangeCondition, UpperBoundRangeCondition,
};

use crate::queries::indexed_property_filters::{
    WhereConditional, WhereOperator,
};

/// The purpose of this trait is to allow it's implementers to specify the index term retrieval
/// logic specific to the requirements of the query it is associated with.
///
/// For example for the event instance query, we are concerned with events and it's overrides
/// because they are combined in the event instance extrapolation process to override the indexed
/// property values for specific occurrences.
///
/// Whilst the event query is not at all concerned with the overrides as it is purely querying the
/// indexed base event properties, so the event specific implementation of this trait strips out
/// any `IndexedConclusion::Exclude` (exclude all with exceptions) returned. For any
/// `IndexedConclusion::Include` with exceptions defined, it will return a clone of it without any
/// exceptions.
pub trait QueryIndexAccessor<'cal> {
    fn new(calendar: &'cal Calendar) -> Self;

    // Positive matching
    fn search_uid_index(&self, uid: &str) -> InvertedCalendarIndexTerm;
    fn search_location_type_index(&self, location_type: &str) -> InvertedCalendarIndexTerm;
    fn search_categories_index(&self, category: &str) -> InvertedCalendarIndexTerm;
    fn search_related_to_index(&self, reltype_uids: &KeyValuePair) -> InvertedCalendarIndexTerm;
    fn search_geo_index(&self, distance: &GeoDistance, long_lat: &GeoPoint) -> InvertedCalendarIndexTerm;
    fn search_class_index(&self, class: &str) -> InvertedCalendarIndexTerm;

    // Negative (not) matching
    fn inverse_search_uid_index(&self, uid: &str) -> InvertedCalendarIndexTerm;
    fn inverse_search_location_type_index(&self, location_type: &str) -> InvertedCalendarIndexTerm;
    fn inverse_search_categories_index(&self, category: &str) -> InvertedCalendarIndexTerm;
    fn inverse_search_related_to_index(&self, reltype_uids: &KeyValuePair) -> InvertedCalendarIndexTerm;
    fn inverse_search_geo_index(&self, distance: &GeoDistance, long_lat: &GeoPoint) -> InvertedCalendarIndexTerm;
    fn inverse_search_class_index(&self, class: &str) -> InvertedCalendarIndexTerm;
}

/// The purpose of this trait is to allow it's implementers to specify the query logic specific to
/// the requirements of the query it is associated with (e.g. querying event instances or just
/// events).
pub trait Query<T: QueryableEntity>: FromStr + PartialEq + Clone + Default {
    fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults<T>, String>;
    fn set_where_conditional(&mut self, where_conditional: Option<WhereConditional>);
    fn get_where_conditional(&self) -> &Option<WhereConditional>;
    fn set_ordering_condition(&mut self, ordering_condition: OrderingCondition);
    fn set_lower_bound_range_condition(&mut self, lower_bound_range_condition: Option<LowerBoundRangeCondition>);
    fn set_upper_bound_range_condition(&mut self, upper_bound_range_condition: Option<UpperBoundRangeCondition>);
    fn set_in_timezone(&mut self, in_timezone: Tz);
    fn set_distinct_uids(&mut self, distinct_uids: bool);
    fn set_offset(&mut self, offset: usize);
    fn set_limit(&mut self, limit: usize);

    // TODO: Clean this up!
    fn insert_new_where_conditional(&mut self, new_where_conditional: Option<WhereConditional>) {
        let Some(new_where_conditional) = new_where_conditional else {
            return;
        };

        let new_where_conditional =
            if let Some(current_where_conditional) = self.get_where_conditional().to_owned() {
                WhereConditional::Operator(
                    Box::new(current_where_conditional),
                    Box::new(new_where_conditional),
                    WhereOperator::And,
                )
            } else {
                new_where_conditional
            };

        self.set_where_conditional(Some(new_where_conditional));
    }
}
