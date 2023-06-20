use std::collections::{BTreeMap, btree_map};

#[derive(Debug, PartialEq)]
pub enum OccurrenceIndexValue {
    Occurrence
}

#[derive(Debug, PartialEq)]
pub struct OccurrenceIndex {
    pub base_timestamp: Option<i64>,
    pub timestamp_offsets: BTreeMap<i64, OccurrenceIndexValue>,
}

impl OccurrenceIndex {
    pub fn new() -> OccurrenceIndex {
        OccurrenceIndex {
            base_timestamp: None,
            timestamp_offsets: BTreeMap::new()
        }
    }

    pub fn insert(&mut self, occurrence: i64, value: OccurrenceIndexValue) {
        match self.base_timestamp {
            Some(base_timestamp) => {
                self.timestamp_offsets.insert(occurrence - base_timestamp, value);
            },
            None => {
                self.base_timestamp = Some(occurrence);
                self.timestamp_offsets.insert(0, value);
            }
        }
    }

    pub fn remove(&mut self, occurrence: i64) {
        match self.base_timestamp {
            Some(base_timestamp) => {
                self.timestamp_offsets.remove(&(occurrence - base_timestamp));
            },
            None => {}
        }

        if self.timestamp_offsets.is_empty() {
            self.base_timestamp = None;
        }
    }

    pub fn get(&mut self, occurrence: i64) -> Option<&OccurrenceIndexValue> {
        match self.base_timestamp {
            Some(base_timestamp) => {
                self.timestamp_offsets.get(&(occurrence - base_timestamp))
            },
            None => {
                None
            }
        }
    }

    pub fn iter(&self) -> OccurrenceIndexIter {
        OccurrenceIndexIter {
            base_timestamp: &self.base_timestamp,
            timestamp_offsets_iter: self.timestamp_offsets.iter()
        }
    }
}

#[derive(Debug)]
pub struct OccurrenceIndexIter<'a> {
    pub base_timestamp: &'a Option<i64>,
    pub timestamp_offsets_iter: btree_map::Iter<'a, i64, OccurrenceIndexValue>,
}

impl<'a> Iterator for OccurrenceIndexIter<'a> {
    type Item = (i64, &'a OccurrenceIndexValue);

    fn next(&mut self) -> Option<Self::Item> {
        match self.base_timestamp {
            Some(base_timestamp) => {
                match self.timestamp_offsets_iter.next() {
                    Some((offset_key, value)) => {
                        Some(
                            (
                                offset_key + base_timestamp,
                                value
                            )
                        )
                    },
                    None => None
                }
            },
            None => None
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn test_occurrence_index_new() {
        assert_eq!(
            OccurrenceIndex::new(),
            OccurrenceIndex {
                base_timestamp: None,
                timestamp_offsets: BTreeMap::new(),
            }
        );
    }

    #[test]
    fn test_occurrence_index_insert() {
        let mut occurrence_index = OccurrenceIndex::new();

        assert_eq!(
            OccurrenceIndex::new(),
            OccurrenceIndex {
                base_timestamp: None,
                timestamp_offsets: BTreeMap::new(),
            }
        );

        occurrence_index.insert(1686938560, OccurrenceIndexValue::Occurrence); // Fri Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            0, // Fri Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.insert(1686949960, OccurrenceIndexValue::Occurrence); // Fri Jun 16 2023 21:12:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            0, // Fri Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.insert(1687068620, OccurrenceIndexValue::Occurrence); // Sun Jun 18 2023 06:10:20 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            0, // Fri Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.insert(1686852160, OccurrenceIndexValue::Occurrence); // Thu Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            0, // Fri Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );
    }

    #[test]
    fn test_occurrence_index_get() {
        let mut occurrence_index = OccurrenceIndex {
            base_timestamp: Some(1686938560),
            timestamp_offsets: BTreeMap::from(
                [
                    (
                        -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        0, // Fri Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    )
                ]
            )
        };

        // Testing non-existent value
        assert_eq!(
            occurrence_index.get(1686949900), // Fri Jun 16 2023 21:11:40 GMT+0000
            None
        );

        assert_eq!(
            occurrence_index.get(1686852160), // Thu Jun 16 2023 18:02:40 GMT+0000
            Some(
                &OccurrenceIndexValue::Occurrence
            )
        );

        assert_eq!(
            occurrence_index.get(1687068620), // Sun Jun 18 2023 06:10:20 GMT+0000
            Some(
                &OccurrenceIndexValue::Occurrence
            )
        );

        assert_eq!(
            occurrence_index.get(1686949960), // Fri Jun 16 2023 21:12:40 GMT+0000
            Some(
                &OccurrenceIndexValue::Occurrence
            )
        );
    }

    #[test]
    fn test_occurrence_index_iter() {
        let occurrence_index = OccurrenceIndex {
            base_timestamp: None,
            timestamp_offsets: BTreeMap::new()
        };

        let mut occurrence_index_iter = occurrence_index.iter();

        assert_eq!(
            occurrence_index_iter.next(),
            None
        );

        let occurrence_index = OccurrenceIndex {
            base_timestamp: Some(1686938560),
            timestamp_offsets: BTreeMap::from(
                [
                    (
                        -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        0, // Fri Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    )
                ]
            )
        };

        let mut occurrence_index_iter = occurrence_index.iter();

        assert_eq!(
            occurrence_index_iter.next(),
            Some((1686852160, &OccurrenceIndexValue::Occurrence)) // Thu Jun 16 2023 18:02:40 GMT+0000
        );

        assert_eq!(
            occurrence_index_iter.next(),
            Some((1686938560, &OccurrenceIndexValue::Occurrence)) // Fri Jun 16 2023 18:02:40 GMT+0000
        );

        assert_eq!(
            occurrence_index_iter.next(),
            Some((1686949960, &OccurrenceIndexValue::Occurrence)) // Fri Jun 16 2023 21:12:40 GMT+0000
        );

        assert_eq!(
            occurrence_index_iter.next(),
            Some((1687068620, &OccurrenceIndexValue::Occurrence)) // Sun Jun 18 2023 06:10:20 GMT+0000
        );

        assert_eq!(
            occurrence_index_iter.next(),
            None
        );
    }

    #[test]
    fn test_occurrence_index_remove() {
        let mut occurrence_index = OccurrenceIndex {
            base_timestamp: Some(1686938560),
            timestamp_offsets: BTreeMap::from(
                [
                    (
                        -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        0, // Fri Jun 16 2023 18:02:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    ),
                    (
                        130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                        OccurrenceIndexValue::Occurrence
                    )
                ]
            )
        };

        occurrence_index.remove(1686938560); // Fri Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            11400, // Fri Jun 16 2023 21:12:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.remove(1686949960); // Fri Jun 16 2023 21:12:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        ),
                        (
                            130060, // Sun Jun 18 2023 06:10:20 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.remove(1687068620); // Sun Jun 18 2023 06:10:20 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: Some(1686938560),
                timestamp_offsets: BTreeMap::from(
                    [
                        (
                            -86400, // Thu Jun 16 2023 18:02:40 GMT+0000
                            OccurrenceIndexValue::Occurrence
                        )
                    ]
                )
            }
        );

        occurrence_index.remove(1686852160); // Thu Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: None,
                timestamp_offsets: BTreeMap::new()
            }
        );

        // Testing removing non-existent timestamp
        occurrence_index.remove(1686852160); // Thu Jun 16 2023 18:02:40 GMT+0000

        assert_eq!(
            occurrence_index,
            OccurrenceIndex {
                base_timestamp: None,
                timestamp_offsets: BTreeMap::new()
            }
        );
    }
}
