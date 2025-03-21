use std::str::FromStr;

use chrono_tz::Tz;

use crate::{
    Calendar, Event, EventInstance, EventInstanceIterator, IndexedConclusion,
    InvertedCalendarIndexTerm, LowerBoundFilterCondition, UpperBoundFilterCondition,
    KeyValuePair, GeoDistance, GeoPoint,
};

use crate::queries::indexed_property_filters::WhereConditional;
use crate::queries::query::{Query, QueryIndexAccessor};
use crate::queries::query_parser::parse_query_string;
use crate::queries::results::QueryResults;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{
    LowerBoundRangeCondition, UpperBoundRangeCondition,
};

use redical_ical::properties::ICalendarDateTimeProperty;

use crate::MergedIterator;

/// This struct implements the `QueryIndexAccessor` trait and it's purpose is to specify the index
/// term retrieval logic specific to the requirements of the event instance query.
///
/// The event instance query is concerned with events and it's overrides because the event instance
/// extrapolation process combines them to ensure overridden indexed property values are reflected
/// for specific occurrences.
pub struct EventInstanceQueryIndexAccessor<'cal> {
    calendar: &'cal Calendar,
    event_uids: Vec<String>,
}

impl<'cal> QueryIndexAccessor<'cal> for EventInstanceQueryIndexAccessor<'cal> {
    fn new(calendar: &'cal Calendar) -> Self {
        let event_uids = calendar.events.keys().cloned().collect();

        EventInstanceQueryIndexAccessor {
            calendar,
            event_uids,
        }
    }

    // For UID, we just return an "include all" consensus for that event UID.
    fn search_uid_index(&self, uid: &str) -> InvertedCalendarIndexTerm {
        InvertedCalendarIndexTerm::new_with_event(
            uid.to_owned(),
            IndexedConclusion::Include(None),
        )
    }

    fn search_location_type_index(&self, location_type: &str) -> InvertedCalendarIndexTerm {
        self.calendar
            .indexed_location_type
            .get_term(&location_type.to_string())
            .unwrap_or(&InvertedCalendarIndexTerm::new())
            .to_owned()
    }

    fn search_categories_index(&self, category: &str) -> InvertedCalendarIndexTerm {
        self.calendar
            .indexed_categories
            .get_term(&category.to_string())
            .unwrap_or(&InvertedCalendarIndexTerm::new())
            .to_owned()
    }

    fn search_related_to_index(&self, reltype_uids: &KeyValuePair) -> InvertedCalendarIndexTerm {
        self.calendar
            .indexed_related_to
            .get_term(reltype_uids)
            .unwrap_or(&InvertedCalendarIndexTerm::new())
            .to_owned()
    }

    fn search_geo_index(&self, distance: &GeoDistance, long_lat: &GeoPoint) -> InvertedCalendarIndexTerm {
        self.calendar
            .indexed_geo
            .locate_within_distance(long_lat, distance)
    }

    fn search_class_index(&self, class: &str) -> InvertedCalendarIndexTerm {
        self.calendar
            .indexed_class
            .get_term(&class.to_string())
            .unwrap_or(&InvertedCalendarIndexTerm::new())
            .to_owned()
    }

    fn search_not_uid_index(&self, uid: &str) -> InvertedCalendarIndexTerm {
        let mut inverse_matches = InvertedCalendarIndexTerm::new();

        let included_event_uids_iter = self.event_uids
            .iter()
            .filter(|event_uid| *event_uid != uid);

        for event_uid in included_event_uids_iter {
            inverse_matches.insert_included_event(event_uid.to_owned(), None);
        }

        inverse_matches
    }

    fn search_not_location_type_index(&self, location_type: &str) -> InvertedCalendarIndexTerm {
        self.calendar.indexed_location_type.get_not_term(
            &location_type.to_string(),
            &self.event_uids,
        )
    }

    fn search_not_categories_index(&self, category: &str) -> InvertedCalendarIndexTerm {
        self.calendar.indexed_categories.get_not_term(
            &category.to_string(),
            &self.event_uids,
        )
    }

    fn search_not_related_to_index(&self, reltype_uids: &KeyValuePair) -> InvertedCalendarIndexTerm {
        self.calendar.indexed_related_to.get_not_term(
            reltype_uids,
            &self.event_uids,
        )
    }

    fn search_not_geo_index(&self, distance: &GeoDistance, long_lat: &GeoPoint) -> InvertedCalendarIndexTerm {
        self.calendar.indexed_geo.locate_not_within_distance(
            long_lat,
            distance,
            &self.event_uids,
        )
    }

    fn search_not_class_index(&self, class: &str) -> InvertedCalendarIndexTerm {
        self.calendar.indexed_class.get_not_term(
            &class.to_string(),
            &self.event_uids,
        )
    }
}

/// This struct implements all the query logic specific to querying all the event instances on a
/// calendar extrapolated from all the contained events (including event specific occurrence
/// overrides).
#[derive(Debug, PartialEq, Clone)]
pub struct EventInstanceQuery {
    pub where_conditional: Option<WhereConditional>,
    pub ordering_condition: OrderingCondition,
    pub lower_bound_range_condition: Option<LowerBoundRangeCondition>,
    pub upper_bound_range_condition: Option<UpperBoundRangeCondition>,
    pub in_timezone: Tz,
    pub distinct_uids: bool,
    pub offset: usize,
    pub limit: usize,
}

impl FromStr for EventInstanceQuery {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_query = parse_query_string(input.trim())?;

        Ok(parsed_query)
    }
}

impl Query<EventInstance> for EventInstanceQuery {
    fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults<EventInstance>, String> {
        let query_index_accessor = EventInstanceQueryIndexAccessor::new(calendar);

        let where_conditional_result = if let Some(where_conditional) = &mut self.where_conditional
        {
            Some(where_conditional.execute(&query_index_accessor)?)
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

impl EventInstanceQuery {
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

    fn populate_merged_iterator_for_dtstart_ordering<'iter, 'cal: 'iter>(
        &self,
        calendar: &'cal Calendar,
        merged_iterator: &'iter mut MergedIterator<EventInstance, EventInstanceIterator<'cal>>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let lower_bound_filter_condition = self.get_lower_bound_filter_condition();
        let upper_bound_filter_condition = self.get_upper_bound_filter_condition();

        match where_conditional_result {
            Some(inverted_calendar_index_term) => {
                for (event_uid, indexed_conclusion) in &inverted_calendar_index_term.events {
                    let Some(event) = calendar.events.get(event_uid) else {
                        // TODO: handle missing indexed event...

                        continue;
                    };

                    self.add_event_to_merged_iterator(
                        event,
                        merged_iterator,
                        &lower_bound_filter_condition,
                        &upper_bound_filter_condition,
                        &Some(indexed_conclusion.clone()),
                    )?;
                }
            }

            None => {
                for event in calendar.events.values() {
                    self.add_event_to_merged_iterator(
                        event,
                        merged_iterator,
                        &lower_bound_filter_condition,
                        &upper_bound_filter_condition,
                        &None,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn execute_for_dtstart_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults<EventInstance>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> =
            MergedIterator::new();

        self.populate_merged_iterator_for_dtstart_ordering(
            calendar,
            &mut merged_iterator,
            where_conditional_result,
        )?;

        for (_, event_instance) in merged_iterator {
            if query_results.len() >= self.limit {
                break;
            }

            query_results.push(event_instance);
        }

        Ok(())
    }

    fn execute_for_dtstart_geo_dist_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults<EventInstance>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> =
            MergedIterator::new();

        self.populate_merged_iterator_for_dtstart_ordering(
            calendar,
            &mut merged_iterator,
            where_conditional_result,
        )?;

        // This is functionally similar to the DtStart ordering, except we need to include all the
        // EventInstances sharing the same dtstart_timestamp before truncating so that they can be
        // ordered by geographical distance.
        //
        // We do this to prevent a group of EventInstances sharing the same dtstart_timestamp from
        // being cut off half way through when the later EventInstances are closer geographically
        // than those pulled in earlier.
        //
        // We can enforce the result limit after this has finished, as the result set will sort
        // itself.
        let mut previous_dtstart_timestamp = None;

        for (_, event_instance) in merged_iterator {
            let is_unique_dtstart_timestamp =
                previous_dtstart_timestamp.is_some_and(|dtstart_timestamp| {
                    dtstart_timestamp != event_instance.dtstart.get_utc_timestamp()
                });

            if is_unique_dtstart_timestamp && query_results.len() >= self.limit {
                break;
            }

            previous_dtstart_timestamp = Some(event_instance.dtstart.get_utc_timestamp());

            query_results.push(event_instance);
        }

        query_results.truncate(self.limit);

        Ok(())
    }

    fn execute_for_geo_dist_dtstart_ordering(
        &self,
        geo_point: &GeoPoint,
        calendar: &Calendar,
        query_results: &mut QueryResults<EventInstance>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) -> Result<(), String> {
        let lower_bound_filter_condition = self.get_lower_bound_filter_condition();
        let upper_bound_filter_condition = self.get_upper_bound_filter_condition();

        for (point, _distance) in calendar
            .indexed_geo
            .coords
            .nearest_neighbor_iter_with_distance_2(&geo_point.to_point())
        {
            let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> =
                MergedIterator::new();

            let current_inverted_index_calendar_term = match where_conditional_result {
                Some(inverted_calendar_index_term) => {
                    InvertedCalendarIndexTerm::merge_and(&point.data, inverted_calendar_index_term)
                }

                None => point.data.to_owned(),
            };

            for (event_uid, indexed_conclusion) in &current_inverted_index_calendar_term.events {
                let Some(event) = calendar.events.get(event_uid) else {
                    // TODO: handle missing indexed event...

                    continue;
                };

                self.add_event_to_merged_iterator(
                    event,
                    &mut merged_iterator,
                    &lower_bound_filter_condition,
                    &upper_bound_filter_condition,
                    &Some(indexed_conclusion.clone()),
                )?;
            }

            for (_, event_instance) in merged_iterator {
                if query_results.len() >= self.limit {
                    break;
                }

                // TODO: Consider maybe reusing the distance available from iterator instead of
                //       wastefully re-calculating it.
                query_results.push(event_instance);
            }
        }

        Ok(())
    }

    fn add_event_to_merged_iterator<'iter, 'evt: 'iter>(
        &self,
        event: &'evt Event,
        merged_iterator: &'iter mut MergedIterator<EventInstance, EventInstanceIterator<'evt>>,
        lower_bound_filter_condition: &Option<LowerBoundFilterCondition>,
        upper_bound_filter_condition: &Option<UpperBoundFilterCondition>,
        filtering_indexed_conclusion: &Option<IndexedConclusion>,
    ) -> Result<(), String> {
        let limit = if self.distinct_uids { Some(1) } else { None };

        let event_uid: String = event.uid.uid.to_string();

        let event_instance_iterator = EventInstanceIterator::new(
            event,
            limit,
            lower_bound_filter_condition.clone(),
            upper_bound_filter_condition.clone(),
            filtering_indexed_conclusion.clone(),
        )?;

        if let Err(error) = merged_iterator.add_iter(event_uid, event_instance_iterator) {
            Err(error)
        } else {
            Ok(())
        }
    }
}

impl Default for EventInstanceQuery {
    fn default() -> Self {
        EventInstanceQuery {
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
    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    use std::collections::{HashSet, HashMap};

    #[test]
    fn test_uid_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let event_one = Event::parse_ical("EVENT_ONE", "").unwrap();
        let event_two = Event::parse_ical("EVENT_TWO", "").unwrap();
        let event_three = Event::parse_ical("EVENT_THREE", "").unwrap();

        calendar.insert_event(event_one);
        calendar.insert_event(event_two);
        calendar.insert_event(event_three);
        calendar.rebuild_indexes().unwrap();

        let accessor = EventInstanceQueryIndexAccessor::new(&calendar);

        // Positive matching: term exists
        assert_eq!(
            accessor.search_uid_index("EVENT_ONE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("EVENT_ONE"), IndexedConclusion::Include(None)),
                ]),
            }
        );

        // TODO: Shouldn't this return an empty event set?
        // Positive matching: term does not exist
        assert_eq!(
            accessor.search_uid_index("EVENT_FOUR"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("EVENT_FOUR"), IndexedConclusion::Include(None)),
                ]),
            }
        );

        // Negative matching: term exists
        assert_eq!(
            accessor.search_not_uid_index("EVENT_ONE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("EVENT_TWO"), IndexedConclusion::Include(None)),
                    (String::from("EVENT_THREE"), IndexedConclusion::Include(None)),
                ]),
            }
        );

        // Negative matching: term does not exist
        assert_eq!(
            accessor.search_not_uid_index("EVENT_FOUR"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("EVENT_ONE"), IndexedConclusion::Include(None)),
                    (String::from("EVENT_TWO"), IndexedConclusion::Include(None)),
                    (String::from("EVENT_THREE"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_location_type_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let indexed_location_types = [
            (
                String::from("ONLINE"),
                [
                    (String::from("All online"), IndexedConclusion::Include(None)),
                    (String::from("Mostly online"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly in person"), IndexedConclusion::Exclude(Some([100].into()))),
                ]
            ),
            (
                String::from("IN-PERSON"),
                [
                    (String::from("All in person"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in person"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly online"), IndexedConclusion::Exclude(Some([100].into()))),
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

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All in person"),
            String::from("All online"),
            String::from("Mostly in person"),
            String::from("Mostly online"),
            String::from("Other event 1"),
            String::from("Other event 2"),
        ];

        let accessor = EventInstanceQueryIndexAccessor { calendar: &calendar, event_uids };

        // Positive matching: term exists
        assert_eq!(
            accessor.search_location_type_index("ONLINE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All online"), IndexedConclusion::Include(None)),
                    (String::from("Mostly online"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly in person"), IndexedConclusion::Exclude(Some([100].into()))),
                ]),
            }
        );

        // Positive matching: term does not exist
        assert_eq!(
            accessor.search_location_type_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::new(),
            }
        );

        // Negative matching: term exists
        assert_eq!(
            accessor.search_not_location_type_index("ONLINE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("Mostly online"), IndexedConclusion::Exclude(Some([100].into()))),
                    (String::from("All in person"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in person"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ])
            }
        );

        // Negative matching: term does not exist
        assert_eq!(
            accessor.search_not_categories_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All online"), IndexedConclusion::Include(None)),
                    (String::from("Mostly online"), IndexedConclusion::Include(None)),
                    (String::from("All in person"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in person"), IndexedConclusion::Include(None)),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_categories_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let indexed_categories = [
            (
                String::from("Adults"),
                [
                    (String::from("All adults"), IndexedConclusion::Include(None)),
                    (String::from("Mostly adults"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly kids"), IndexedConclusion::Exclude(Some([100].into()))),
                ]
            ),
            (
                String::from("Kids"),
                [
                    (String::from("All kids"), IndexedConclusion::Include(None)),
                    (String::from("Mostly kids"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly adults"), IndexedConclusion::Exclude(Some([100].into()))),
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

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All adults"),
            String::from("All kids"),
            String::from("Mostly adults"),
            String::from("Mostly kids"),
            String::from("Other event 1"),
            String::from("Other event 2"),
        ];

        let accessor = EventInstanceQueryIndexAccessor { calendar: &calendar, event_uids };

        // Positive matching: term exists
        assert_eq!(
            accessor.search_categories_index("Kids"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All kids"), IndexedConclusion::Include(None)),
                    (String::from("Mostly kids"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly adults"), IndexedConclusion::Exclude(Some([100].into()))),
                ]),
            }
        );

        // Positive matching: term does not exist
        assert_eq!(
            accessor.search_categories_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::new(),
            }
        );

        // Negative matching: term exists
        assert_eq!(
            accessor.search_not_categories_index("Kids"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("Mostly kids"), IndexedConclusion::Exclude(Some([100].into()))),
                    (String::from("All adults"), IndexedConclusion::Include(None)),
                    (String::from("Mostly adults"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ])
            }
        );

        // Negative matching: term does not exist
        assert_eq!(
            accessor.search_not_categories_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All adults"), IndexedConclusion::Include(None)),
                    (String::from("Mostly adults"), IndexedConclusion::Include(None)),
                    (String::from("All kids"), IndexedConclusion::Include(None)),
                    (String::from("Mostly kids"), IndexedConclusion::Include(None)),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_related_to_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let indexed_related_to = [
            (
                KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-1"),
                ),
                [
                    (String::from("All account-1"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-1"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly account-2"), IndexedConclusion::Exclude(Some([100].into()))),
                ]
            ),
            (
                KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-2"),
                ),
                [
                    (String::from("All account-2"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-2"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly account-1"), IndexedConclusion::Exclude(Some([100].into()))),
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

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All account-1"),
            String::from("All account-2"),
            String::from("Mostly account-1"),
            String::from("Mostly account-2"),
            String::from("Other event 1"),
            String::from("Other event 2"),
        ];

        let accessor = EventInstanceQueryIndexAccessor { calendar: &calendar, event_uids };

        // Positive matching: term exists
        assert_eq!(
            accessor.search_related_to_index(
                &KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-1"),
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All account-1"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-1"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly account-2"), IndexedConclusion::Exclude(Some([100].into()))),
                ]),
            }
        );

        // Positive matching: term does not exist
        assert_eq!(
            accessor.search_related_to_index(
                &KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-4"),
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::new(),
            }
        );

        // Negative matching: term exists
        assert_eq!(
            accessor.search_not_related_to_index(
                &KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("account-1"),
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("Mostly account-1"), IndexedConclusion::Exclude(Some([100].into()))),
                    (String::from("All account-2"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-2"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ])
            }
        );

        // Negative matching: term does not exist
        assert_eq!(
            accessor.search_not_related_to_index(
                &KeyValuePair::new(
                    String::from("X-ACCOUNT"),
                    String::from("FOOBAR"),
                )
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All account-1"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-1"), IndexedConclusion::Include(None)),
                    (String::from("All account-2"), IndexedConclusion::Include(None)),
                    (String::from("Mostly account-2"), IndexedConclusion::Include(None)),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_geo_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        const LONDON: GeoPoint = GeoPoint { lat: 51.5074_f64, long: -0.1278_f64 };
        const OXFORD: GeoPoint = GeoPoint { lat: 51.8773_f64, long: -1.2475878_f64 };
        const NEW_YORK_CITY: GeoPoint = GeoPoint { lat: 40.7128_f64, long: -74.006_f64 };

        let indexed_geo = [
            (
                LONDON,
                [
                    (String::from("All in London"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in London"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly in Oxford"), IndexedConclusion::Exclude(Some([100].into()))),
                ]
            ),
            (
                OXFORD,
                [
                    (String::from("All in Oxford"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in Oxford"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly in London"), IndexedConclusion::Exclude(Some([100].into()))),
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

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All in London"),
            String::from("All in Oxford"),
            String::from("Mostly in London"),
            String::from("Mostly in Oxford"),
            String::from("Other event 1"),
            String::from("Other event 2"),
        ];

        let accessor = EventInstanceQueryIndexAccessor { calendar: &calendar, event_uids };

        let search_distance = GeoDistance::new_from_miles_float(10.0_f64);

        // Positive matching: events located within search distance
        assert_eq!(
            accessor.search_geo_index(&search_distance, &OXFORD),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All in Oxford"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in Oxford"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly in London"), IndexedConclusion::Exclude(Some([100].into()))),
                ]),
            }
        );

        // Positive matching: no events located within search distance
        assert_eq!(
            accessor.search_geo_index(&search_distance, &NEW_YORK_CITY),
            InvertedCalendarIndexTerm {
                events: HashMap::new(),
            }
        );

        // Negative matching: some events located outside search distance
        assert_eq!(
            accessor.search_not_geo_index(&search_distance, &OXFORD),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("Mostly in Oxford"), IndexedConclusion::Exclude(Some([100].into()))),
                    (String::from("All in London"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in London"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ])
            }
        );

        // Negative matching: all events outside search distance
        assert_eq!(
            accessor.search_not_geo_index(&search_distance, &NEW_YORK_CITY),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All in London"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in London"), IndexedConclusion::Include(None)),
                    (String::from("All in Oxford"), IndexedConclusion::Include(None)),
                    (String::from("Mostly in Oxford"), IndexedConclusion::Include(None)),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_class_index_retrieval() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let indexed_class = [
            (
                String::from("PUBLIC"),
                [
                    (String::from("All public"), IndexedConclusion::Include(None)),
                    (String::from("Mostly public"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly private"), IndexedConclusion::Exclude(Some([100].into()))),
                ]
            ),
            (
                String::from("PRIVATE"),
                [
                    (String::from("All private"), IndexedConclusion::Include(None)),
                    (String::from("Mostly private"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly public"), IndexedConclusion::Exclude(Some([100].into()))),
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

        // Contains extra event uids to simulate events referenced on other indexes.
        let event_uids = vec![
            String::from("All public"),
            String::from("All private"),
            String::from("Mostly public"),
            String::from("Mostly private"),
            String::from("Other event 1"),
            String::from("Other event 2"),
        ];

        let accessor = EventInstanceQueryIndexAccessor { calendar: &calendar, event_uids };

        // Positive matching: term exists
        assert_eq!(
            accessor.search_class_index("PRIVATE"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All private"), IndexedConclusion::Include(None)),
                    (String::from("Mostly private"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Mostly public"), IndexedConclusion::Exclude(Some([100].into()))),
                ]),
            }
        );

        // Positive matching: term does not exist
        assert_eq!(
            accessor.search_class_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::new(),
            }
        );

        // Negative matching: term exists
        assert_eq!(
            accessor.search_not_class_index("PUBLIC"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("Mostly public"), IndexedConclusion::Exclude(Some([100].into()))),
                    (String::from("All private"), IndexedConclusion::Include(None)),
                    (String::from("Mostly private"), IndexedConclusion::Include(Some([100].into()))),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ])
            }
        );

        // Negative matching: term does not exist
        assert_eq!(
            accessor.search_not_class_index("FOOBAR"),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("All public"), IndexedConclusion::Include(None)),
                    (String::from("Mostly public"), IndexedConclusion::Include(None)),
                    (String::from("All private"), IndexedConclusion::Include(None)),
                    (String::from("Mostly private"), IndexedConclusion::Include(None)),
                    (String::from("Other event 1"), IndexedConclusion::Include(None)),
                    (String::from("Other event 2"), IndexedConclusion::Include(None)),
                ]),
            }
        );
    }

    #[test]
    fn test_event_instance_query_index_accessor() {
        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        let categories_index_entries = [
            (String::from("FULLY_INCLUDED_EVENT_UID"),     String::from("CATEGORY"), &IndexedConclusion::Include(None)),
            (String::from("FULLY_EXCLUDED_EVENT_UID"),     String::from("CATEGORY"), &IndexedConclusion::Exclude(None)),
            (String::from("PARTIALLY_INCLUDED_EVENT_UID"), String::from("CATEGORY"), &IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
            (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), String::from("CATEGORY"), &IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
        ];

        let location_type_index_entries = [
            (String::from("FULLY_INCLUDED_EVENT_UID"),     String::from("ONLINE"), &IndexedConclusion::Include(None)),
            (String::from("FULLY_EXCLUDED_EVENT_UID"),     String::from("ONLINE"), &IndexedConclusion::Exclude(None)),
            (String::from("PARTIALLY_INCLUDED_EVENT_UID"), String::from("ONLINE"), &IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
            (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), String::from("ONLINE"), &IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
        ];

        let related_to_index_entries = [
            (String::from("FULLY_INCLUDED_EVENT_UID"),     KeyValuePair::new(String::from("PARENT"), String::from("UID")), &IndexedConclusion::Include(None)),
            (String::from("FULLY_EXCLUDED_EVENT_UID"),     KeyValuePair::new(String::from("PARENT"), String::from("UID")), &IndexedConclusion::Exclude(None)),
            (String::from("PARTIALLY_INCLUDED_EVENT_UID"), KeyValuePair::new(String::from("PARENT"), String::from("UID")), &IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
            (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), KeyValuePair::new(String::from("PARENT"), String::from("UID")), &IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
        ];

        let geo_index_entries = [
            (String::from("FULLY_INCLUDED_EVENT_UID"),     GeoPoint::new(51.5074_f64, -0.1278_f64), &IndexedConclusion::Include(None)),
            (String::from("FULLY_EXCLUDED_EVENT_UID"),     GeoPoint::new(51.5074_f64, -0.1278_f64), &IndexedConclusion::Exclude(None)),
            (String::from("PARTIALLY_INCLUDED_EVENT_UID"), GeoPoint::new(51.5074_f64, -0.1278_f64), &IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
            (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), GeoPoint::new(51.5074_f64, -0.1278_f64), &IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
        ];

        let class_index_entries = [
            (String::from("FULLY_INCLUDED_EVENT_UID"),     String::from("PUBLIC"), &IndexedConclusion::Include(None)),
            (String::from("FULLY_EXCLUDED_EVENT_UID"),     String::from("PUBLIC"), &IndexedConclusion::Exclude(None)),
            (String::from("PARTIALLY_INCLUDED_EVENT_UID"), String::from("PUBLIC"), &IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
            (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), String::from("PUBLIC"), &IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
        ];

        categories_index_entries
            .into_iter()
            .for_each(|(event_uid, category, indexed_conclusion)| {
                let _ = calendar.indexed_categories.insert(event_uid, category, indexed_conclusion);
            });

        location_type_index_entries
            .into_iter()
            .for_each(|(event_uid, location_type, indexed_conclusion)| {
                let _ = calendar.indexed_location_type.insert(event_uid, location_type, indexed_conclusion);
            });

        related_to_index_entries
            .into_iter()
            .for_each(|(event_uid, reltype_pair, indexed_conclusion)| {
                let _ = calendar.indexed_related_to.insert(event_uid, reltype_pair, indexed_conclusion);
            });

        geo_index_entries
            .into_iter()
            .for_each(|(event_uid, geo_point, indexed_conclusion)| {
                let _ = calendar.indexed_geo.insert(event_uid, &geo_point, indexed_conclusion);
            });

        class_index_entries
            .into_iter()
            .for_each(|(event_uid, class, indexed_conclusion)| {
                let _ = calendar.indexed_class.insert(event_uid, class, indexed_conclusion);
            });

        let event_instance_query_index_accessor = EventInstanceQueryIndexAccessor::new(&calendar);

        assert_eq_sorted!(
            event_instance_query_index_accessor.search_uid_index("UID"),
            InvertedCalendarIndexTerm::new_with_event(String::from("UID"), IndexedConclusion::Include(None)),
        );

        // Test existing calendar categories index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_categories_index("CATEGORY"),
            InvertedCalendarIndexTerm::new_with_events(
                vec![
                    (String::from("FULLY_INCLUDED_EVENT_UID"),     IndexedConclusion::Include(None)),
                    (String::from("FULLY_EXCLUDED_EVENT_UID"),     IndexedConclusion::Exclude(None)),
                    (String::from("PARTIALLY_INCLUDED_EVENT_UID"), IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                    (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                ]
            ),
        );

        // Test non-existent calendar categories index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_categories_index("NON-EXISTENT"),
            InvertedCalendarIndexTerm::new(),
        );

        // Test existing calendar location type index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_location_type_index("ONLINE"),
            InvertedCalendarIndexTerm::new_with_events(
                vec![
                    (String::from("FULLY_INCLUDED_EVENT_UID"),     IndexedConclusion::Include(None)),
                    (String::from("FULLY_EXCLUDED_EVENT_UID"),     IndexedConclusion::Exclude(None)),
                    (String::from("PARTIALLY_INCLUDED_EVENT_UID"), IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                    (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                ]
            ),
        );

        // Test non-existent calendar location type index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_location_type_index("NON-EXISTENT"),
            InvertedCalendarIndexTerm::new(),
        );

        // Test existing calendar related to index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_related_to_index(
                &KeyValuePair::new(String::from("PARENT"), String::from("UID"))
            ),
            InvertedCalendarIndexTerm::new_with_events(
                vec![
                    (String::from("FULLY_INCLUDED_EVENT_UID"),     IndexedConclusion::Include(None)),
                    (String::from("FULLY_EXCLUDED_EVENT_UID"),     IndexedConclusion::Exclude(None)),
                    (String::from("PARTIALLY_INCLUDED_EVENT_UID"), IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                    (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                ]
            ),
        );

        // Test non-existent calendar related to index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_related_to_index(
                &KeyValuePair::new(String::from("PARENT"), String::from("NON-EXISTENT-UID"))
            ),
            InvertedCalendarIndexTerm::new(),
        );

        // Test existing calendar geo index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_geo_index(
                &GeoDistance::new_from_miles_float(10.0_f64),
                &GeoPoint::new(51.5074_f64, -0.1278_f64),
            ),
            InvertedCalendarIndexTerm::new_with_events(
                vec![
                    (String::from("FULLY_INCLUDED_EVENT_UID"),     IndexedConclusion::Include(None)),
                    (String::from("PARTIALLY_INCLUDED_EVENT_UID"), IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                    (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                ]
            ),
        );

        // Test non-existent calendar geo index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_geo_index(
                &GeoDistance::new_from_miles_float(10.0_f64),
                &GeoPoint::new(51.8773_f64, -2.1686_f64),
            ),
            InvertedCalendarIndexTerm::new(),
        );

        // Test existing calendar class index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_class_index("PUBLIC"),
            InvertedCalendarIndexTerm::new_with_events(
                vec![
                    (String::from("FULLY_INCLUDED_EVENT_UID"),     IndexedConclusion::Include(None)),
                    (String::from("FULLY_EXCLUDED_EVENT_UID"),     IndexedConclusion::Exclude(None)),
                    (String::from("PARTIALLY_INCLUDED_EVENT_UID"), IndexedConclusion::Include(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                    (String::from("PARTIALLY_EXCLUDED_EVENT_UID"), IndexedConclusion::Exclude(Some(HashSet::from([100_i64, 200_i64, 300_i64])))),
                ]
            ),
        );

        // Test non-existent calendar class index entry.
        assert_eq_sorted!(
            event_instance_query_index_accessor.search_class_index("NON-EXISTENT"),
            InvertedCalendarIndexTerm::new(),
        );
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            EventInstanceQuery::from_str("X-LIMIT:50 UNCONSUMED_ENDING"),
            Err(
                String::from("Error - parse error Eof at \"UNCONSUMED_ENDING\"")
            )
        );

        assert_eq!(
            EventInstanceQuery::from_str("INVALID"),
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
            EventInstanceQuery::from_str(query_string.as_str()),
            Ok(EventInstanceQuery {
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
