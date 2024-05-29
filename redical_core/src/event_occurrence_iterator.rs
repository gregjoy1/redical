use crate::{EventOccurrenceOverride, IndexedConclusion, ScheduleProperties};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub enum FilterProperty {
    DtStart(i64),
    DtEnd(i64),
}

#[derive(Debug, Clone)]
pub enum LowerBoundFilterCondition {
    GreaterThan(FilterProperty),
    GreaterEqualThan(FilterProperty),
}

impl LowerBoundFilterCondition {
    pub fn is_dtstart_filter_property(&self) -> bool {
        matches!(
            self,
            LowerBoundFilterCondition::GreaterThan(FilterProperty::DtStart(_))
                | LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtStart(_))
        )
    }

    pub fn is_dtend_filter_property(&self) -> bool {
        matches!(
            self,
            LowerBoundFilterCondition::GreaterThan(FilterProperty::DtEnd(_))
                | LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtEnd(_))
        )
    }
}

#[derive(Debug, Clone)]
pub enum UpperBoundFilterCondition {
    LessThan(FilterProperty),
    LessEqualThan(FilterProperty),
}

impl UpperBoundFilterCondition {
    pub fn is_dtstart_filter_property(&self) -> bool {
        matches!(
            self,
            UpperBoundFilterCondition::LessThan(FilterProperty::DtStart(_))
                | UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtStart(_))
        )
    }

    pub fn is_dtend_filter_property(&self) -> bool {
        matches!(
            self,
            UpperBoundFilterCondition::LessThan(FilterProperty::DtEnd(_))
                | UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtEnd(_))
        )
    }
}

#[derive(Debug)]
pub struct EventOccurrenceIterator<'a> {
    event_occurrence_overrides: BTreeMap<i64, EventOccurrenceOverride>,
    rrule_set_iter: Option<rrule::RRuleSetIter<'a>>,
    base_duration: i64,
    limit: Option<usize>,
    count: usize,
    is_ended: bool,
    filter_from: Option<LowerBoundFilterCondition>,
    filter_until: Option<UpperBoundFilterCondition>,
    filtering_indexed_conclusion: Option<IndexedConclusion>,
    internal_min_max_bounds: Option<(i64, i64)>,
}

impl<'a> EventOccurrenceIterator<'a> {
    pub fn new(
        schedule_properties: &'a ScheduleProperties,
        event_occurrence_overrides: &'a BTreeMap<i64, EventOccurrenceOverride>,
        limit: Option<usize>,
        filter_from: Option<LowerBoundFilterCondition>,
        filter_until: Option<UpperBoundFilterCondition>,
        filtering_indexed_conclusion: Option<IndexedConclusion>,
    ) -> Result<EventOccurrenceIterator<'a>, String> {
        let rrule_set_iter = match &schedule_properties.parsed_rrule_set {
            Some(parsed_rrule_set) => Some(parsed_rrule_set.into_iter()),
            None => None,
        };

        let base_duration = schedule_properties.get_duration_in_seconds().unwrap_or(0);

        let count = 0usize;
        let is_ended = false;

        let internal_min_max_bounds =
            filtering_indexed_conclusion
                .as_ref()
                .and_then(|indexed_conclusion| {
                    if matches!(indexed_conclusion, IndexedConclusion::Exclude(_)) {
                        indexed_conclusion.min_max_exceptions()
                    } else {
                        None
                    }
                });

        Ok(EventOccurrenceIterator {
            event_occurrence_overrides: event_occurrence_overrides.clone(),
            rrule_set_iter,
            base_duration,
            limit,
            count,
            is_ended,
            filter_from,
            filter_until,
            filtering_indexed_conclusion,
            internal_min_max_bounds,
        })
    }

    fn is_within_limit(&self) -> bool {
        self.limit.is_none() || matches!(self.limit, Some(limit) if limit > self.count)
    }

    fn is_greater_than_filtered_lower_bounds(
        &self,
        dtstart_timestamp: &i64,
        duration: &i64,
    ) -> bool {
        // If filtering_indexed_conclusion is IndexedConclusion::Exclude with exceptions, we work
        // out the min/max bounds and only iterate from the min bounds value.
        if self
            .internal_min_max_bounds
            .is_some_and(|(min, _max)| *dtstart_timestamp < min)
        {
            return false;
        }

        match self.filter_from {
            Some(LowerBoundFilterCondition::GreaterThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp > &comparison
            }

            Some(LowerBoundFilterCondition::GreaterThan(FilterProperty::DtEnd(comparison))) => {
                (dtstart_timestamp > &comparison) || ((dtstart_timestamp + duration) > comparison)
            }

            Some(LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtStart(
                comparison,
            ))) => dtstart_timestamp >= &comparison,

            Some(LowerBoundFilterCondition::GreaterEqualThan(FilterProperty::DtEnd(
                comparison,
            ))) => {
                (dtstart_timestamp >= &comparison) || ((dtstart_timestamp + duration) >= comparison)
            }

            _ => true,
        }
    }

    fn is_less_than_filtered_upper_bounds(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        // If filtering_indexed_conclusion is IndexedConclusion::Exclude with exceptions, we work
        // out the min/max bounds and only iterate until the max bounds value.
        if self
            .internal_min_max_bounds
            .is_some_and(|(_min, max)| *dtstart_timestamp > max)
        {
            return false;
        }

        match self.filter_until {
            Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp < &comparison
            }

            Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtEnd(comparison))) => {
                if dtstart_timestamp > &comparison {
                    false
                } else {
                    (dtstart_timestamp + duration) < comparison
                }
            }

            Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp <= &comparison
            }

            Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtEnd(comparison))) => {
                if dtstart_timestamp > &comparison {
                    false
                } else {
                    (dtstart_timestamp + duration) <= comparison
                }
            }

            _ => true,
        }
    }

    // We rely purely on dtstart_timestamp for this method, to avoid the expense of ascertaining an
    // EventOccurrenceOverride to determine a duration.
    fn has_reached_the_end(&self, dtstart_timestamp: &i64) -> bool {
        // If filtering_indexed_conclusion is IndexedConclusion::Exclude with exceptions, we work
        // out the min/max bounds and only iterate as far as the max value. This prevents this
        // process from iterating infinite recurrences.
        if self
            .internal_min_max_bounds
            .is_some_and(|(_min, max)| *dtstart_timestamp >= max)
        {
            return true;
        }

        match self.filter_until {
            Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp > &comparison
            }

            Some(UpperBoundFilterCondition::LessThan(FilterProperty::DtEnd(comparison))) => {
                // If event starts after filtered DtEnd upper bound, we can assume that we have
                // reached the end.
                dtstart_timestamp > &comparison
            }

            Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp >= &comparison
            }

            Some(UpperBoundFilterCondition::LessEqualThan(FilterProperty::DtEnd(comparison))) => {
                // If event starts after filtered DtEnd upper bound, we can assume that we have
                // reached the end.
                dtstart_timestamp > &comparison
            }

            _ => false,
        }
    }

    fn is_excluded_by_pre_override_enrichment_filters(
        &self,
        dtstart_timestamp: &i64,
        duration: &i64,
    ) -> bool {
        if let Some(filtering_indexed_conclusion) = &self.filtering_indexed_conclusion {
            if filtering_indexed_conclusion.exclude_event_occurrence(dtstart_timestamp.clone()) {
                return true;
            }
        }

        if let Some(filter_condition) = &self.filter_from {
            if filter_condition.is_dtstart_filter_property()
                && !self.is_greater_than_filtered_lower_bounds(dtstart_timestamp, duration)
            {
                return true;
            }
        }

        if let Some(filter_condition) = &self.filter_until {
            if filter_condition.is_dtstart_filter_property()
                && !self.is_less_than_filtered_upper_bounds(dtstart_timestamp, duration)
            {
                return true;
            }
        }

        false
    }

    fn is_excluded_by_post_override_enrichment_filters(
        &self,
        dtstart_timestamp: &i64,
        duration: &i64,
    ) -> bool {
        if let Some(filter_condition) = &self.filter_from {
            if filter_condition.is_dtend_filter_property()
                && !self.is_greater_than_filtered_lower_bounds(dtstart_timestamp, duration)
            {
                return true;
            }
        }

        if let Some(filter_condition) = &self.filter_until {
            if filter_condition.is_dtend_filter_property()
                && !self.is_less_than_filtered_upper_bounds(dtstart_timestamp, duration)
            {
                return true;
            }
        }

        false
    }

    fn rrule_set_iter_next(&mut self) -> Option<chrono::DateTime<rrule::Tz>> {
        match &mut self.rrule_set_iter {
            Some(rrule_set_iter) => rrule_set_iter.next(),
            None => None,
        }
    }
}

impl<'a> Iterator for EventOccurrenceIterator<'a> {
    type Item = (i64, i64, Option<EventOccurrenceOverride>);
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_ended {
            return None;
        }

        while self.is_within_limit() {
            if let Some(dtstart) = self.rrule_set_iter_next() {
                let dtstart_timestamp = dtstart.timestamp();
                let mut duration = self.base_duration;

                if self
                    .is_excluded_by_pre_override_enrichment_filters(&dtstart_timestamp, &duration)
                {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        break;
                    } else {
                        continue;
                    }
                }

                let event_occurrenece_override =
                    self.event_occurrence_overrides.get(&dtstart_timestamp);

                if let Some(event_occurrenece_override) = event_occurrenece_override {
                    duration = match event_occurrenece_override.get_duration_in_seconds() {
                        Some(duration) => duration,
                        _ => self.base_duration,
                    };
                }

                if self
                    .is_excluded_by_post_override_enrichment_filters(&dtstart_timestamp, &duration)
                {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        return None;
                    } else {
                        continue;
                    }
                }

                self.count += 1;

                return Some((
                    dtstart_timestamp,
                    dtstart_timestamp + duration,
                    event_occurrenece_override.cloned(),
                ));
            } else {
                self.is_ended = true;

                break;
            }
        }

        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::str::FromStr;

    use redical_ical::properties::{DTEndProperty, DTStartProperty, RRuleProperty};

    use crate::testing::macros::build_property_from_ical;

    use std::collections::{BTreeMap, HashSet};

    use crate::event::{IndexedProperties, PassiveProperties};

    use redical_ical::properties::LastModifiedProperty;

    use pretty_assertions_sorted::assert_eq;

    fn build_schedule_properties() -> ScheduleProperties {
        let mut schedule_properties = ScheduleProperties {
            rrule: Some(build_property_from_ical!(
                RRuleProperty,
                "RRULE:FREQ=SECONDLY;COUNT=10;INTERVAL=100"
            )),
            exrule: None,
            rdates: None,
            exdates: None,
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T000000Z"
            )),
            dtend: Some(build_property_from_ical!(
                DTEndProperty,
                "DTEND:19700101T000005Z"
            )),
            parsed_rrule_set: None,
        };

        assert!(schedule_properties.build_parsed_rrule_set().is_ok());

        schedule_properties
    }

    fn build_event_occurrence_override_300() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T000500Z"
            )),
            dtend: None,
        }
    }

    fn build_event_occurrence_override_500() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T000820Z"
            )),
            dtend: Some(build_property_from_ical!(
                DTEndProperty,
                "DTEND:19700101T000830Z"
            )),
        }
    }

    fn build_event_occurrence_override_700() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T001140Z"
            )),
            dtend: None,
        }
    }

    fn build_event_occurrence_override_900() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                categories: None,
                related_to: None,
            },
            passive_properties: PassiveProperties::new(),
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T001500Z"
            )),
            dtend: Some(build_property_from_ical!(
                DTEndProperty,
                "DTEND:19700101T001515Z"
            )),
        }
    }

    fn build_event_occurrence_overrides() -> BTreeMap<i64, EventOccurrenceOverride> {
        BTreeMap::from([
            (300, build_event_occurrence_override_300()),
            (500, build_event_occurrence_override_500()),
            (700, build_event_occurrence_override_700()),
            (900, build_event_occurrence_override_900()),
        ])
    }

    // This aims to achieve the following:
    //
    // (0,    OccurrenceCacheValue::Occurrence),
    // (100,  OccurrenceCacheValue::Occurrence),
    // (200,  OccurrenceCacheValue::Occurrence),
    // (300,  OccurrenceCacheValue::Override(None)),
    // (400,  OccurrenceCacheValue::Occurrence),
    // (500,  OccurrenceCacheValue::Override(Some(10))),
    // (600,  OccurrenceCacheValue::Occurrence),
    // (700,  OccurrenceCacheValue::Override(None)),
    // (800,  OccurrenceCacheValue::Occurrence),
    // (900,  OccurrenceCacheValue::Override(Some(15))),

    #[test]
    fn test_event_occurrence_iterator() {
        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), Some((0, 5, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((100, 105, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((200, 205, None)));

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((300, 305, Some(build_event_occurrence_override_300())))
        );

        assert_eq!(event_occurrence_iterator.next(), Some((400, 405, None)));

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((500, 510, Some(build_event_occurrence_override_500())))
        );

        assert_eq!(event_occurrence_iterator.next(), Some((600, 605, None)));

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((700, 705, Some(build_event_occurrence_override_700())))
        );

        assert_eq!(event_occurrence_iterator.next(), Some((800, 805, None)));

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((900, 915, Some(build_event_occurrence_override_900())))
        );

        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_limit() {
        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            Some(3),
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), Some((0, 5, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((100, 105, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((200, 205, None)));
        assert_eq!(event_occurrence_iterator.next(), None);

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            Some(0),
            None,
            None,
            None,
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gt_dtstart() {
        // Test filters -- greater equal than - DtStart
        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtStart(800),
            )),
            None,
            None,
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), Some((800, 805, None)));

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((900, 915, Some(build_event_occurrence_override_900())))
        );

        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_lte_dtend() {
        // Test filters -- less equal than - DtEnd

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            None,
            Some(UpperBoundFilterCondition::LessEqualThan(
                FilterProperty::DtEnd(210),
            )),
            None,
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), Some((0, 5, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((100, 105, None)));
        assert_eq!(event_occurrence_iterator.next(), Some((200, 205, None)));
        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gte_dtstart_lt_dtend() {
        // Test filters -- greater equal than - DtEnd -- less than - DtStart

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtEnd(302),
            )),
            Some(UpperBoundFilterCondition::LessThan(
                FilterProperty::DtStart(500),
            )),
            None,
        )
        .unwrap();

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((300, 305, Some(build_event_occurrence_override_300())))
        );

        assert_eq!(event_occurrence_iterator.next(), Some((400, 405, None)));
        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gte_dtstart_lt_dtend_indexed_conclusion_include() {
        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Include(None)

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtEnd(302),
            )),
            Some(UpperBoundFilterCondition::LessThan(
                FilterProperty::DtStart(500),
            )),
            Some(IndexedConclusion::Include(None)),
        )
        .unwrap();

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((300, 305, Some(build_event_occurrence_override_300())))
        );

        assert_eq!(event_occurrence_iterator.next(), Some((400, 405, None)));
        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gte_dtstart_lt_dtend_indexed_conclusion_include_exceptions(
    ) {
        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Include(300)

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtEnd(302),
            )),
            Some(UpperBoundFilterCondition::LessThan(
                FilterProperty::DtStart(500),
            )),
            Some(IndexedConclusion::Include(Some(HashSet::from([300])))),
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), Some((400, 405, None)));
        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gte_dtstart_lt_dtend_indexed_conclusion_exclude() {
        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Exclude(None)

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtEnd(302),
            )),
            Some(UpperBoundFilterCondition::LessThan(
                FilterProperty::DtStart(500),
            )),
            Some(IndexedConclusion::Exclude(None)),
        )
        .unwrap();

        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_filters_gte_dtstart_lt_dtend_indexed_conclusion_exclude_exceptions(
    ) {
        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Exclude(300)

        let schedule_properties = build_schedule_properties();
        let event_occurrence_overrides = build_event_occurrence_overrides();

        let mut event_occurrence_iterator = EventOccurrenceIterator::new(
            &schedule_properties,
            &event_occurrence_overrides,
            None,
            Some(LowerBoundFilterCondition::GreaterEqualThan(
                FilterProperty::DtEnd(302),
            )),
            Some(UpperBoundFilterCondition::LessThan(
                FilterProperty::DtStart(500),
            )),
            Some(IndexedConclusion::Exclude(Some(HashSet::from([300])))),
        )
        .unwrap();

        assert_eq!(
            event_occurrence_iterator.next(),
            Some((300, 305, Some(build_event_occurrence_override_300())))
        );

        assert_eq!(event_occurrence_iterator.next(), None);
    }

    #[test]
    fn test_event_occurrence_iterator_handle_expensive_runaway_indexed_conclusion_exclude_exceptions(
    ) {
        let mut schedule_properties = ScheduleProperties {
            rrule: Some(build_property_from_ical!(
                RRuleProperty,
                "RRULE:FREQ=SECONDLY;INTERVAL=100"
            )),
            exrule: None,
            rdates: None,
            exdates: None,
            duration: None,
            dtstart: Some(build_property_from_ical!(
                DTStartProperty,
                "DTSTART:19700101T000000Z"
            )),
            dtend: Some(build_property_from_ical!(
                DTEndProperty,
                "DTEND:19700101T000005Z"
            )),
            parsed_rrule_set: None,
        };

        assert!(schedule_properties.build_parsed_rrule_set().is_ok());

        let event_occurrence_overrides = BTreeMap::new();

        let (done_tx, done_rx) = ::std::sync::mpsc::channel();

        let handle = ::std::thread::Builder::new()
            .spawn(move || {
                let mut event_occurrence_iterator = EventOccurrenceIterator::new(
                    &schedule_properties,
                    &event_occurrence_overrides,
                    None,
                    None,
                    None,
                    Some(IndexedConclusion::Exclude(Some(HashSet::from([300])))),
                )
                .unwrap();

                assert_eq!(event_occurrence_iterator.next(), Some((300, 305, None)));

                assert_eq!(event_occurrence_iterator.next(), None);

                let _ = done_tx.send(());
            })
            .unwrap();

        match done_rx.recv_timeout(std::time::Duration::from_secs(1)) {
            Err(::std::sync::mpsc::RecvTimeoutError::Timeout) => {
                panic!("Test took too long");
            }

            _ => {
                if let Err(err) = handle.join() {
                    ::std::panic::resume_unwind(err);
                }
            }
        }
    }
}
