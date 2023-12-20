use crate::core::{
    Calendar, EventInstance, EventInstanceIterator, GeoPoint, InvertedCalendarIndexTerm,
    KeyValuePair, LowerBoundFilterCondition, UpperBoundFilterCondition,
};
use rrule::Tz;

use crate::core::parsers::ical_query::parse_query_string;
use crate::core::queries::indexed_property_filters::WhereConditional;
use crate::core::queries::results::QueryResults;
use crate::core::queries::results_ordering::OrderingCondition;
use crate::core::queries::results_range_bounds::{
    LowerBoundRangeCondition, UpperBoundRangeCondition,
};

use crate::core::MergedIterator;

#[derive(Debug, PartialEq, Clone)]
pub struct Query {
    pub where_conditional: Option<WhereConditional>,
    pub ordering_condition: OrderingCondition,
    pub lower_bound_range_condition: Option<LowerBoundRangeCondition>,
    pub upper_bound_range_condition: Option<UpperBoundRangeCondition>,
    pub in_timezone: Tz,
    pub limit: usize,
}

impl Query {
    pub fn execute(&mut self, calendar: &Calendar) -> Result<QueryResults, String> {
        let where_conditional_result = if let Some(where_conditional) = &mut self.where_conditional
        {
            Some(where_conditional.execute(calendar)?)
        } else {
            None
        };

        let mut query_results = QueryResults::new(self.ordering_condition.clone());

        match &self.ordering_condition {
            OrderingCondition::DtStart => {
                self.execute_for_dtstart_ordering(
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                );
            }

            OrderingCondition::DtStartGeoDist(_geo_point) => {
                self.execute_for_dtstart_geo_dist_ordering(
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                );
            }

            OrderingCondition::GeoDistDtStart(geo_point) => {
                self.execute_for_geo_dist_dtstart_ordering(
                    geo_point,
                    calendar,
                    &mut query_results,
                    &where_conditional_result,
                );
            }
        }

        Ok(query_results)
    }

    fn get_lower_bound_filter_condition(&self) -> Option<LowerBoundFilterCondition> {
        self.lower_bound_range_condition
            .clone()
            .and_then(|lower_bound_range_condition| {
                let lower_bound_filter_condition: LowerBoundFilterCondition =
                    lower_bound_range_condition.into();

                Some(lower_bound_filter_condition)
            })
    }

    fn get_upper_bound_filter_condition(&self) -> Option<UpperBoundFilterCondition> {
        self.upper_bound_range_condition
            .clone()
            .and_then(|upper_bound_range_condition| {
                let upper_bound_filter_condition: UpperBoundFilterCondition =
                    upper_bound_range_condition.into();

                Some(upper_bound_filter_condition)
            })
    }

    fn populate_merged_iterator_for_dtstart_ordering<'iter, 'cal: 'iter>(
        &self,
        calendar: &'cal Calendar,
        merged_iterator: &'iter mut MergedIterator<EventInstance, EventInstanceIterator<'cal>>,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) {
        let lower_bound_filter_condition = self.get_lower_bound_filter_condition();
        let upper_bound_filter_condition = self.get_upper_bound_filter_condition();

        match where_conditional_result {
            Some(inverted_calendar_index_term) => {
                for (event_uuid, indexed_conclusion) in &inverted_calendar_index_term.events {
                    let Some(event) = calendar.events.get(event_uuid) else {
                        // TODO: handle missing indexed event...

                        continue;
                    };

                    let event_instance_iterator = EventInstanceIterator::new(
                        event,
                        None,
                        lower_bound_filter_condition.clone(),
                        upper_bound_filter_condition.clone(),
                        Some(indexed_conclusion.clone()),
                    )
                    .unwrap(); // TODO: handle this properly...

                    let _result =
                        merged_iterator.add_iter(event_uuid.clone(), event_instance_iterator);
                }
            }

            None => {
                for (event_uuid, event) in &calendar.events {
                    let event_instance_iterator = EventInstanceIterator::new(
                        event,
                        None,
                        lower_bound_filter_condition.clone(),
                        upper_bound_filter_condition.clone(),
                        None,
                    )
                    .unwrap(); // TODO: handle this properly...

                    let _result =
                        merged_iterator.add_iter(event_uuid.clone(), event_instance_iterator);
                }
            }
        }
    }

    fn execute_for_dtstart_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) {
        let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> =
            MergedIterator::new();

        self.populate_merged_iterator_for_dtstart_ordering(
            calendar,
            &mut merged_iterator,
            where_conditional_result,
        );

        for (_, event_instance) in merged_iterator {
            if query_results.len() >= self.limit {
                break;
            }

            query_results.push(event_instance);
        }
    }

    fn execute_for_dtstart_geo_dist_ordering(
        &self,
        calendar: &Calendar,
        query_results: &mut QueryResults,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) {
        let mut merged_iterator: MergedIterator<EventInstance, EventInstanceIterator> =
            MergedIterator::new();

        self.populate_merged_iterator_for_dtstart_ordering(
            calendar,
            &mut merged_iterator,
            where_conditional_result,
        );

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
                    dtstart_timestamp != event_instance.dtstart_timestamp
                });

            if is_unique_dtstart_timestamp && query_results.len() >= self.limit {
                break;
            }

            previous_dtstart_timestamp = Some(event_instance.dtstart_timestamp.clone());

            query_results.push(event_instance);
        }

        query_results.truncate(self.limit);
    }

    fn execute_for_geo_dist_dtstart_ordering(
        &self,
        geo_point: &GeoPoint,
        calendar: &Calendar,
        query_results: &mut QueryResults,
        where_conditional_result: &Option<InvertedCalendarIndexTerm>,
    ) {
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

            for (event_uuid, indexed_conclusion) in &current_inverted_index_calendar_term.events {
                let Some(event) = calendar.events.get(event_uuid) else {
                    // TODO: handle missing indexed event...

                    continue;
                };

                let event_instance_iterator = EventInstanceIterator::new(
                    event,
                    None,
                    lower_bound_filter_condition.clone(),
                    upper_bound_filter_condition.clone(),
                    Some(indexed_conclusion.clone()),
                )
                .unwrap(); // TODO: handle this properly...

                let _result = merged_iterator.add_iter(event_uuid.clone(), event_instance_iterator);
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
    }
}

impl TryFrom<&str> for Query {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match parse_query_string(value.trim()) {
            Ok((remaining, parsed_query)) => {
                if remaining.is_empty() {
                    Ok(parsed_query)
                } else {
                    Err(format!("Unexpected values: {remaining}"))
                }
            }

            Err(error) => Err(error.to_string()),
        }
    }
}

impl Default for Query {
    fn default() -> Self {
        Query {
            where_conditional: None,
            ordering_condition: OrderingCondition::DtStart,
            lower_bound_range_condition: None,
            upper_bound_range_condition: None,
            in_timezone: Tz::UTC,
            limit: 50,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::core::queries::indexed_property_filters::{
        WhereConditional, WhereConditionalProperty, WhereOperator,
    };
    use crate::core::queries::results_range_bounds::{
        LowerBoundRangeCondition, RangeConditionProperty, UpperBoundRangeCondition,
    };
    use crate::core::{Calendar, Event, GeoPoint};
    use crate::testing::utils::{build_event_and_overrides_from_ical, build_event_from_ical};
    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    fn build_overridden_recurring_event() -> Event {
        build_event_and_overrides_from_ical(
            "overridden_recurring_event_UUID",
            vec![
                "DESCRIPTION:BASE description text.",
                "DTSTART:20210105T183000Z",
                "DTEND:20210105T190000Z",
                "RRULE:FREQ=WEEKLY;UNTIL=20210202T183000Z;INTERVAL=1",
                "CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO",
                "RELATED-TO;RELTYPE=PARENT:BASE_ParentdUUID",
                "RELATED-TO;RELTYPE=CHILD:BASE_ChildUUID",
            ],
            vec![
                (
                    "20210105T183000Z",
                    vec![
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID",
                    ],
                ),
                (
                    "20210112T183000Z",
                    vec![
                        "RELATED-TO;RELTYPE=CHILD:BASE_ChildUUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID",
                    ],
                ),
                (
                    "20210126T183000Z",
                    vec![
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID",
                    ],
                ),
            ],
        )
    }

    fn build_one_off_event() -> Event {
        build_event_from_ical(
            "one_off_event_UUID",
            vec![
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T183100Z",
                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE",
                "RELATED-TO;RELTYPE=CHILD:ChildUUID",
                "RELATED-TO;RELTYPE=PARENT:ParentUUID_One",
                "RELATED-TO;RELTYPE=PARENT:ParentUUID_Two",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two",
                "DESCRIPTION:Event description text.",
                "LOCATION:Event address text.",
            ],
        )
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            Query::try_from("X-LIMIT:50 UNCONSUMED_ENDING"),
            Err(
                String::from("Parsing Failure: VerboseError { errors: [(\"UNCONSUMED_ENDING\", Char('(')), (\"UNCONSUMED_ENDING\", Nom(Alt)), (\"X-LIMIT:50 UNCONSUMED_ENDING\", Context(\"outer parse query string\"))] }")
            )
        );

        assert_eq!(
            Query::try_from("INVALID"),
            Err(
                String::from("Parsing Failure: VerboseError { errors: [(\"INVALID\", Char('(')), (\"INVALID\", Nom(Alt)), (\"INVALID\", Context(\"outer parse query string\"))] }")
            )
        );

        let query_string = [
            " ",
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO",
            "X-RELATED-TO:PARENT_UUID",
            "X-LIMIT:50",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST",
            "   ",
        ]
        .join(" ");

        assert_eq!(
            Query::try_from(query_string.as_str()),
            Ok(Query {
                where_conditional: Some(WhereConditional::Operator(
                    Box::new(WhereConditional::Group(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Categories(String::from("CATEGORY_ONE"),),
                                None,
                            )),
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Categories(String::from("CATEGORY_TWO"),),
                                None,
                            )),
                            WhereOperator::Or,
                            None,
                        )),
                        None,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("PARENT_UUID"),
                        )),
                        None,
                    )),
                    WhereOperator::And,
                    None,
                )),

                ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                    long: 2.36885,
                    lat: 48.85299,
                },),

                lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(
                    RangeConditionProperty::DtStart(875779200,),
                    Some(String::from("Event_UUID"),),
                ),),

                upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(
                    RangeConditionProperty::DtStart(878461200,),
                ),),

                in_timezone: rrule::Tz::Europe__Vilnius,

                limit: 50,
            })
        );
    }
}
