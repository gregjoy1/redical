use serde::{Serialize, Deserialize};

use std::collections::{HashMap, HashSet};

use crate::data_types::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedCalendarIndexTerm {
    pub events: HashMap<String, IndexedConclusion>,
}

impl InvertedCalendarIndexTerm {

    pub fn new() -> Self {
        InvertedCalendarIndexTerm {
            events: HashMap::new()
        }
    }

    pub fn new_with_event(event_uuid: String, indexed_conclusion: IndexedConclusion) -> Self {
        let mut inverted_calendar_index_term = Self::new();

        match indexed_conclusion {
            IndexedConclusion::Include(exceptions) => inverted_calendar_index_term.insert_included_event(event_uuid, exceptions),
            IndexedConclusion::Exclude(exceptions) => inverted_calendar_index_term.insert_excluded_event(event_uuid, exceptions),
        };

        inverted_calendar_index_term
    }

    pub fn merge(inverted_index_term_a: InvertedCalendarIndexTerm, inverted_index_term_b: InvertedCalendarIndexTerm) -> InvertedCalendarIndexTerm {
        let events_a = inverted_index_term_a.events;
        let events_b = inverted_index_term_b.events;

        let mut compound_events = HashMap::<String, IndexedConclusion>::new();

        // TODO:
        //   * Iterate on the smallest events HashMap for efficiency
        //   * clone()/borrowing etc

        for (event_uuid, indexed_conclusion_a) in events_a.iter() {
            if let Some(indexed_conclusion_b) = events_b.get(event_uuid) {
                compound_events.insert(
                    event_uuid.clone(),
                    IndexedConclusion::merge(
                        indexed_conclusion_a,
                        indexed_conclusion_b
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
            Some(indexed_conclusion) => indexed_conclusion.include_event_occurrence(occurrence),
            None => false
        }
    }

    pub fn insert_included_event(&mut self, event_uuid: String, exceptions: Option<HashSet<i64>>) -> Option<IndexedConclusion> {
        self.events.insert(event_uuid, IndexedConclusion::Include(exceptions))
    }

    pub fn insert_excluded_event(&mut self, event_uuid: String, exceptions: Option<HashSet<i64>>) -> Option<IndexedConclusion> {
        self.events.insert(event_uuid, IndexedConclusion::Exclude(exceptions))
    }

    pub fn remove_event(&mut self, event_uuid: String) -> Result<&mut Self, String> {
        self.events.remove_entry(&event_uuid);

        Ok(self)
    }

    pub fn insert_exception(&mut self, event_uuid: String, exception: i64) -> Result<&mut IndexedConclusion, String> {
        match self.events.get_mut(&event_uuid) {
            Some(indexed_conclusion) => {
                indexed_conclusion.insert_exception(exception);

                Ok(indexed_conclusion)
            },
            None => {
                Err(format!("Could not insert exception for non-existent event with UUID: {event_uuid}"))
            }
        }
    }

    pub fn remove_exception(&mut self, event_uuid: String, exception: i64) -> Result<&mut IndexedConclusion, String> {
        match self.events.get_mut(&event_uuid) {
            Some(indexed_conclusion) => {
                indexed_conclusion.remove_exception(exception);

                Ok(indexed_conclusion)
            },
            None => {
                Err(format!("Could not remove exception for non-existent event with UUID: {event_uuid}"))
            }
        }
    }
}

pub trait InvertedIndexListener {
    fn handle_update(&mut self, updated_term: &String, indexed_conclusion: Option<&IndexedConclusion>);
}

// Single layer inverted index (for one event) - indexed term - include/exclude
#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct InvertedEventIndex {
    pub terms: HashMap<String, IndexedConclusion>
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

        for (term, indexed_conclusion) in self.terms.iter() {
            match indexed_conclusion {
                IndexedConclusion::Include(_) => {
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
                *indexed_term = IndexedConclusion::merge(
                    indexed_term,
                    &IndexedConclusion::Include(None)
                );

                inverted_index_listener.handle_update(&term, Some(indexed_term));
            })
            .or_insert_with(|| {
                let indexed_conclusion = IndexedConclusion::Include(None);

                inverted_index_listener.handle_update(&term, Some(&indexed_conclusion));

                indexed_conclusion
            });
    }

    pub fn insert_override(&mut self, timestamp: i64, override_terms_set: &HashSet<String>, inverted_index_listener: &mut dyn InvertedIndexListener) {
        let indexed_terms_set = self.get_currently_indexed_terms();

        // Check for currently indexed terms NOT present in the override, and add them as an exception to
        // IndexedConclusion::Include (include all except timestamp).
        for excluded_term in indexed_terms_set.difference(&override_terms_set) {
            self.terms.get_mut(excluded_term)
                      .and_then(|indexed_term| {
                          indexed_term.insert_exception(timestamp);

                          inverted_index_listener.handle_update(excluded_term, Some(indexed_term));

                          Some(indexed_term)
                      });
        }

        // Check for overridden terms NOT already currently indexed, and add them as an
        // exception to IndexedConclusion::Exclude (exclude all except timestamp).
        for included_term in override_terms_set.difference(&indexed_terms_set) {
            self.terms.entry(included_term.clone())
                      .and_modify(|indexed_term| {
                          indexed_term.insert_exception(timestamp);

                          inverted_index_listener.handle_update(included_term, Some(indexed_term));
                      })
                      .or_insert_with(|| {
                          let indexed_conclusion = IndexedConclusion::Exclude(
                              Some(
                                  HashSet::from([timestamp])
                              )
                          );

                          inverted_index_listener.handle_update(included_term, Some(&indexed_conclusion));

                          indexed_conclusion
                      });
        }
    }

    pub fn remove_override(&mut self, timestamp: i64, inverted_index_listener: &mut dyn InvertedIndexListener) {
        self.terms
            .retain(|removed_term, indexed_conclusion| {
                if indexed_conclusion.remove_exception(timestamp) && indexed_conclusion.is_empty_exclude() {
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

    pub fn insert(&mut self, event_uuid: String, term: String, indexed_conclusion: &IndexedConclusion) -> Result<&mut Self, String> {
        self.terms
            .entry(term)
            .and_modify(|term_events| {
                match indexed_conclusion {
                    IndexedConclusion::Include(exceptions) => term_events.insert_included_event(event_uuid.clone(), exceptions.clone()),
                    IndexedConclusion::Exclude(exceptions) => term_events.insert_excluded_event(event_uuid.clone(), exceptions.clone()),
                };
            })
            .or_insert(InvertedCalendarIndexTerm::new_with_event(event_uuid.clone(), indexed_conclusion.clone()));

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

    //     for (category, indexed_conclusion) in indexed_categories.categories.iter() {
    //         move || {
    //             self.terms.entry(*category).and_modify(|inverted_index_term| {
    //                 inverted_index_term.events.insert(event.uuid, *indexed_conclusion);
    //             }).or_insert(
    //                          InvertedCalendarIndexTerm {
    //                     events: HashMap::from([ (event.uuid, *indexed_conclusion) ])
    //                 }
    //             );
    //         };
    //     }

    //     Ok(self)
    // }

}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum IndexedConclusion {
    Include(Option<HashSet<i64>>),
    Exclude(Option<HashSet<i64>>),
}

impl IndexedConclusion {

    pub fn merge(indexed_conclusion_a: &IndexedConclusion, indexed_conclusion_b: &IndexedConclusion) -> IndexedConclusion {

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

        match (indexed_conclusion_a, indexed_conclusion_b) {
            (
                IndexedConclusion::Include(exceptions_a),
                IndexedConclusion::Include(exceptions_b)
            ) => {
                IndexedConclusion::Include(
                    merge_include_all_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedConclusion::Exclude(exceptions_a),
                IndexedConclusion::Exclude(exceptions_b)
            ) => {
                IndexedConclusion::Exclude(
                    merge_exclude_all_exception_sets(exceptions_a, exceptions_b)
                )
            },

            (
                IndexedConclusion::Include(exceptions_to_exclude),
                IndexedConclusion::Exclude(exceptions_to_include)
            ) | (
                IndexedConclusion::Exclude(exceptions_to_include),
                IndexedConclusion::Include(exceptions_to_exclude)
            ) => {
                IndexedConclusion::Exclude(
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
            IndexedConclusion::Include(_) => false,
            IndexedConclusion::Exclude(overrides) => {
                overrides.is_none()
            }
        }
    }

    pub fn include_event_occurrence(&self, occurrence: i64) -> bool {
        match self {
            IndexedConclusion::Include(_) => !self.contains_exception(occurrence),
            IndexedConclusion::Exclude(_) =>  self.contains_exception(occurrence)
        }
    }

    pub fn contains_exception(&self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => { Self::exception_set_contains(exceptions, exception) },
            IndexedConclusion::Exclude(exceptions) => { Self::exception_set_contains(exceptions, exception) },
        }
    }

    pub fn insert_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => { Self::push_to_exception_set(exceptions, exception) },
            IndexedConclusion::Exclude(exceptions) => { Self::push_to_exception_set(exceptions, exception) },
        }
    }

    pub fn remove_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => { Self::remove_from_exception_set(exceptions, exception) },
            IndexedConclusion::Exclude(exceptions) => { Self::remove_from_exception_set(exceptions, exception) },
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
                            (String::from("event_one"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_two"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_three"), IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_four"),  IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_five"),  IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_six"),   IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                            (String::from("event_seven"), IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_eight"), IndexedConclusion::Exclude(None)),
                            (String::from("event_nine"),  IndexedConclusion::Exclude(None))
                        ])
                },
                InvertedCalendarIndexTerm {
                    events:
                        HashMap::from([
                            (String::from("event_one"),   IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                            (String::from("event_two"),   IndexedConclusion::Include(Some(HashSet::from([200, 300])))),
                            (String::from("event_three"), IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))),
                            (String::from("event_four"),  IndexedConclusion::Exclude(None)),
                            (String::from("event_five"),  IndexedConclusion::Include(None)),
                            (String::from("event_six"),   IndexedConclusion::Exclude(None)),
                            (String::from("event_seven"), IndexedConclusion::Include(None)),
                            (String::from("event_eight"), IndexedConclusion::Include(None)),
                        ]),
                }
            ),
            InvertedCalendarIndexTerm {
                events:
                    HashMap::from([
                        (String::from("event_one"),   IndexedConclusion::Exclude(None)),
                        (String::from("event_two"),   IndexedConclusion::Include(Some(HashSet::from([100, 200, 300])))),
                        (String::from("event_three"), IndexedConclusion::Exclude(Some(HashSet::from([200])))),
                        (String::from("event_four"),  IndexedConclusion::Exclude(None)),
                        (String::from("event_five"),  IndexedConclusion::Include(Some(HashSet::from([100, 200])))),
                        (String::from("event_six"),   IndexedConclusion::Exclude(None)),
                        (String::from("event_seven"), IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))),
                        (String::from("event_eight"), IndexedConclusion::Exclude(None)),
                    ]),
            },
        );
    }

    #[test]
    fn test_indexed_conclusion_merge() {
        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
            ),
            IndexedConclusion::Exclude(None),
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 200, 300])))
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([200])))
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Exclude(None)
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Exclude(None)
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge(
                &IndexedConclusion::Exclude(None),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Exclude(None)
        );

    }

    #[test]
    fn test_indexed_conclusion() {
        let mut included_event = IndexedConclusion::Include(None);
        let mut excluded_event = IndexedConclusion::Exclude(None);

        // Testing exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(100), true);
        assert_eq!(excluded_event.insert_exception(100), true);

        assert_eq!(included_event, IndexedConclusion::Include(Some(HashSet::from([100]))));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(Some(HashSet::from([100]))));

        // Testing multiple exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), true);
        assert_eq!(excluded_event.insert_exception(200), true);

        assert_eq!(included_event, IndexedConclusion::Include(Some(HashSet::from([100, 200]))));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))));

        // Testing inserting existing exception into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), false);
        assert_eq!(excluded_event.insert_exception(200), false);

        assert_eq!(included_event, IndexedConclusion::Include(Some(HashSet::from([100, 200]))));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))));

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

        assert_eq!(included_event, IndexedConclusion::Include(Some(HashSet::from([100]))));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(Some(HashSet::from([100]))));

        assert_eq!(included_event.remove_exception(100), true);
        assert_eq!(excluded_event.remove_exception(100), true);

        assert_eq!(included_event, IndexedConclusion::Include(None));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(None));

        // Testing removing non-existent exception from empty Include and Exclude

        assert_eq!(included_event.remove_exception(100), false);
        assert_eq!(excluded_event.remove_exception(100), false);

        assert_eq!(included_event, IndexedConclusion::Include(None));
        assert_eq!(excluded_event, IndexedConclusion::Exclude(None));

        // Testing checking existence of non-existing exceptions from empty Include and Exclude

        assert_eq!(included_event.contains_exception(100), false);
        assert_eq!(excluded_event.contains_exception(100), false);
    }
}
