use std::str::FromStr;

use chrono_tz::Tz;

use crate::{
    Calendar, Event, GeoPoint, IndexedConclusion,
    InvertedCalendarIndexTerm, LowerBoundFilterCondition, UpperBoundFilterCondition,
    FilterProperty,
};

use crate::queries::indexed_property_filters::WhereConditional;
use crate::queries::query::Query;
use crate::queries::query_parser::parse_query_string;
use crate::queries::results::QueryResults;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{
    LowerBoundRangeCondition, UpperBoundRangeCondition,
};

#[derive(Debug, PartialEq, Clone)]
pub struct EventQuery {
    pub where_conditional: Option<WhereConditional>,
    pub ordering_condition: OrderingCondition,
    pub lower_bound_range_condition: Option<LowerBoundRangeCondition>,
    pub upper_bound_range_condition: Option<UpperBoundRangeCondition>,
    pub in_timezone: Tz,
    pub distinct_uids: bool,
    pub offset: usize,
    pub limit: usize,
}

impl FromStr for EventQuery {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_query = parse_query_string(input.trim())?;

        Ok(parsed_query)
    }
}

impl Query<Event> for EventQuery {
    fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults<Event>, String> {
        let where_conditional_result = if let Some(where_conditional) = &mut self.where_conditional
        {
            Some(where_conditional.execute(calendar)?)
        } else {
            None
        };

        let mut query_results = QueryResults::new(
            self.ordering_condition.clone(),
            self.offset,
            self.distinct_uids,
        );

        match &self.ordering_condition {
            OrderingCondition::DtStart => {
                self.execute_for_dtstart_ordering(
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                )?;
            }

            OrderingCondition::DtStartGeoDist(_geo_point) => {
                self.execute_for_dtstart_geo_dist_ordering(
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                )?;
            }

            OrderingCondition::GeoDistDtStart(geo_point) => {
                self.execute_for_geo_dist_dtstart_ordering(
                    geo_point,
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                )?;
            }
        }

        Ok(query_results)
    }

    fn set_where_conditional(&mut self, where_conditional: Option<WhereConditional>) {
        self.where_conditional = where_conditional;
    }

    fn get_where_conditional(&self) -> &Option<WhereConditional> {
        &self.where_conditional
    }

    fn set_ordering_condition(&mut self, ordering_condition: OrderingCondition) {
        self.ordering_condition = ordering_condition;
    }

    fn set_lower_bound_range_condition(&mut self, lower_bound_range_condition: Option<LowerBoundRangeCondition>) {
        self.lower_bound_range_condition = lower_bound_range_condition;
    }

    fn set_upper_bound_range_condition(&mut self, upper_bound_range_condition: Option<UpperBoundRangeCondition>) {
        self.upper_bound_range_condition = upper_bound_range_condition;
    }

    fn set_in_timezone(&mut self, in_timezone: Tz) {
        self.in_timezone = in_timezone;
    }

    fn set_distinct_uids(&mut self, distinct_uids: bool) {
        self.distinct_uids = distinct_uids;
    }

    fn set_offset(&mut self, offset: usize) {
        self.offset = offset;
    }

    fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
    }
}

impl EventQuery {
    fn get_lower_bound_filter_condition(&self) -> Option<LowerBoundFilterCondition> {
        self.lower_bound_range_condition
            .to_owned()
            .map(LowerBoundFilterCondition::from)
    }

    fn get_upper_bound_filter_condition(&self) -> Option<UpperBoundFilterCondition> {
        self.upper_bound_range_condition
            .to_owned()
            .map(UpperBoundFilterCondition::from)
    }

    fn is_event_within_bound_filter_conditions(
        &self,
        event: &Event,
        lower_bound_filter_condition: &Option<LowerBoundFilterCondition>, 
        upper_bound_filter_condition: &Option<UpperBoundFilterCondition>,
    ) -> bool {
        let within_lower_bound_filter_condition =
            match lower_bound_filter_condition {
                Some(LowerBoundFilterCondition::GreaterThan(FilterProperty::DtStart(timestamp))) => {
                    event.schedule_properties.get_dtstart_timestamp().is_some_and(|dtstart_timestamp| &dtstart_timestamp > timestamp)
                }

                Some(LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtStart(timestamp))) => {
                    event.schedule_properties.get_dtstart_timestamp().is_some_and(|dtstart_timestamp| &dtstart_timestamp >= timestamp)
                }

                Some(LowerBoundFilterCondition::GreaterThan(FilterProperty::DtEnd(timestamp))) => {
                    event.schedule_properties.get_dtend_timestamp().is_some_and(|dtend_timestamp| &dtend_timestamp > timestamp)
                }

                Some(LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtEnd(timestamp))) => {
                    event.schedule_properties.get_dtend_timestamp().is_some_and(|dtend_timestamp| &dtend_timestamp >= timestamp)
                }

                _ => true
            };

        let within_upper_bound_filter_condition =
            match upper_bound_filter_condition {
                Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtStart(timestamp))) => {
                    event.schedule_properties.get_dtstart_timestamp().is_some_and(|dtstart_timestamp| &dtstart_timestamp < timestamp)
                }

                Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtStart(timestamp))) => {
                    event.schedule_properties.get_dtstart_timestamp().is_some_and(|dtstart_timestamp| &dtstart_timestamp <= timestamp)
                }

                Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtEnd(timestamp))) => {
                    event.schedule_properties.get_dtend_timestamp().is_some_and(|dtend_timestamp| &dtend_timestamp < timestamp)
                }

                Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtEnd(timestamp))) => {
                    event.schedule_properties.get_dtend_timestamp().is_some_and(|dtend_timestamp| &dtend_timestamp <= timestamp)
                }

                _ => true
            };

        within_lower_bound_filter_condition && within_upper_bound_filter_condition
    }

    #[allow(clippy::borrowed_box)]
    fn populate_sorted_vec_for_dtstart_ordering<'event, 'cal: 'event>(
        &self,
        calendar: &'cal Calendar,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<Vec<&'event Box<Event>>, String> {
        let lower_bound_filter_condition = self.get_lower_bound_filter_condition();
        let upper_bound_filter_condition = self.get_upper_bound_filter_condition();

        let mut sorted_events: Vec<&'event Box<Event>> = Vec::new();

        match where_conditional_result {
            Some(inverted_calendar_index_term) => {
                for (event_uid, indexed_conclusion) in &inverted_calendar_index_term.events {
                    let Some(event) = calendar.events.get(event_uid) else {
                        // TODO: handle missing indexed event...

                        continue;
                    };

                    // We only care about matching base properties defined on the Event, not any
                    // occurrence specific overrides - we are querying the events not the event
                    // instances extrapolated from those events and associated occurrence specific
                    // overrides.
                    //
                    // Exclude means that it is not present on the base Event, so we skip it.
                    if matches!(indexed_conclusion, IndexedConclusion::Exclude(_)) {
                        continue;
                    }

                    if self.is_event_within_bound_filter_conditions(event, &lower_bound_filter_condition, &upper_bound_filter_condition) {
                        sorted_events.push(event);
                    }
                }
            }

            None => {
                for event in calendar.events.values() {
                    if self.is_event_within_bound_filter_conditions(event, &lower_bound_filter_condition, &upper_bound_filter_condition) {
                        sorted_events.push(event);
                    }
                }
            }
        }

        sorted_events.sort_by(|event, other_event| {
            let timestamp       = event.schedule_properties.get_dtstart_timestamp();
            let other_timestamp = other_event.schedule_properties.get_dtstart_timestamp();

            timestamp.cmp(&other_timestamp)
        });

        Ok(sorted_events)
    }

    fn execute_for_dtstart_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults<Event>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let sorted_events = self.populate_sorted_vec_for_dtstart_ordering(calendar, where_conditional_result)?;

        for event in sorted_events {
            if query_results.len() >= self.limit {
                break;
            }

            query_results.push(*event.to_owned());
        }

        Ok(())
    }

    fn execute_for_dtstart_geo_dist_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults<Event>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let sorted_events = self.populate_sorted_vec_for_dtstart_ordering(calendar, where_conditional_result)?;

        // This is functionally similar to the DtStart ordering, except we need to include all the
        // Event sharing the same dtstart_timestamp before truncating so that they can be ordered
        // by geographical distance.
        //
        // We do this to prevent a group of EventInstances sharing the same dtstart_timestamp from
        // being cut off half way through when the later EventInstances are closer geographically
        // than those pulled in earlier.
        //
        // We can enforce the result limit after this has finished, as the result set will sort
        // itself.
        let mut previous_dtstart_timestamp = None;

        for event in sorted_events {
            let is_unique_dtstart_timestamp =
                previous_dtstart_timestamp.is_some_and(|dtstart_timestamp| {
                    dtstart_timestamp != event.schedule_properties.get_dtstart_timestamp()
                });

            if is_unique_dtstart_timestamp && query_results.len() >= self.limit {
                break;
            }

            previous_dtstart_timestamp = Some(event.schedule_properties.get_dtstart_timestamp());

            query_results.push(*event.to_owned());
        }

        query_results.truncate(self.limit);

        Ok(())
    }

    fn execute_for_geo_dist_dtstart_ordering(
        &self,
        geo_point: &GeoPoint,
        calendar: &Calendar,
        query_results: &mut QueryResults<Event>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let lower_bound_filter_condition = self.get_lower_bound_filter_condition();
        let upper_bound_filter_condition = self.get_upper_bound_filter_condition();

        'outer: for (point, _distance) in calendar
            .indexed_geo
            .coords
            .nearest_neighbor_iter_with_distance_2(&geo_point.to_point())
        {
            let current_inverted_index_calendar_term = match where_conditional_result {
                Some(inverted_calendar_index_term) => {
                    InvertedCalendarIndexTerm::merge_and(&point.data, inverted_calendar_index_term)
                }

                None => point.data.to_owned(),
            };

            for (event_uid, indexed_conclusion) in &current_inverted_index_calendar_term.events {
                // We only care about matching base properties defined on the Event, not any
                // occurrence specific overrides - we are querying the events not the event
                // instances extrapolated from those events and associated occurrence specific
                // overrides.
                //
                // Exclude means that it is not present on the base Event, so we skip it.
                if matches!(indexed_conclusion, IndexedConclusion::Exclude(_)) {
                    continue;
                }

                let Some(event) = calendar.events.get(event_uid) else {
                    // TODO: handle missing indexed event...
                    continue;
                };

                if self.is_event_within_bound_filter_conditions(event, &lower_bound_filter_condition, &upper_bound_filter_condition) {
                    if query_results.len() >= self.limit {
                        break 'outer;
                    }

                    // TODO: Consider maybe reusing the distance available from iterator instead of
                    //       wastefully re-calculating it.
                    query_results.push(*event.to_owned());
                }
            }
        }

        Ok(())
    }
}

impl Default for EventQuery {
    fn default() -> Self {
        EventQuery {
            where_conditional: None,
            ordering_condition: OrderingCondition::DtStart,
            lower_bound_range_condition: None,
            upper_bound_range_condition: None,
            in_timezone: Tz::UTC,
            distinct_uids: false,
            offset: 0,
            limit: 50,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::queries::indexed_property_filters::{
        WhereConditional, WhereConditionalProperty, WhereOperator,
    };

    use crate::queries::results_range_bounds::{
        LowerBoundRangeCondition, RangeConditionProperty, UpperBoundRangeCondition,
    };

    use crate::{GeoPoint, KeyValuePair};
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_from_str() {
        assert_eq!(
            EventQuery::from_str("X-LIMIT:50 UNCONSUMED_ENDING"),
            Err(
                String::from("Error - parse error Eof at \"UNCONSUMED_ENDING\"")
            )
        );

        assert_eq!(
            EventQuery::from_str("INVALID"),
            Err(
                String::from("Error - expected '(' at \"INVALID\" -- Context: GROUP")
            )
        );

        let query_string = [
            " ",
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO",
            "X-RELATED-TO:PARENT_UID",
            "X-LIMIT:50",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885",
            "   ",
        ]
        .join(" ");

        assert_eq!(
            EventQuery::from_str(query_string.as_str()),
            Ok(EventQuery {
                where_conditional: Some(WhereConditional::Operator(
                    Box::new(WhereConditional::Group(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                            )),
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Categories(String::from("CATEGORY_TWO")),
                            )),
                            WhereOperator::Or,
                        )),
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("PARENT_UID"),
                        )),
                    )),
                    WhereOperator::And,
                )),

                ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                    long: 2.36885,
                    lat: 48.85299,
                },),

                lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(RangeConditionProperty::DtStart(875779200))),
                upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(RangeConditionProperty::DtStart(878461200))),

                in_timezone: chrono_tz::Tz::Europe__Vilnius,

                distinct_uids: false,

                offset: 0,
                limit: 50,
            })
        );
    }
}
