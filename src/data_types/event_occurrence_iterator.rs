use serde::{Serialize, Deserialize};

use std::iter::{Map, Filter};

use std::collections::BTreeMap;

use crate::data_types::{ScheduleProperties, EventOccurrenceOverride, EventOccurrenceOverrides, IndexedConclusion};

#[derive(Debug, Clone)]
pub enum FilterProperty {
    DtStart(i64),
    DtEnd(i64),
}

impl FilterProperty {

    pub fn get_property_value(&self, dtstart_timestamp: &i64, duration: &i64) -> (i64, i64) {
        match self {
            FilterProperty::DtStart(comparison) => (dtstart_timestamp.to_owned(),   comparison.to_owned()),
            FilterProperty::DtEnd(comparison)   => ((dtstart_timestamp + duration), comparison.to_owned()),
        }
    }

}

#[derive(Debug, Clone)]
pub enum FilterCondition {
    LessThan(FilterProperty),
    LessEqualThan(FilterProperty),

    GreaterThan(FilterProperty),
    GreaterEqualThan(FilterProperty),
}

impl FilterCondition {

    pub fn is_filtered(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self {
            FilterCondition::LessThan(filter_property) => {
                let (value, comparison) = filter_property.get_property_value(dtstart_timestamp, duration);

                value < comparison
            },

            FilterCondition::LessEqualThan(filter_property) => {
                let (value, comparison) = filter_property.get_property_value(dtstart_timestamp, duration);

                value <= comparison
            },

            FilterCondition::GreaterThan(filter_property) => {
                let (value, comparison) = filter_property.get_property_value(dtstart_timestamp, duration);

                value > comparison
            },

            FilterCondition::GreaterEqualThan(filter_property) => {
                let (value, comparison) = filter_property.get_property_value(dtstart_timestamp, duration);

                value >= comparison
            },
        }
    }

    pub fn is_dtstart_filter_property(&self) -> bool {
        matches!(
            self,
            FilterCondition::LessThan(FilterProperty::DtStart(_)) |
            FilterCondition::GreaterThan(FilterProperty::DtStart(_)) |
            FilterCondition::LessEqualThan(FilterProperty::DtStart(_)) |
            FilterCondition::GreaterEqualThan(FilterProperty::DtStart(_))
        )
    }

    pub fn is_dtend_filter_property(&self) -> bool {
        matches!(
            self,
            FilterCondition::LessThan(FilterProperty::DtEnd(_)) |
            FilterCondition::GreaterThan(FilterProperty::DtEnd(_)) |
            FilterCondition::LessEqualThan(FilterProperty::DtEnd(_)) |
            FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(_))
        )
    }

}

/*
#[derive(Debug)]
pub struct EventOccurrenceIterator<'a> {
    schedule_properties:          ScheduleProperties,
    event_occurrence_overrides:   EventOccurrenceOverrides,
    rrule_set:                    rrule::RRuleSet,
    rrule_set_iter:               Option<rrule::RRuleSetIter<'a>>,
    base_duration:                i64,
    limit:                        Option<u16>,
    count:                        u16,
    is_ended:                     bool,
    filter_from:                  Option<FilterCondition>,
    filter_until:                 Option<FilterCondition>,
    filtering_indexed_conclusion: Option<IndexedConclusion>,
}

impl<'a> EventOccurrenceIterator<'a> {

    pub fn new(
        schedule_properties:          &'a ScheduleProperties,
        event_occurrence_overrides:   &'a EventOccurrenceOverrides,
        limit:                        Option<u16>,
        filter_from:                  Option<FilterCondition>,
        filter_until:                 Option<FilterCondition>,
        filtering_indexed_conclusion: Option<IndexedConclusion>,
    ) -> Result<EventOccurrenceIterator<'a>, String> {
        let rrule_set =
            schedule_properties.parse_rrule()
                               .map_err(|error| error.to_string())?;

        let rrule_set_iter = None;

        let base_duration =
            schedule_properties.get_duration()
                           .map_err(|error| error.to_string())?
                           .unwrap_or(0);

        let count = 0u16;
        let is_ended = false;

        Ok(
            EventOccurrenceIterator {
                schedule_properties:        schedule_properties.clone(),
                event_occurrence_overrides: event_occurrence_overrides.clone(),
                rrule_set,
                rrule_set_iter,
                base_duration,
                limit,
                count,
                is_ended,
                filter_from,
                filter_until,
                filtering_indexed_conclusion,
            }
        )
    }

    fn is_within_limit(&self) -> bool {
        self.limit.is_none() || matches!(self.limit, Some(limit) if limit > self.count)
    }

    fn is_greater_than_filtered_lower_bounds(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self.filter_from {
            Some(FilterCondition::GreaterThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp > &comparison
            },

            Some(FilterCondition::GreaterThan(FilterProperty::DtEnd(comparison))) => {
                (dtstart_timestamp > &comparison) || ((dtstart_timestamp + duration) > comparison)
            },

            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp >= &comparison
            },

            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(comparison))) => {
                (dtstart_timestamp >= &comparison) || ((dtstart_timestamp + duration) >= comparison)
            },

            _ => true,
        }
    }

    fn is_less_than_filtered_upper_bounds(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        match self.filter_from {
            Some(FilterCondition::LessThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp < &comparison
            },

            Some(FilterCondition::LessThan(FilterProperty::DtEnd(comparison))) => {
                if dtstart_timestamp > &comparison {
                    false
                } else {
                    (dtstart_timestamp + duration) < comparison
                }
            },

            Some(FilterCondition::LessEqualThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp <= &comparison
            },

            Some(FilterCondition::LessEqualThan(FilterProperty::DtEnd(comparison))) => {
                if dtstart_timestamp > &comparison {
                    false
                } else {
                    (dtstart_timestamp + duration) <= comparison
                }
            },

            _ => true,
        }
    }

    // We rely purely on dtstart_timestamp for this method, to avoid the expense of ascertaining an
    // EventOccurrenceOverride to determine a duration.
    fn has_reached_the_end(&self, dtstart_timestamp: &i64) -> bool {
        match self.filter_from {
            Some(FilterCondition::LessThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp > &comparison
            },

            Some(FilterCondition::LessThan(FilterProperty::DtEnd(comparison))) => {
                // If event starts after filtered DtEnd upper bound, we can assume that we have
                // reached the end.
                dtstart_timestamp > &comparison
            },

            Some(FilterCondition::LessEqualThan(FilterProperty::DtStart(comparison))) => {
                dtstart_timestamp >= &comparison
            },

            Some(FilterCondition::LessEqualThan(FilterProperty::DtEnd(comparison))) => {
                // If event starts after filtered DtEnd upper bound, we can assume that we have
                // reached the end.
                dtstart_timestamp > &comparison
            },

            _ => false,
        }
    }

    fn is_excluded_by_pre_override_enrichment_filters(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        if let Some(filter_condition) = &self.filter_from {
            if filter_condition.is_dtstart_filter_property() && !self.is_greater_than_filtered_lower_bounds(dtstart_timestamp, duration) {
                return true;
            }
        }

        if let Some(filter_condition) = &self.filter_until {
            if filter_condition.is_dtstart_filter_property() && !self.is_less_than_filtered_upper_bounds(dtstart_timestamp, duration) {
                return true;
            }
        }

        false
    }

    fn is_excluded_by_post_override_enrichment_filters(&self, dtstart_timestamp: &i64, duration: &i64) -> bool {
        if let Some(filter_condition) = &self.filter_from {
            if filter_condition.is_dtend_filter_property() && !self.is_greater_than_filtered_lower_bounds(dtstart_timestamp, duration) {
                return true;
            }
        }

        if let Some(filter_condition) = &self.filter_until {
            if filter_condition.is_dtend_filter_property() && !self.is_less_than_filtered_upper_bounds(dtstart_timestamp, duration) {
                return true;
            }
        }

        false
    }

    fn rrule_set_iter_next(&mut self) -> Option<chrono::DateTime<rrule::Tz>> {
        //self.rrule_set_iter.insert(self.rrule_set.into_iter()).next()

        let new_iterator = self.rrule_set.into_iter();

        let mut rrule_set_iter = self.rrule_set_iter.clone().unwrap_or(new_iterator);
        let result = rrule_set_iter.next();

        self.rrule_set_iter = Some(rrule_set_iter);

        result
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

                if self.is_excluded_by_pre_override_enrichment_filters(&dtstart_timestamp, &duration) {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        break;
                    } else {
                        continue;
                    }
                }

                let event_occurrenece_override = self.event_occurrence_overrides.current.get(&dtstart_timestamp);

                if let Some(event_occurrenece_override) = event_occurrenece_override {
                    duration =
                        match event_occurrenece_override.get_duration(&dtstart_timestamp) {
                            Ok(Some(duration)) => duration,

                            _ => self.base_duration,
                        };
                }

                if self.is_excluded_by_post_override_enrichment_filters(&dtstart_timestamp, &duration) {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        return None;
                    }
                }

                return Some(
                    (
                        dtstart_timestamp,
                        dtstart_timestamp + duration,
                        event_occurrenece_override.cloned(),
                    )
                );
            } else {
                self.is_ended = true;

                break;
            }
        }

        None
    }

    /*
    fn next(&mut self) -> Option<Self::Item> {
        if self.is_ended {
            return None;
        }

        while self.is_within_limit() {
            if let Some(dtstart) = self.rrule_set_iter_next() {
                let dtstart_timestamp = dtstart.timestamp();
                let mut duration = self.base_duration;

                if self.is_excluded_by_pre_override_enrichment_filters(&dtstart_timestamp, &duration) {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        break;
                    } else {
                        continue;
                    }
                }

                let event_occurrenece_override = self.event_occurrence_overrides.current.get(&dtstart_timestamp);

                if let Some(event_occurrenece_override) = event_occurrenece_override {
                    duration =
                        match event_occurrenece_override.get_duration(&dtstart_timestamp) {
                            Ok(Some(duration)) => duration,

                            _ => self.base_duration,
                        };
                }

                if self.is_excluded_by_post_override_enrichment_filters(&dtstart_timestamp, &duration) {
                    if self.has_reached_the_end(&dtstart_timestamp) {
                        self.is_ended = true;

                        return None;
                    }
                }

                return Some(
                    (
                        dtstart_timestamp,
                        dtstart_timestamp + duration,
                        event_occurrenece_override.cloned(),
                    )
                );
            } else {
                self.is_ended = true;

                break;
            }
        }

        None
    }
    */
}

/*
#[cfg(test)]
mod test {
    use super::*;

    use std::collections::HashSet;

    #[test]
    fn test_occurrence_cache_iterator() {
        let occurrence_cache = OccurrenceCache {
            base_duration: 5,
            occurrences:   BTreeMap::from([
                (100,  OccurrenceCacheValue::Occurrence),
                (200,  OccurrenceCacheValue::Occurrence),
                (300,  OccurrenceCacheValue::Override(None)),
                (400,  OccurrenceCacheValue::Occurrence),
                (500,  OccurrenceCacheValue::Override(Some(10))),
                (600,  OccurrenceCacheValue::Occurrence),
                (700,  OccurrenceCacheValue::Override(None)),
                (800,  OccurrenceCacheValue::Occurrence),
                (900,  OccurrenceCacheValue::Override(Some(15))),
                (1000, OccurrenceCacheValue::Occurrence),
            ])
        };

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(&occurrence_cache, None, None, None);

        assert_eq!(occurrence_cache_iterator.next(), Some((100,  105,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((200,  205,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((300,  305,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), Some((400,  405,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((500,  510,  OccurrenceCacheValue::Override(Some(10)))));
        assert_eq!(occurrence_cache_iterator.next(), Some((600,  605,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((700,  705,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), Some((800,  805,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((900,  915,  OccurrenceCacheValue::Override(Some(15)))));
        assert_eq!(occurrence_cache_iterator.next(), Some((1000, 1005, OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters -- greater equal than - DtStart

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtStart(900))),
            None,
            None,
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((900,  915,  OccurrenceCacheValue::Override(Some(15)))));
        assert_eq!(occurrence_cache_iterator.next(), Some((1000, 1005, OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters -- less equal than - DtEnd

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            None,
            Some(FilterCondition::LessEqualThan(FilterProperty::DtEnd(210))),
            None,
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((100,  105,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((200,  205,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters -- greater equal than - DtEnd -- less than - DtStart

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
            None,
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((300,  305,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), Some((400,  405,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test impossible filters -- less than - DtStart -- greater equal than - DtEnd

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::LessThan(FilterProperty::DtStart(300))),
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(502))),
            None,
        );

        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Include(None)

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
            Some(IndexedConclusion::Include(None)),
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((300,  305,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), Some((400,  405,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Include(300)

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
            Some(IndexedConclusion::Include(Some(HashSet::from([300])))),
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((400,  405,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Exclude(None)

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
            Some(IndexedConclusion::Exclude(None)),
        );

        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters
        //  -- greater equal than - DtEnd
        //  -- less than - DtStart
        //  -- IndexedConclusion::Exclude(300)

        let mut occurrence_cache_iterator = EventOccurrenceIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
            Some(IndexedConclusion::Exclude(Some(HashSet::from([300])))),
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((300,  305,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), None);

    }
}
*/
*/
