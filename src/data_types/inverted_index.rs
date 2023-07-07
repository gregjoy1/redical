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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedIndex {
    pub terms: HashMap<String, InvertedIndexTerm>,
}

impl IndexedEvent {

    pub fn contains_exception(&mut self, exception: i64) -> bool {
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

    fn exception_set_contains(exceptions: &mut Option<HashSet<i64>>, exception: i64) -> bool {
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
