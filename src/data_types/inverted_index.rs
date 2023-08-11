use serde::{Serialize, Deserialize};

use std::collections::{HashMap, HashSet};

use crate::data_types::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedCalendarIndexTerm {
    pub events: HashMap<String, IndexedEvent>,
}

impl InvertedCalendarIndexTerm {

    pub fn new() -> Self {
        InvertedCalendarIndexTerm {
            events: HashMap::new()
        }
    }

    pub fn new_with_event(event_uuid: String, indexed_event: IndexedEvent) -> Self {
        let mut inverted_calendar_index_term = Self::new();

        match indexed_event {
            IndexedEvent::Include(exceptions) => inverted_calendar_index_term.insert_included_event(event_uuid, exceptions),
            IndexedEvent::Exclude(exceptions) => inverted_calendar_index_term.insert_excluded_event(event_uuid, exceptions),
        };

        inverted_calendar_index_term
    }

    pub fn merge(inverted_index_term_a: InvertedCalendarIndexTerm, inverted_index_term_b: InvertedCalendarIndexTerm) -> InvertedCalendarIndexTerm {
        let events_a = inverted_index_term_a.events;
        let events_b = inverted_index_term_b.events;

        let mut compound_events = HashMap::<String, IndexedEvent>::new();

        // TODO:
        //   * Iterate on the smallest events HashMap for efficiency
        //   * clone()/borrowing etc

        for (event_uuid, indexed_event_a) in events_a.iter() {
            if let Some(indexed_event_b) = events_b.get(event_uuid) {
                compound_events.insert(
                    event_uuid.clone(),
                    IndexedEvent::merge(
                        indexed_event_a,
                        indexed_event_b
                    )
                );
            }
        }

        InvertedCalendarIndexTerm {
            events: compound_events
        }
    }

    pub fn include_event_occurrence(&self, event_uuid: String, occurrence: i64) -> bool {
        match self.events.get(&event_uuid) {
            Some(indexed_event) => indexed_event.include_event_occurrence(occurrence),
            None => false
        }
    }

    pub fn insert_included_event(&mut self, event_uuid: String, exceptions: Option<HashSet<i64>>) -> Option<IndexedEvent> {
        self.events.insert(event_uuid, IndexedEvent::Include(exceptions))
    }

    pub fn insert_excluded_event(&mut self, event_uuid: String, exceptions: Option<HashSet<i64>>) -> Option<IndexedEvent> {
        self.events.insert(event_uuid, IndexedEvent::Exclude(exceptions))
    }

    pub fn remove_event(&mut self, event_uuid: String) -> Result<&mut Self, String> {
        self.events.remove_entry(&event_uuid);

        Ok(self)
    }

    pub fn insert_exception(&mut self, event_uuid: String, exception: i64) -> Result<&mut IndexedEvent, String> {
        match self.events.get_mut(&event_uuid) {
            Some(indexed_event) => {
                indexed_event.insert_exception(exception);

                Ok(indexed_event)
            },
            None => {
                Err(format!("Could not insert exception for non-existent event with UUID: {event_uuid}"))
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
                Err(format!("Could not remove exception for non-existent event with UUID: {event_uuid}"))
            }
        }
    }
}

pub trait InvertedIndexListener {
    fn handle_update(&mut self, updated_term: &String, indexed_event: Option<&IndexedEvent>);
}

// Single layer inverted index (for one event) - indexed term - include/exclude
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedEventIndex {
    pub terms: HashMap<String, IndexedEvent>
}

impl InvertedEventIndex {

    pub fn new_from_event_categories(event: &Event, inverted_index_listener: &mut dyn InvertedIndexListener) -> InvertedEventIndex {
        let mut indexed_categories = InvertedEventIndex {
            terms: HashMap::new()
        };

        if let Some(ref categories) = event.indexed_properties.categories {
            for category in categories.iter() {
                indexed_categories.insert(&category, inverted_index_listener);
            }
        }

        for (timestamp, event_override) in event.overrides.current.iter() {
            if let Some(override_categories_set) = &event_override.categories {
                indexed_categories.insert_override(timestamp, override_categories_set, inverted_index_listener);
            }
        }

        indexed_categories
    }

    fn get_currently_indexed_terms(&self) -> HashSet<String> {
        let mut indexed_terms_set: HashSet<String> = HashSet::new();

        for (term, indexed_event) in self.terms.iter() {
            match indexed_event {
                IndexedEvent::Include(_) => {
                    indexed_terms_set.insert(term.clone());
                },

                _ => {
                    continue;
                }
            }
        }

        indexed_terms_set
    }

    pub fn insert(&mut self, term: &String, inverted_index_listener: &mut dyn InvertedIndexListener) {
        self.terms
            .entry(term.clone())
            .and_modify(|indexed_term| {
                *indexed_term = IndexedEvent::merge(
                    indexed_term,
                    &IndexedEvent::Include(None)
                );

                inverted_index_listener.handle_update(&term, Some(indexed_term));
            })
            .or_insert_with(|| {
                let indexed_event = IndexedEvent::Include(None);

                inverted_index_listener.handle_update(&term, Some(&indexed_event));

                indexed_event
            });
    }

    pub fn insert_override(&mut self, timestamp: i64, override_terms_set: &HashSet<String>, inverted_index_listener: &mut dyn InvertedIndexListener) {
        let indexed_terms_set = self.get_currently_indexed_terms();

        // Check for currently indexed terms NOT present in the override, and add them as an exception to
        // IndexedEvent::Include (include all except timestamp).
        for excluded_term in indexed_terms_set.difference(&override_terms_set) {
            self.terms.get_mut(excluded_term)
                      .and_then(|indexed_term| {
                          indexed_term.insert_exception(timestamp);

                          inverted_index_listener.handle_update(excluded_term, Some(indexed_term));

                          Some(indexed_term)
                      });
        }

        // Check for overridden terms NOT already currently indexed, and add them as an
        // exception to IndexedEvent::Exclude (exclude all except timestamp).
        for included_term in override_terms_set.difference(&indexed_terms_set) {
            self.terms.entry(included_term.clone())
                      .and_modify(|indexed_term| {
                          indexed_term.insert_exception(timestamp);

                          inverted_index_listener.handle_update(included_term, Some(indexed_term));
                      })
                      .or_insert_with(|| {
                          let indexed_event = IndexedEvent::Exclude(
                              Some(
                                  HashSet::from([timestamp])
                              )
                          );

                          inverted_index_listener.handle_update(included_term, Some(&indexed_event));

                          indexed_event
                      });
        }
    }

    pub fn remove_override(&mut self, timestamp: i64, inverted_index_listener: &mut dyn InvertedIndexListener) {
        self.terms
            .retain(|removed_term, indexed_event| {
                if indexed_event.remove_exception(timestamp) && indexed_event.is_empty_exclude() {
                    inverted_index_listener.handle_update(removed_term, None);

                    false
                } else {
                    true
                }
            });
    }

}

// Multi layer inverted index (for multiple events) - indexed term - event - include/exclude
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedCalendarIndex {
    pub terms: HashMap<String, InvertedCalendarIndexTerm>,
}

impl InvertedCalendarIndex {

    pub fn new() -> Self {
        InvertedCalendarIndex {
            terms: HashMap::new()
        }
    }

    pub fn insert(&mut self, event_uuid: String, term: String, indexed_event: &IndexedEvent) -> Result<&mut Self, String> {
        self.terms
            .entry(term)
            .and_modify(|term_events| {
                match indexed_event {
                    IndexedEvent::Include(exceptions) => term_events.insert_included_event(event_uuid.clone(), exceptions.clone()),
                    IndexedEvent::Exclude(exceptions) => term_events.insert_excluded_event(event_uuid.clone(), exceptions.clone()),
                };
            })
            .or_insert(InvertedCalendarIndexTerm::new_with_event(event_uuid.clone(), indexed_event.clone()));

        Ok(self)
    }

    pub fn remove(&mut self, event_uuid: String, term: String) -> Result<&mut Self, String> {
        self.terms
            .entry(term)
            .and_modify(|inverted_calendar_index_term| {
                inverted_calendar_index_term.remove_event(event_uuid);
            });

        Ok(self)
    }

    // pub fn insert_all_event(&mut self, event: Event) -> Result<&mut Self, String> {
    //     if event.indexed_categories.is_none() {
    //         return Ok(self);
    //     }

    //     let Some(indexed_categories) = event.indexed_categories;

    //     for (category, indexed_event) in indexed_categories.categories.iter() {
    //         move || {
    //             self.terms.entry(*category).and_modify(|inverted_index_term| {
    //                 inverted_index_term.events.insert(event.uuid, *indexed_event);
    //             }).or_insert(
    //                          InvertedCalendarIndexTerm {
    //                     events: HashMap::from([ (event.uuid, *indexed_event) ])
    //                 }
    //             );
    //         };
    //     }

    //     Ok(self)
    // }

}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IndexedEvent {
    Include(Option<HashSet<i64>>),
    Exclude(Option<HashSet<i64>>),
}

impl IndexedEvent {

    pub fn merge(indexed_event_a: &IndexedEvent, indexed_event_b: &IndexedEvent) -> IndexedEvent {

        // Merging exceptions for same types (e.g. Include & Include, Exclude & Exclude).
        fn merge_include_all_exception_sets(exceptions_a: &Option<HashSet<i64>>, exceptions_b: &Option<HashSet<i64>>) -> Option<HashSet<i64>> {
            let exception_set_a = exceptions_a.clone().unwrap_or(HashSet::new());
            let exception_set_b = exceptions_b.clone().unwrap_or(HashSet::new());

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

        // Merging exceptions for differing types:
        //  (e.g. (Include all - overrides) & (Exclude - overrides))
        fn merge_exclude_all_exception_sets(exceptions_to_include_a: &Option<HashSet<i64>>, exceptions_to_include_b: &Option<HashSet<i64>>) -> Option<HashSet<i64>> {
            let exception_set_to_include_a = exceptions_to_include_a.clone().unwrap_or(HashSet::new());
            let exception_set_to_include_b = exceptions_to_include_b.clone().unwrap_or(HashSet::new());

            // Take all exceptions to include and subtract all exceptions to exclude from it.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    exclude all except  [ 5, 8 ]
            let compound_exception_set: HashSet<i64> =
                exception_set_to_include_a.intersection(&exception_set_to_include_b)
                                          .map(|element| *element)
                                          .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        // Merging exceptions for differing types:
        //  (e.g. (Include all - overrides) & (Exclude - overrides))
        fn merge_unaligned_exception_sets(exceptions_to_include: &Option<HashSet<i64>>, exceptions_to_exclude: &Option<HashSet<i64>>) -> Option<HashSet<i64>> {
            let exception_set_to_include = exceptions_to_include.clone().unwrap_or(HashSet::new());
            let exception_set_to_exclude = exceptions_to_exclude.clone().unwrap_or(HashSet::new());

            // Take all exceptions to include and subtract all exceptions to exclude from it.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    exclude all except  [ 5, 8 ]
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
                    merge_include_all_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedEvent::Exclude(exceptions_a),
                IndexedEvent::Exclude(exceptions_b)
            ) => {
                IndexedEvent::Exclude(
                    merge_exclude_all_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedEvent::Include(exceptions_to_exclude),
                IndexedEvent::Exclude(exceptions_to_include)
            ) | (
                IndexedEvent::Exclude(exceptions_to_include),
                IndexedEvent::Include(exceptions_to_exclude)
            ) => {
                IndexedEvent::Exclude(
                    merge_unaligned_exception_sets(
                        exceptions_to_include,
                        exceptions_to_exclude
                    )
                )
            },
        }
    }

    pub fn is_empty_exclude(&self) -> bool {
        match self {
            IndexedEvent::Include(_) => false,
            IndexedEvent::Exclude(overrides) => {
                overrides.is_none()
            }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_inverted_index_term_merge() {

        assert_eq!(
            InvertedCalendarIndexTerm::merge(
                InvertedCalendarIndexTerm {
                    events:
                        HashMap::from([
                            (String::from("event_one"),   IndexedEvent::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_two"),   IndexedEvent::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_three"), IndexedEvent::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_four"),  IndexedEvent::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_five"),  IndexedEvent::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_six"),   IndexedEvent::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_seven"), IndexedEvent::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_eight"), IndexedEvent::Exclude(None)),
                            (String::from("event_nine"),  IndexedEvent::Exclude(None))
                        ])
                },
                InvertedCalendarIndexTerm {
                    events:
                        HashMap::from([
                            (String::from("event_one"),   IndexedEvent::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_two"),   IndexedEvent::Include(Some(HashSet::from([200, 300])))),
                            (String::from("event_three"), IndexedEvent::Exclude(Some(HashSet::from([200, 300])))),
                            (String::from("event_four"),  IndexedEvent::Exclude(None)),
                            (String::from("event_five"),  IndexedEvent::Include(None)),
                            (String::from("event_six"),   IndexedEvent::Exclude(None)),
                            (String::from("event_seven"), IndexedEvent::Include(None)),
                            (String::from("event_eight"), IndexedEvent::Include(None)),
                        ]),
                }
            ),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("event_one"),   IndexedEvent::Exclude(None)),
                        (String::from("event_two"),   IndexedEvent::Include(Some(HashSet::from([100, 200, 300])))),
                        (String::from("event_three"), IndexedEvent::Exclude(Some(HashSet::from([200])))),
                        (String::from("event_four"),  IndexedEvent::Exclude(None)),
                        (String::from("event_five"),  IndexedEvent::Include(Some(HashSet::from([100, 200])))),
                        (String::from("event_six"),   IndexedEvent::Exclude(None)),
                        (String::from("event_seven"), IndexedEvent::Exclude(Some(HashSet::from([100, 200])))),
                        (String::from("event_eight"), IndexedEvent::Exclude(None)),
                    ]),
            },
        );
    }

    #[test]
    fn test_indexed_event_merge() {
        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Include(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Exclude(Some(HashSet::from([100, 200])))
            ),
            IndexedEvent::Exclude(None),
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Include(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Include(Some(HashSet::from([200, 300])))
            ),
            IndexedEvent::Include(Some(HashSet::from([100, 200, 300])))
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Exclude(Some(HashSet::from([200, 300])))
            ),
            IndexedEvent::Exclude(Some(HashSet::from([200])))
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Exclude(None)
            ),
            IndexedEvent::Exclude(None)
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Include(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Include(None)
            ),
            IndexedEvent::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Include(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Exclude(None)
            ),
            IndexedEvent::Exclude(None)
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedEvent::Include(None)
            ),
            IndexedEvent::Exclude(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedEvent::merge(
                &IndexedEvent::Exclude(None),
                &IndexedEvent::Include(None)
            ),
            IndexedEvent::Exclude(None)
        );

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
