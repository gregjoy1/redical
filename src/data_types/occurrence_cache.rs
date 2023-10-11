use serde::{Serialize, Deserialize};

use std::iter::{Map, Filter};

use std::collections::{BTreeMap, btree_map};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum OccurrenceCacheValue {
    Occurrence,
    Override(Option<i64>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct OccurrenceCache {
    pub base_duration:  i64,
    pub occurrences:    BTreeMap<i64, OccurrenceCacheValue>,
}

impl OccurrenceCache {
    pub fn new(base_duration: Option<i64>) -> OccurrenceCache {
        OccurrenceCache {
            base_duration: base_duration.unwrap_or(0),
            occurrences:   BTreeMap::new(),
        }
    }
}

type OccurrenceCacheIteratorMapFn    = Box<dyn Fn((&i64, &OccurrenceCacheValue)) -> (i64, i64, OccurrenceCacheValue)>;
type OccurrenceCacheIteratorFilterFn = Box<dyn Fn(&(i64, i64, OccurrenceCacheValue)) -> bool>;

#[derive(Debug, Clone)]
pub enum FilterProperty {
    DtStart(i64),
    DtEnd(i64),
}

impl FilterProperty {

    pub fn get_property_value(&self, iterator_item: &(i64, i64, OccurrenceCacheValue)) -> (i64, i64) {
        match self {
            FilterProperty::DtStart(comparison) => (iterator_item.0, comparison.to_owned()),
            FilterProperty::DtEnd(comparison)   => (iterator_item.1, comparison.to_owned()),
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

    pub fn filter_iterator_item(&self, iterator_item: &(i64, i64, OccurrenceCacheValue)) -> bool {
        match self {
            FilterCondition::LessThan(filter_property) => {
                let values = filter_property.get_property_value(iterator_item);

                values.0 < values.1
            },

            FilterCondition::LessEqualThan(filter_property) => {
                let values = filter_property.get_property_value(iterator_item);

                values.0 <= values.1
            },

            FilterCondition::GreaterThan(filter_property) => {
                let values = filter_property.get_property_value(iterator_item);

                values.0 > values.1
            },

            FilterCondition::GreaterEqualThan(filter_property) => {
                let values = filter_property.get_property_value(iterator_item);

                values.0 >= values.1
            },
        }
    }

}

#[derive(Debug)]
pub struct OccurrenceCacheIterator<'a> {
    base_duration: i64,
    internal_iter: Filter<Map<btree_map::Iter<'a, i64, OccurrenceCacheValue>, OccurrenceCacheIteratorMapFn>, OccurrenceCacheIteratorFilterFn>,
}

impl<'a> OccurrenceCacheIterator<'a> {

    pub fn new(
        occurrence_cache: &'a OccurrenceCache,
        filter_from:      Option<FilterCondition>,
        filter_until:     Option<FilterCondition>
    ) -> OccurrenceCacheIterator<'a> {
        let base_duration = occurrence_cache.base_duration;

        let internal_iter =
            occurrence_cache.occurrences
                            .iter()
                            .map(Self::build_map_function(base_duration))
                            .filter(Self::build_filter_function(filter_from, filter_until));

        OccurrenceCacheIterator {
            base_duration,
            internal_iter,
        }
    }

    fn build_map_function(base_duration: i64) -> OccurrenceCacheIteratorMapFn {
        Box::new(move |(dtstart_timestamp, value)| {
            let dtend_timestamp = match value {
                OccurrenceCacheValue::Override(Some(overridden_duration)) => dtstart_timestamp + overridden_duration.to_owned(),
                _                                                               => dtstart_timestamp + base_duration,
            };

            (
                dtstart_timestamp.to_owned(),
                dtend_timestamp,
                value.clone(),
            )
        })
    }

    fn build_filter_function(
        filter_from:  Option<FilterCondition>,
        filter_until: Option<FilterCondition>
    ) -> OccurrenceCacheIteratorFilterFn {
        Box::new(move |iterator_item| {
            match (&filter_from, &filter_until) {
                (None, None) => {
                    true
                },

                (Some(filter_from_cond), Some(filter_until_cond)) => {
                    filter_from_cond.filter_iterator_item(iterator_item) && filter_until_cond.filter_iterator_item(iterator_item)
                },

                (Some(filter_from_cond), None) => {
                    filter_from_cond.filter_iterator_item(iterator_item)
                },

                (None, Some(filter_until_cond)) => {
                    filter_until_cond.filter_iterator_item(iterator_item)
                }
            }
        })
    }
}

impl<'a> Iterator for OccurrenceCacheIterator<'a> {
    type Item = (i64, i64, OccurrenceCacheValue);

    fn next(&mut self) -> Option<Self::Item> {
        self.internal_iter.next()
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

        let mut occurrence_cache_iterator = OccurrenceCacheIterator::new(&occurrence_cache, None, None);

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

        let mut occurrence_cache_iterator = OccurrenceCacheIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtStart(900))),
            None
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((900,  915,  OccurrenceCacheValue::Override(Some(15)))));
        assert_eq!(occurrence_cache_iterator.next(), Some((1000, 1005, OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters -- less equal than - DtEnd

        let mut occurrence_cache_iterator = OccurrenceCacheIterator::new(
            &occurrence_cache,
            None,
            Some(FilterCondition::LessEqualThan(FilterProperty::DtEnd(210))),
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((100,  105,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), Some((200,  205,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test filters -- greater equal than - DtEnd -- less than - DtStart

        let mut occurrence_cache_iterator = OccurrenceCacheIterator::new(
            &occurrence_cache,
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(302))),
            Some(FilterCondition::LessThan(FilterProperty::DtStart(500))),
        );

        assert_eq!(occurrence_cache_iterator.next(), Some((300,  305,  OccurrenceCacheValue::Override(None))));
        assert_eq!(occurrence_cache_iterator.next(), Some((400,  405,  OccurrenceCacheValue::Occurrence)));
        assert_eq!(occurrence_cache_iterator.next(), None);

        // Test impossible filters -- less than - DtStart -- greater equal than - DtEnd

        let mut occurrence_cache_iterator = OccurrenceCacheIterator::new(
            &occurrence_cache,
            Some(FilterCondition::LessThan(FilterProperty::DtStart(300))),
            Some(FilterCondition::GreaterEqualThan(FilterProperty::DtEnd(502))),
        );

        assert_eq!(occurrence_cache_iterator.next(), None);
    }
}
