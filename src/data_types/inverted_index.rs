use serde::{Serialize, Deserialize};

use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IndexedEvent {
    Include(Option<HashSet<i64>>),
    Exclude(Option<HashSet<i64>>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedIndexTerm {
    pub events: HashMap<String, IndexedEvent>,
}

impl InvertedIndexTerm {

    pub fn include_event_occurrence(&self, event_uuid: String, occurrence: i64) -> bool {
        match self.events.get(&event_uuid) {
            Some(indexed_event) => indexed_event.include_event_occurrence(occurrence),
            None => false
        }
    }

    pub fn insert_included_event(&mut self, event_uuid: String) -> Option<IndexedEvent> {
        self.events.insert(event_uuid, IndexedEvent::Include(None))
    }

    pub fn insert_excluded_event(&mut self, event_uuid: String) -> Option<IndexedEvent> {
        self.events.insert(event_uuid, IndexedEvent::Exclude(None))
    }

    pub fn insert_exception(&mut self, event_uuid: String, exception: i64) -> Result<&mut IndexedEvent, String> {
        match self.events.get_mut(&event_uuid) {
            Some(indexed_event) => {
                indexed_event.insert_exception(exception);

                Ok(indexed_event)
            },
            None => {
                Err(String::from(format!("Could not insert exception for non-existent event with UUID: {event_uuid}")))
            }
        }
    }

    pub fn remove_exception(&mut self, event_uuid: String, exception: i64) -> Result<&mut IndexedEvent, String> {
        match self.events.get_mut(&event_uuid) {
            Some(indexed_event) => {
                indexed_event.remove_exception(exception);

                Ok(indexed_event)
            },
            None => {
                Err(String::from(format!("Could not remove exception for non-existent event with UUID: {event_uuid}")))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedIndex {
    pub terms: HashMap<String, InvertedIndexTerm>,
}

impl IndexedEvent {

    pub fn merge(indexed_event_a: IndexedEvent, indexed_event_b: IndexedEvent) -> IndexedEvent {

        // Merging exceptions for same types (e.g. Include & Include, Exclude & Exclude).
        fn merge_aligned_exception_sets(exceptions_a: Option<HashSet<i64>>, exceptions_b: Option<HashSet<i64>>) -> Option<HashSet<i64>> {
            let exception_set_a = exceptions_a.unwrap_or(HashSet::new());
            let exception_set_b = exceptions_b.unwrap_or(HashSet::new());

            let compound_exception_set: HashSet<i64> =
                exception_set_a.union(&exception_set_b)
                               .map(|element| *element)
                               .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        // Merging exceptions for differing types (e.g. Include & Exclude).
        fn merge_unaligned_exception_sets(exceptions_to_include: Option<HashSet<i64>>, exceptions_to_exclude: Option<HashSet<i64>>) -> Option<HashSet<i64>> {
            let exception_set_to_include = exceptions_to_include.unwrap_or(HashSet::new());
            let exception_set_to_exclude = exceptions_to_exclude.unwrap_or(HashSet::new());

            // Take all exceptions to include and subtract all exceptions to exclude from it.
            // e.g.
            //  to_include [ 1, 2, 3, 4 ]
            //  to_exclude [ 2, 3, 5, 8 ]
            //  combined   [ 1, 4 ]
            let compound_exception_set: HashSet<i64> =
                exception_set_to_include.difference(&exception_set_to_exclude)
                               .map(|element| *element)
                               .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        match (indexed_event_a, indexed_event_b) {
            (
                IndexedEvent::Include(exceptions_a),
                IndexedEvent::Include(exceptions_b)
            ) => {
                IndexedEvent::Include(
                    merge_aligned_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedEvent::Exclude(exceptions_a),
                IndexedEvent::Exclude(exceptions_b)
            ) => {
                IndexedEvent::Exclude(
                    merge_aligned_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedEvent::Include(exceptions_to_exclude),
                IndexedEvent::Exclude(exceptions_to_include)
            ) | (
                IndexedEvent::Exclude(exceptions_to_include),
                IndexedEvent::Include(exceptions_to_exclude)
            ) => {
                IndexedEvent::Include(
                    merge_unaligned_exception_sets(
                        exceptions_to_include,
                        exceptions_to_exclude
                    )
                )
            },
        }
    }

    pub fn include_event_occurrence(&self, occurrence: i64) -> bool {
        match self {
            IndexedEvent::Include(_) => !self.contains_exception(occurrence),
            IndexedEvent::Exclude(_) =>  self.contains_exception(occurrence)
        }
    }

    pub fn contains_exception(&self, exception: i64) -> bool {
        match self {
            IndexedEvent::Include(exceptions) => { Self::exception_set_contains(exceptions, exception) },
            IndexedEvent::Exclude(exceptions) => { Self::exception_set_contains(exceptions, exception) },
        }
    }

    pub fn insert_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedEvent::Include(exceptions) => { Self::push_to_exception_set(exceptions, exception) },
            IndexedEvent::Exclude(exceptions) => { Self::push_to_exception_set(exceptions, exception) },
        }
    }

    pub fn remove_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedEvent::Include(exceptions) => { Self::remove_from_exception_set(exceptions, exception) },
            IndexedEvent::Exclude(exceptions) => { Self::remove_from_exception_set(exceptions, exception) },
        }
    }

    fn exception_set_contains(exceptions: &Option<HashSet<i64>>, exception: i64) -> bool {
        match exceptions {
            Some(exception_set) => exception_set.contains(&exception),
            None => false
        }
    }

    fn push_to_exception_set(exceptions: &mut Option<HashSet<i64>>, exception: i64) -> bool {
        match exceptions {
            Some(exception_set) => {
                exception_set.insert(exception)
            },
            None => {
                *exceptions = Some(HashSet::from([exception]));

                true
            },
        }
    }

    fn remove_from_exception_set(exceptions: &mut Option<HashSet<i64>>, exception: i64) -> bool {
        match exceptions {
            Some(exception_set) => {
                let was_present = exception_set.remove(&exception);

                if exception_set.is_empty() {
                    *exceptions = None;
                }

                was_present
            },
            None => false
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn test_indexed_event_merge() {
    }

    #[test]
    fn test_indexed_event() {
        let mut included_event = IndexedEvent::Include(None);
        let mut excluded_event = IndexedEvent::Exclude(None);

        // Testing exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(100), true);
        assert_eq!(excluded_event.insert_exception(100), true);

        assert_eq!(included_event, IndexedEvent::Include(Some(HashSet::from([100]))));
        assert_eq!(excluded_event, IndexedEvent::Exclude(Some(HashSet::from([100]))));

        // Testing multiple exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), true);
        assert_eq!(excluded_event.insert_exception(200), true);

        assert_eq!(included_event, IndexedEvent::Include(Some(HashSet::from([100, 200]))));
        assert_eq!(excluded_event, IndexedEvent::Exclude(Some(HashSet::from([100, 200]))));

        // Testing inserting existing exception into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), false);
        assert_eq!(excluded_event.insert_exception(200), false);

        assert_eq!(included_event, IndexedEvent::Include(Some(HashSet::from([100, 200]))));
        assert_eq!(excluded_event, IndexedEvent::Exclude(Some(HashSet::from([100, 200]))));

        // Testing checking existence of both existing and non-existing exceptions from populated Include and Exclude

        assert_eq!(included_event.contains_exception(100), true);
        assert_eq!(included_event.contains_exception(200), true);
        assert_eq!(included_event.contains_exception(300), false);

        assert_eq!(excluded_event.contains_exception(100), true);
        assert_eq!(excluded_event.contains_exception(200), true);
        assert_eq!(excluded_event.contains_exception(300), false);

        // Testing querying inclusion of event occurrence from populated Include and Exclude

        // Exclude occurrences 100, and 200 because they are present as exceptions to an overall event inclusion.
        assert_eq!(included_event.include_event_occurrence(100), false);
        assert_eq!(included_event.include_event_occurrence(200), false);

        // Include occurrence 300 because it is not present as an exception to an overall event inclusion.
        assert_eq!(included_event.include_event_occurrence(300), true);

        // Include occurrences 100, and 200 because they are present as exceptions to an overall event exclusion.
        assert_eq!(excluded_event.include_event_occurrence(100), true);
        assert_eq!(excluded_event.include_event_occurrence(200), true);

        // Exclude occurrence 300 because it is not present as an exception to an overall event exclusion.
        assert_eq!(excluded_event.include_event_occurrence(300), false);

        // Testing removing non-existent exception from populated Include and Exclude

        assert_eq!(included_event.remove_exception(300), false);
        assert_eq!(excluded_event.remove_exception(300), false);

        // Testing removing existing exception from populated Include and Exclude

        assert_eq!(included_event.remove_exception(200), true);
        assert_eq!(excluded_event.remove_exception(200), true);

        assert_eq!(included_event, IndexedEvent::Include(Some(HashSet::from([100]))));
        assert_eq!(excluded_event, IndexedEvent::Exclude(Some(HashSet::from([100]))));

        assert_eq!(included_event.remove_exception(100), true);
        assert_eq!(excluded_event.remove_exception(100), true);

        assert_eq!(included_event, IndexedEvent::Include(None));
        assert_eq!(excluded_event, IndexedEvent::Exclude(None));

        // Testing removing non-existent exception from empty Include and Exclude

        assert_eq!(included_event.remove_exception(100), false);
        assert_eq!(excluded_event.remove_exception(100), false);

        assert_eq!(included_event, IndexedEvent::Include(None));
        assert_eq!(excluded_event, IndexedEvent::Exclude(None));

        // Testing checking existence of non-existing exceptions from empty Include and Exclude

        assert_eq!(included_event.contains_exception(100), false);
        assert_eq!(excluded_event.contains_exception(100), false);
    }
}
