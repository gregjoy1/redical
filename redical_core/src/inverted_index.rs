use std::collections::{HashMap, HashSet};

use crate::event::Event;
use crate::geo_index::GeoPoint;

use crate::utils::{KeyValuePair, UpdatedHashMapMembers, UpdatedSetMembers};

use redical_ical::properties::ICalendarGeoProperty;

#[derive(Default, Debug, PartialEq, Eq, Clone)]
pub struct InvertedCalendarIndexTerm {
    pub events: HashMap<String, IndexedConclusion>,
}

impl InvertedCalendarIndexTerm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_event(event_uid: String, indexed_conclusion: IndexedConclusion) -> Self {
        Self::new_with_events(
            vec![
                (event_uid, indexed_conclusion),
            ]
        )
    }

    pub fn new_with_events(event_uid_index_conclusion_pairs: Vec<(String, IndexedConclusion)>) -> Self {
        let mut inverted_calendar_index_term = Self::new();

        for (event_uid, indexed_conclusion) in event_uid_index_conclusion_pairs {
            match indexed_conclusion {
                IndexedConclusion::Include(exceptions) => {
                    inverted_calendar_index_term.insert_included_event(event_uid, exceptions)
                }
                IndexedConclusion::Exclude(exceptions) => {
                    inverted_calendar_index_term.insert_excluded_event(event_uid, exceptions)
                }
            };
        }

        inverted_calendar_index_term
    }

    pub fn merge_and(
        inverted_index_term_a: &InvertedCalendarIndexTerm,
        inverted_index_term_b: &InvertedCalendarIndexTerm,
    ) -> InvertedCalendarIndexTerm {
        let events_a = &inverted_index_term_a.events;
        let events_b = &inverted_index_term_b.events;

        let mut compound_events = HashMap::<String, IndexedConclusion>::new();

        // TODO:
        //   * Iterate on the smallest events HashMap for efficiency
        //   * clone()/borrowing etc

        for (event_uid, indexed_conclusion_a) in events_a.iter() {
            if let Some(indexed_conclusion_b) = events_b.get(event_uid) {
                compound_events.insert(
                    event_uid.clone(),
                    IndexedConclusion::merge_and(indexed_conclusion_a, indexed_conclusion_b),
                );
            }
        }

        InvertedCalendarIndexTerm {
            events: compound_events,
        }
    }

    pub fn merge_or(
        inverted_index_term_a: &InvertedCalendarIndexTerm,
        inverted_index_term_b: &InvertedCalendarIndexTerm,
    ) -> InvertedCalendarIndexTerm {
        let events_a = &inverted_index_term_a.events;
        let events_b = &inverted_index_term_b.events;

        let mut compound_events = HashMap::<String, IndexedConclusion>::new();

        // TODO:
        //   * clone()/borrowing etc
        //   * refine this logic to be more concise/readable...

        let events_a_uids = HashSet::<String>::from_iter(events_a.keys().cloned());
        let events_b_uids = HashSet::<String>::from_iter(events_b.keys().cloned());

        let uid_key_diff = UpdatedSetMembers::new(Some(&events_a_uids), Some(&events_b_uids));

        for events_a_exclusive_uid in uid_key_diff.removed.iter() {
            if let Some(indexed_conclusion) = events_a.get(events_a_exclusive_uid) {
                if indexed_conclusion.is_empty_exclude() {
                    continue;
                }

                compound_events.insert(events_a_exclusive_uid.clone(), indexed_conclusion.clone());
            }
        }

        for events_b_exclusive_uid in uid_key_diff.added.iter() {
            if let Some(indexed_conclusion) = events_b.get(events_b_exclusive_uid) {
                if indexed_conclusion.is_empty_exclude() {
                    continue;
                }

                compound_events.insert(events_b_exclusive_uid.clone(), indexed_conclusion.clone());
            }
        }

        for event_uid in uid_key_diff.maintained.iter() {
            let indexed_conclusion_a = events_a
                .get(event_uid)
                .expect("Expected events a to contain a present IndexedConclusion.");
            let indexed_conclusion_b = events_b
                .get(event_uid)
                .expect("Expected events b to contain a present IndexedConclusion.");

            compound_events.insert(
                event_uid.clone(),
                IndexedConclusion::merge_or(indexed_conclusion_a, indexed_conclusion_b),
            );
        }

        InvertedCalendarIndexTerm {
            events: compound_events,
        }
    }

    pub fn inverse(&self) -> Self {
        let inverted_events = self.events.iter()
            .map(|(uid, indexed_conclusion)| (uid.clone(), indexed_conclusion.negate()))
            .collect();

        Self::new_with_events(inverted_events)
    }

    pub fn include_event_occurrence(&self, event_uid: String, occurrence: i64) -> bool {
        match self.events.get(&event_uid) {
            Some(indexed_conclusion) => indexed_conclusion.include_event_occurrence(occurrence),
            None => false,
        }
    }

    pub fn insert_included_event(
        &mut self,
        event_uid: String,
        exceptions: Option<HashSet<i64>>,
    ) -> Option<IndexedConclusion> {
        self.events
            .insert(event_uid, IndexedConclusion::Include(exceptions))
    }

    pub fn insert_excluded_event(
        &mut self,
        event_uid: String,
        exceptions: Option<HashSet<i64>>,
    ) -> Option<IndexedConclusion> {
        self.events
            .insert(event_uid, IndexedConclusion::Exclude(exceptions))
    }

    pub fn remove_event(&mut self, event_uid: String) -> Result<&mut Self, String> {
        self.events.remove_entry(&event_uid);

        Ok(self)
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn insert_exception(
        &mut self,
        event_uid: String,
        exception: i64,
    ) -> Result<&mut IndexedConclusion, String> {
        match self.events.get_mut(&event_uid) {
            Some(indexed_conclusion) => {
                indexed_conclusion.insert_exception(exception);

                Ok(indexed_conclusion)
            }
            None => Err(format!(
                "Could not insert exception for non-existent event with UID: {event_uid}"
            )),
        }
    }

    pub fn remove_exception(
        &mut self,
        event_uid: String,
        exception: i64,
    ) -> Result<&mut IndexedConclusion, String> {
        match self.events.get_mut(&event_uid) {
            Some(indexed_conclusion) => {
                indexed_conclusion.remove_exception(exception);

                Ok(indexed_conclusion)
            }
            None => Err(format!(
                "Could not remove exception for non-existent event with UID: {event_uid}"
            )),
        }
    }
}

// TODO: Make more generic as this is used into the geo index
// Single layer inverted index (for one event) - indexed term - include/exclude
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct InvertedEventIndex<K>
where
    K: std::hash::Hash + Clone + std::cmp::Eq,
{
    pub terms: HashMap<K, IndexedConclusion>,
}

impl<K> Default for InvertedEventIndex<K>
where
    K: std::hash::Hash + Clone + std::cmp::Eq,
{
    fn default() -> Self {
        InvertedEventIndex {
            terms: HashMap::new(),
        }
    }
}

impl<K> InvertedEventIndex<K>
where
    K: std::hash::Hash + Clone + std::cmp::Eq,
{
    pub fn new_from_event_categories(event: &Event) -> InvertedEventIndex<String> {
        let mut indexed_categories = InvertedEventIndex {
            terms: HashMap::new(),
        };

        if let Some(categories_properties) = event.indexed_properties.categories.as_ref() {
            for categories_property in categories_properties {
                for category in &categories_property.categories {
                    indexed_categories.insert(&category.to_string());
                }
            }
        }

        for (timestamp, event_override) in event.overrides.iter() {
            if let Some(override_categories_set) = &event_override
                .indexed_properties
                .extract_all_category_strings()
            {
                indexed_categories.insert_override(timestamp.to_owned(), override_categories_set);
            }
        }

        indexed_categories
    }

    pub fn new_from_event_location_type(event: &Event) -> InvertedEventIndex<String> {
        let mut indexed_location_type = InvertedEventIndex {
            terms: HashMap::new(),
        };

        if let Some(location_type_property) = event.indexed_properties.location_type.as_ref() {
            for location_type in &location_type_property.types {
                indexed_location_type.insert(&location_type.to_string());
            }
        }

        for (timestamp, event_override) in event.overrides.iter() {
            if let Some(override_location_type_set) = &event_override
                .indexed_properties
                .extract_all_location_type_strings()
            {
                indexed_location_type.insert_override(timestamp.to_owned(), override_location_type_set);
            }
        }

        indexed_location_type
    }

    pub fn new_from_event_related_to(event: &Event) -> InvertedEventIndex<KeyValuePair> {
        let mut indexed_related_to = InvertedEventIndex {
            terms: HashMap::new(),
        };

        if let Some(related_to_properties) = event.indexed_properties.related_to.as_ref() {
            for related_to_property in related_to_properties {
                indexed_related_to.insert(&related_to_property.to_reltype_uid_pair().into());
            }
        }

        for (timestamp, event_override) in event.overrides.iter() {
            if let Some(override_related_to_set) = &event_override
                .indexed_properties
                .extract_all_related_to_key_value_pairs()
            {
                indexed_related_to.insert_override(timestamp.to_owned(), override_related_to_set);
            }
        }

        indexed_related_to
    }

    // TODO: Add tests...
    pub fn new_from_event_geo(event: &Event) -> InvertedEventIndex<GeoPoint> {
        let mut indexed_geo = InvertedEventIndex {
            terms: HashMap::new(),
        };

        if let Some(geo_property) = event.indexed_properties.geo.as_ref() {
            if let Ok(geo_point) = GeoPoint::try_from(geo_property.get_lat_long_pair()) {
                indexed_geo.insert(&geo_point);
            }
        }

        for (timestamp, event_override) in event.overrides.iter() {
            if event_override.indexed_properties.geo.is_none() {
                continue;
            };

            let timestamp = timestamp.to_owned();

            // Allow events with GEO defined to be overridden to make GEO blank (specific events online only).
            if let Some(overridden_geo_point) = &event_override.indexed_properties.extract_geo_point() {
                // If a non-blank GEO property defined override is present, insert this.
                indexed_geo.insert_override(timestamp, &HashSet::from([overridden_geo_point.to_owned()]));
            } else {
                // If a blank GEO property defined override is present, insert the blank.
                indexed_geo.insert_override(timestamp, &HashSet::from([]));
            }
        }

        indexed_geo
    }

    // TODO: Add tests...
    pub fn new_from_event_class(event: &Event) -> InvertedEventIndex<String> {
        let mut indexed_class = InvertedEventIndex {
            terms: HashMap::new(),
        };

        if let Some(class_property) = event.indexed_properties.class.as_ref() {
            indexed_class.insert(&class_property.class.to_string());
        }

        for (timestamp, event_override) in event.overrides.iter() {
            if let Some(overridden_class) = &event_override.indexed_properties.extract_class() {
                indexed_class.insert_override(
                    timestamp.to_owned(),
                    &HashSet::from([overridden_class.to_string()]),
                );
            }
        }

        indexed_class
    }

    pub fn diff_indexed_terms(
        original: Option<&InvertedEventIndex<K>>,
        updated: Option<&InvertedEventIndex<K>>,
    ) -> UpdatedHashMapMembers<K, IndexedConclusion> {
        let original_terms = original.map(|inverted_index| inverted_index.terms.to_owned());
        let updated_terms = updated.map(|inverted_index| inverted_index.terms.to_owned());

        UpdatedHashMapMembers::new(original_terms.as_ref(), updated_terms.as_ref())
    }

    fn get_currently_indexed_terms(&self) -> HashSet<K>
    where
        K: Clone,
    {
        let mut indexed_terms_set: HashSet<K> = HashSet::new();

        for (term, indexed_conclusion) in self.terms.iter() {
            match indexed_conclusion {
                IndexedConclusion::Include(_) => {
                    indexed_terms_set.insert(term.clone());
                }

                _ => {
                    continue;
                }
            }
        }

        indexed_terms_set
    }

    pub fn insert(&mut self, term: &K)
    where
        K: std::hash::Hash + Clone + std::cmp::Eq,
    {
        self.terms
            .entry(term.clone())
            .and_modify(|indexed_term| {
                *indexed_term =
                    IndexedConclusion::merge_and(indexed_term, &IndexedConclusion::Include(None));
            })
            .or_insert_with(|| IndexedConclusion::Include(None));
    }

    pub fn insert_override(&mut self, timestamp: i64, override_terms_set: &HashSet<K>)
    where
        K: std::hash::Hash + Clone + std::cmp::Eq,
    {
        let indexed_terms_set = self.get_currently_indexed_terms();

        // Check for currently indexed terms NOT present in the override, and add them as an exception to
        // IndexedConclusion::Include (include all except timestamp).
        for excluded_term in indexed_terms_set.difference(override_terms_set) {
            if let Some(indexed_term) = self.terms.get_mut(excluded_term) {
                indexed_term.insert_exception(timestamp);
            }
        }

        // Check for overridden terms NOT already currently indexed, and add them as an
        // exception to IndexedConclusion::Exclude (exclude all except timestamp).
        for included_term in override_terms_set.difference(&indexed_terms_set) {
            self.terms
                .entry(included_term.clone())
                .and_modify(|indexed_term| {
                    indexed_term.insert_exception(timestamp);
                })
                .or_insert_with(|| IndexedConclusion::Exclude(Some(HashSet::from([timestamp]))));
        }
    }

    pub fn remove_override(&mut self, timestamp: i64) {
        self.terms.retain(|_removed_term, indexed_conclusion| {
            // Remove empty and redundant indexed conclusion (empty as in no exceptions).
            !(indexed_conclusion.remove_exception(timestamp) && indexed_conclusion.is_empty_exclude())
        });
    }
}

// Multi layer inverted index (for multiple events) - indexed term - event - include/exclude
#[derive(Debug, PartialEq, Clone)]
pub struct InvertedCalendarIndex<K>
where
    K: std::hash::Hash + Clone + Eq,
{
    pub terms: HashMap<K, InvertedCalendarIndexTerm>,
}

impl<K> Default for InvertedCalendarIndex<K>
where
    K: std::hash::Hash + Clone + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K> InvertedCalendarIndex<K>
where
    K: std::hash::Hash + Clone + Eq,
{
    pub fn new() -> Self {
        InvertedCalendarIndex {
            terms: HashMap::new(),
        }
    }

    pub fn insert(
        &mut self,
        event_uid: String,
        term: K,
        indexed_conclusion: &IndexedConclusion,
    ) -> Result<&mut Self, String> {
        self.terms
            .entry(term)
            .and_modify(|term_events| {
                match indexed_conclusion {
                    IndexedConclusion::Include(exceptions) => {
                        term_events.insert_included_event(event_uid.clone(), exceptions.clone())
                    }
                    IndexedConclusion::Exclude(exceptions) => {
                        term_events.insert_excluded_event(event_uid.clone(), exceptions.clone())
                    }
                };
            })
            .or_insert(InvertedCalendarIndexTerm::new_with_event(
                event_uid.clone(),
                indexed_conclusion.clone(),
            ));

        Ok(self)
    }

    pub fn remove(&mut self, event_uid: String, term: K) -> Result<&mut Self, String> {
        self.terms
            .entry(term)
            .and_modify(|inverted_calendar_index_term| {
                let _ = inverted_calendar_index_term.remove_event(event_uid);
            });

        Ok(self)
    }

    /// Returns an indexed Event set that matches the given term.
    pub fn get_term(&self, term: &K) -> Option<&InvertedCalendarIndexTerm> {
        self.terms.get(term)
    }

    /// Returns a virtual indexed event set of events where the given term does not match (NOT).
    /// As there may be other events in the calendar outside those indexed here, a vector of
    /// all the event uids contained in the calendar must be passed so that they can be referenced
    /// in the negated event set, as by design they will not match the given term.
    ///
    /// The negated virtual index is formed by building an index of full inclusions of all events
    /// in the calendar and then merging in the inverse of the event set of the given term.
    ///
    /// As this is a virtual index, ownership is transferred to the callsite.
    pub fn get_not_term(
        &self,
        term: &K,
        calendar_event_uids: &[String]
    ) -> InvertedCalendarIndexTerm {
        // Create an empty event set
        let mut negated_event_set = InvertedCalendarIndexTerm::new();

        // Initially index all events as Included
        for event_uid in calendar_event_uids.iter() {
            negated_event_set.insert_included_event(event_uid.to_owned(), None);
        }

        // Merge the inverse of the matching event set if present
        if let Some(matching_term_event_set) = self.get_term(term) {
            let not_matching_term_event_set = matching_term_event_set.inverse();

            for (event_uid, indexed_conclusion) in not_matching_term_event_set.events {
                // Remove Exclude(None) results or merge into the virtual index.
                if indexed_conclusion.is_empty_exclude() {
                    negated_event_set.events.remove(&event_uid);
                } else {
                    negated_event_set.events.insert(event_uid, indexed_conclusion);
                }
            }
        }

        negated_event_set
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IndexedConclusion {
    Include(Option<HashSet<i64>>),
    Exclude(Option<HashSet<i64>>),
}

impl IndexedConclusion {
    // Merges two IndexedConclusion structs with the AND logicial operator (intersecting).
    pub fn merge_and(
        indexed_conclusion_a: &IndexedConclusion,
        indexed_conclusion_b: &IndexedConclusion,
    ) -> IndexedConclusion {
        // Merging exceptions for same type - Include & Include
        fn merge_include_all_exception_sets(
            exceptions_a: &Option<HashSet<i64>>,
            exceptions_b: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            let exception_set_a = exceptions_a.clone().unwrap_or_default();
            let exception_set_b = exceptions_b.clone().unwrap_or_default();

            // Take all exceptions to include_a and combine with all exceptions to include_b with it.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_include all except [ 2, 3, 5, 8 ]
            //  combined:
            //    include all except  [ 1, 2, 3, 4, 5, 8 ]
            let compound_exception_set: HashSet<i64> = exception_set_a
                .union(&exception_set_b)
                .cloned()
                .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        // Merging exceptions for same type - Exclude & Exclude
        fn merge_exclude_all_exception_sets(
            exceptions_to_include_a: &Option<HashSet<i64>>,
            exceptions_to_include_b: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            let exception_set_to_include_a =
                exceptions_to_include_a.clone().unwrap_or_default();
            let exception_set_to_include_b =
                exceptions_to_include_b.clone().unwrap_or_default();

            // Take all intersecting exceptions to exclude_a, and exclude_b.
            // e.g.
            //  to_exclude all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    exclude all except  [ 2, 3 ]
            let compound_exception_set: HashSet<i64> = exception_set_to_include_a
                .intersection(&exception_set_to_include_b)
                .cloned()
                .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        // Merging exceptions for differing types:
        //  (e.g. (Include all - overrides) & (Exclude - overrides))
        fn merge_unaligned_exception_sets(
            exceptions_to_include: &Option<HashSet<i64>>,
            exceptions_to_exclude: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            let exception_set_to_include = exceptions_to_include.clone().unwrap_or_default();
            let exception_set_to_exclude = exceptions_to_exclude.clone().unwrap_or_default();

            // Take all exceptions to include and subtract all exceptions to exclude from it.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    exclude all except  [ 5, 8 ]
            let compound_exception_set: HashSet<i64> = exception_set_to_include
                .difference(&exception_set_to_exclude)
                .cloned()
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
                IndexedConclusion::Include(exceptions_b),
            ) => IndexedConclusion::Include(merge_include_all_exception_sets(
                exceptions_a,
                exceptions_b,
            )),

            (
                IndexedConclusion::Exclude(exceptions_a),
                IndexedConclusion::Exclude(exceptions_b),
            ) => IndexedConclusion::Exclude(merge_exclude_all_exception_sets(
                exceptions_a,
                exceptions_b,
            )),

            (
                IndexedConclusion::Include(exceptions_to_exclude),
                IndexedConclusion::Exclude(exceptions_to_include),
            )
            | (
                IndexedConclusion::Exclude(exceptions_to_include),
                IndexedConclusion::Include(exceptions_to_exclude),
            ) => IndexedConclusion::Exclude(merge_unaligned_exception_sets(
                exceptions_to_include,
                exceptions_to_exclude,
            )),
        }
    }

    // Merges two IndexedConclusion structs with the OR logicial operator (union).
    pub fn merge_or(
        indexed_conclusion_a: &IndexedConclusion,
        indexed_conclusion_b: &IndexedConclusion,
    ) -> IndexedConclusion {
        // Merging exceptions for same type - Include & Include
        fn merge_include_all_exception_sets(
            exceptions_a: &Option<HashSet<i64>>,
            exceptions_b: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            if exceptions_a.is_none() || exceptions_b.is_none() {
                return None;
            }

            let exception_set_a = exceptions_a.clone().unwrap_or_default();
            let exception_set_b = exceptions_b.clone().unwrap_or_default();

            // Combine all exceptions to include_a, and include_b removing intersecting exceptions.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_include all except [ 2, 3, 5, 8 ]
            //  combined:
            //    include all except  [ 1, 4, 5, 8 ]
            let combined_exception_set: HashSet<i64> = exception_set_a
                .union(&exception_set_b)
                .cloned()
                .collect();

            let intersecting_exception_set: HashSet<i64> = exception_set_a
                .intersection(&exception_set_b)
                .cloned()
                .collect();

            let subtracted_exception_set: HashSet<i64> = combined_exception_set
                .difference(&intersecting_exception_set)
                .cloned()
                .collect();

            if subtracted_exception_set.is_empty() {
                None
            } else {
                Some(subtracted_exception_set)
            }
        }

        // Merging exceptions for same type - Exclude & Exclude
        fn merge_exclude_all_exception_sets(
            exceptions_to_include_a: &Option<HashSet<i64>>,
            exceptions_to_include_b: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            let exception_set_to_include_a =
                exceptions_to_include_a.clone().unwrap_or_default();
            let exception_set_to_include_b =
                exceptions_to_include_b.clone().unwrap_or_default();

            // Take all exceptions to exclude_a and combine with all exceptions to exclude_b with it.
            // e.g.
            //  to_exclude all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    exclude all except  [ 2, 3 ]
            let compound_exception_set: HashSet<i64> = exception_set_to_include_a
                .union(&exception_set_to_include_b)
                .cloned()
                .collect();

            if compound_exception_set.is_empty() {
                None
            } else {
                Some(compound_exception_set)
            }
        }

        // Merging exceptions for differing types:
        //  (e.g. (Include all - overrides) & (Exclude - overrides))
        fn merge_unaligned_exception_sets(
            exceptions_to_include: &Option<HashSet<i64>>,
            exceptions_to_exclude: &Option<HashSet<i64>>,
        ) -> Option<HashSet<i64>> {
            let exception_set_to_include = exceptions_to_include.clone().unwrap_or_default();
            let exception_set_to_exclude = exceptions_to_exclude.clone().unwrap_or_default();

            // Take all exceptions to exclude and subtract all exceptions to include from it.
            // e.g.
            //  to_include all except [ 1, 2, 3, 4 ]
            //  to_exclude all except [ 2, 3, 5, 8 ]
            //  combined:
            //    include all except  [ 1, 4 ]
            let compound_exception_set: HashSet<i64> = exception_set_to_exclude
                .difference(&exception_set_to_include)
                .cloned()
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
                IndexedConclusion::Include(exceptions_b),
            ) => IndexedConclusion::Include(merge_include_all_exception_sets(
                exceptions_a,
                exceptions_b,
            )),

            (
                IndexedConclusion::Exclude(exceptions_a),
                IndexedConclusion::Exclude(exceptions_b),
            ) => IndexedConclusion::Exclude(merge_exclude_all_exception_sets(
                exceptions_a,
                exceptions_b,
            )),

            (
                IndexedConclusion::Include(exceptions_to_exclude),
                IndexedConclusion::Exclude(exceptions_to_include),
            )
            | (
                IndexedConclusion::Exclude(exceptions_to_include),
                IndexedConclusion::Include(exceptions_to_exclude),
            ) => IndexedConclusion::Include(merge_unaligned_exception_sets(
                exceptions_to_include,
                exceptions_to_exclude,
            )),
        }
    }

    // Flips the polarity of an IndexedConclusion whilst maintaining exceptions.
    pub fn negate(&self) -> Self {
        match self {
            Self::Include(exceptions) => Self::Exclude(exceptions.clone()),
            Self::Exclude(exceptions) => Self::Include(exceptions.clone()),
        }
    }

    pub fn min_max_exceptions(&self) -> Option<(i64, i64)> {
        let exceptions = match self {
            IndexedConclusion::Include(exceptions) => exceptions,
            IndexedConclusion::Exclude(exceptions) => exceptions,
        };

        let Some(exceptions) = exceptions else {
            return None;
        };

        if exceptions.is_empty() {
            return None;
        }

        let mut min_max: Option<(i64, i64)> = None;

        for exception in exceptions {
            if let Some((min, max)) = min_max {
                if *exception > max {
                    min_max = Some((min, *exception));
                }

                if *exception < min {
                    min_max = Some((*exception, max));
                }
            } else {
                min_max = Some((*exception, *exception));
            }
        }

        min_max
    }

    pub fn is_empty_exclude(&self) -> bool {
        match self {
            IndexedConclusion::Include(_) => false,
            IndexedConclusion::Exclude(overrides) => overrides.is_none(),
        }
    }

    pub fn exclude_event_occurrence(&self, occurrence: i64) -> bool {
        !self.include_event_occurrence(occurrence)
    }

    pub fn include_event_occurrence(&self, occurrence: i64) -> bool {
        match self {
            IndexedConclusion::Include(_) => !self.contains_exception(occurrence),
            IndexedConclusion::Exclude(_) => self.contains_exception(occurrence),
        }
    }

    pub fn contains_exception(&self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => {
                Self::exception_set_contains(exceptions, exception)
            }
            IndexedConclusion::Exclude(exceptions) => {
                Self::exception_set_contains(exceptions, exception)
            }
        }
    }

    pub fn insert_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => {
                Self::push_to_exception_set(exceptions, exception)
            }
            IndexedConclusion::Exclude(exceptions) => {
                Self::push_to_exception_set(exceptions, exception)
            }
        }
    }

    pub fn remove_exception(&mut self, exception: i64) -> bool {
        match self {
            IndexedConclusion::Include(exceptions) => {
                Self::remove_from_exception_set(exceptions, exception)
            }
            IndexedConclusion::Exclude(exceptions) => {
                Self::remove_from_exception_set(exceptions, exception)
            }
        }
    }

    fn exception_set_contains(exceptions: &Option<HashSet<i64>>, exception: i64) -> bool {
        match exceptions {
            Some(exception_set) => exception_set.contains(&exception),
            None => false,
        }
    }

    fn push_to_exception_set(exceptions: &mut Option<HashSet<i64>>, exception: i64) -> bool {
        match exceptions {
            Some(exception_set) => exception_set.insert(exception),
            None => {
                *exceptions = Some(HashSet::from([exception]));

                true
            }
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
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    fn example_calendar_index() -> InvertedCalendarIndex<String> {
        InvertedCalendarIndex {
            terms: HashMap::from([
                (
                    String::from("ONLINE"),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([
                            (
                                String::from("Always online"),
                                IndexedConclusion::Include(None)
                            ),
                            (
                                String::from("Mostly online"),
                                IndexedConclusion::Include(Some([100].into()))
                            ),
                            (
                                String::from("Mostly in person"),
                                IndexedConclusion::Exclude(Some([100].into()))
                            ),
                        ])
                    }
                ),
                (
                    String::from("IN-PERSON"),
                    InvertedCalendarIndexTerm {
                        events: HashMap::from([
                            (
                                String::from("Always in person"),
                                IndexedConclusion::Include(None)
                            ),
                            (
                                String::from("Mostly in person"),
                                IndexedConclusion::Include(Some([100].into()))
                            ),
                            (
                                String::from("Mostly online"),
                                IndexedConclusion::Exclude(Some([100].into()))
                            ),
                        ])
                    }
                )
            ])
        }
    }

    #[test]
    fn test_inverted_calendar_index_get_term() {
        let index = example_calendar_index();

        // With a term that is indexed it returns the corresponding term event set
        assert_eq!(
            index.get_term(&String::from("ONLINE")),
            Some(
                &InvertedCalendarIndexTerm {
                    events: HashMap::from([
                        (
                            String::from("Always online"),
                            IndexedConclusion::Include(None)
                        ),
                        (
                            String::from("Mostly online"),
                            IndexedConclusion::Include(Some([100].into()))
                        ),
                        (
                            String::from("Mostly in person"),
                            IndexedConclusion::Exclude(Some([100].into()))
                        ),
                    ])
                }
            )
        );

        // With a term that is not indexed it returns None
        assert_eq!(index.get_term(&String::from("FOOBAR")), None);
    }

    #[test]
    fn test_inverted_calendar_index_get_not_term() {
        let index = example_calendar_index();

        // Contains some event uids not included in the target index to mimic events indexed
        // elsewhere in the calendar (e.g. another index).
        let calendar_event_uids = vec![
            String::from("Always online"),
            String::from("Always in person"),
            String::from("Mostly online"),
            String::from("Mostly in person"),
            String::from("Not specified 1"),
            String::from("Not specified 2"),
        ];

        // With a term that is indexed it merges the negated term index into all events:
        assert_eq!(
            index.get_not_term(
                &String::from("ONLINE"),
                &calendar_event_uids
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("Always in person"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Exclude(Some([100].into())),
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(Some([100].into())),
                    ),
                    (
                        String::from("Not specified 1"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Not specified 2"),
                        IndexedConclusion::Include(None),
                    ),
                ])
            }
        );

        // With a term that is not indexed it returns a virtual index of all calendar events:
        assert_eq!(
            index.get_not_term(
                &String::from("FOOBAR"),
                &calendar_event_uids
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (
                        String::from("Always online"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Always in person"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Mostly online"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Mostly in person"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Not specified 1"),
                        IndexedConclusion::Include(None),
                    ),
                    (
                        String::from("Not specified 2"),
                        IndexedConclusion::Include(None),
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_inverted_index_term_merge_and() {
        assert_eq_sorted!(
            InvertedCalendarIndexTerm::merge_and(
                &InvertedCalendarIndexTerm {
                    events: HashMap::from([
                        (
                            String::from("event_one"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_two"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_three"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_four"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_five"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_six"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_seven"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_eight"),
                            IndexedConclusion::Exclude(None)
                        ),
                        (String::from("event_nine"), IndexedConclusion::Exclude(None)),
                    ])
                },
                &InvertedCalendarIndexTerm {
                    events: HashMap::from([
                        (
                            String::from("event_one"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_two"),
                            IndexedConclusion::Include(Some(HashSet::from([200, 300])))
                        ),
                        (
                            String::from("event_three"),
                            IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))
                        ),
                        (String::from("event_four"), IndexedConclusion::Exclude(None)),
                        (String::from("event_five"), IndexedConclusion::Include(None)),
                        (String::from("event_six"), IndexedConclusion::Exclude(None)),
                        (
                            String::from("event_seven"),
                            IndexedConclusion::Include(None)
                        ),
                        (
                            String::from("event_eight"),
                            IndexedConclusion::Include(None)
                        ),
                    ]),
                }
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("event_one"), IndexedConclusion::Exclude(None)),
                    (
                        String::from("event_two"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200, 300])))
                    ),
                    (
                        String::from("event_three"),
                        IndexedConclusion::Exclude(Some(HashSet::from([200])))
                    ),
                    (String::from("event_four"), IndexedConclusion::Exclude(None)),
                    (
                        String::from("event_five"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (String::from("event_six"), IndexedConclusion::Exclude(None)),
                    (
                        String::from("event_seven"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("event_eight"),
                        IndexedConclusion::Exclude(None)
                    ),
                ]),
            },
        );
    }

    #[test]
    fn test_inverted_index_term_merge_or() {
        assert_eq_sorted!(
            InvertedCalendarIndexTerm::merge_or(
                &InvertedCalendarIndexTerm {
                    events: HashMap::from([
                        (
                            String::from("event_one"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_two"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_three"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_four"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_five"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_six"),
                            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_seven"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_eight"),
                            IndexedConclusion::Exclude(None)
                        ),
                        (String::from("event_nine"), IndexedConclusion::Exclude(None)),
                    ])
                },
                &InvertedCalendarIndexTerm {
                    events: HashMap::from([
                        (
                            String::from("event_one"),
                            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                        ),
                        (
                            String::from("event_two"),
                            IndexedConclusion::Include(Some(HashSet::from([200, 300])))
                        ),
                        (
                            String::from("event_three"),
                            IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))
                        ),
                        (String::from("event_four"), IndexedConclusion::Exclude(None)),
                        (String::from("event_five"), IndexedConclusion::Include(None)),
                        (String::from("event_six"), IndexedConclusion::Exclude(None)),
                        (
                            String::from("event_seven"),
                            IndexedConclusion::Include(None)
                        ),
                        (
                            String::from("event_eight"),
                            IndexedConclusion::Include(None)
                        ),
                        (
                            String::from("event_ten"),
                            IndexedConclusion::Exclude(Some(HashSet::from([200])))
                        ),
                    ]),
                }
            ),
            InvertedCalendarIndexTerm {
                events: HashMap::from([
                    (String::from("event_one"), IndexedConclusion::Include(None)),
                    (
                        String::from("event_two"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 300])))
                    ),
                    (
                        String::from("event_three"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200, 300])))
                    ),
                    (
                        String::from("event_four"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
                    ),
                    (String::from("event_five"), IndexedConclusion::Include(None)),
                    (
                        String::from("event_six"),
                        IndexedConclusion::Include(Some(HashSet::from([100, 200])))
                    ),
                    (
                        String::from("event_seven"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("event_eight"),
                        IndexedConclusion::Include(None)
                    ),
                    (
                        String::from("event_ten"),
                        IndexedConclusion::Exclude(Some(HashSet::from([200])))
                    ),
                ]),
            },
        );
    }

    #[test]
    fn test_inverted_index_term_inverse() {
        let events = vec![
            (String::from("Event one"), IndexedConclusion::Include(None)),
            (String::from("Event two"), IndexedConclusion::Include(Some(HashSet::from([100])))),
            (String::from("Event three"), IndexedConclusion::Exclude(None)),
            (String::from("Event four"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
        ];

        let inverted_index_term = InvertedCalendarIndexTerm::new_with_events(events);

        assert_eq!(
            inverted_index_term.inverse(),
            InvertedCalendarIndexTerm {
                events: HashMap::from(
                    [
                        (String::from("Event one"), IndexedConclusion::Exclude(None)),
                        (String::from("Event two"), IndexedConclusion::Exclude(Some(HashSet::from([100])))),
                        (String::from("Event three"), IndexedConclusion::Include(None)),
                        (String::from("Event four"), IndexedConclusion::Include(Some(HashSet::from([100])))),
                    ]
                )
            }
        );
    }

    #[test]
    fn test_indexed_conclusion_merge_and() {
        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
            ),
            IndexedConclusion::Exclude(None),
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 200, 300])))
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([200])))
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Exclude(None)
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Exclude(None)
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge_and(
                &IndexedConclusion::Exclude(None),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Exclude(None)
        );
    }

    #[test]
    fn test_indexed_conclusion_merge_or() {
        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
            ),
            IndexedConclusion::Include(None),
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 300])))
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(Some(HashSet::from([200, 300])))
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200, 300])))
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Include(None)
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Exclude(None)
            ),
            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Include(None)
        );

        assert_eq!(
            IndexedConclusion::merge_or(
                &IndexedConclusion::Exclude(None),
                &IndexedConclusion::Include(None)
            ),
            IndexedConclusion::Include(None)
        );
    }

    #[test]
    fn test_indexed_conclusion_negate() {
        assert_eq!(
            IndexedConclusion::Include(None).negate(),
            IndexedConclusion::Exclude(None)
        );

        assert_eq!(
            IndexedConclusion::Include(Some(HashSet::from([100, 200]))).negate(),
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))),
        );

        assert_eq!(
            IndexedConclusion::Exclude(None).negate(),
            IndexedConclusion::Include(None)
        );

        assert_eq!(
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200]))).negate(),
            IndexedConclusion::Include(Some(HashSet::from([100, 200]))),
        );
    }

    #[test]
    fn test_indexed_conclusion() {
        let mut included_event = IndexedConclusion::Include(None);
        let mut excluded_event = IndexedConclusion::Exclude(None);

        // Testing min/max

        assert_eq!(included_event.min_max_exceptions(), None);
        assert_eq!(excluded_event.min_max_exceptions(), None);

        // Testing exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(100), true);
        assert_eq!(excluded_event.insert_exception(100), true);

        assert_eq!(
            included_event,
            IndexedConclusion::Include(Some(HashSet::from([100])))
        );

        assert_eq!(
            excluded_event,
            IndexedConclusion::Exclude(Some(HashSet::from([100])))
        );

        // Testing min/max

        assert_eq!(included_event.min_max_exceptions(), Some((100, 100)));
        assert_eq!(excluded_event.min_max_exceptions(), Some((100, 100)));

        // Testing multiple exception inserts into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), true);
        assert_eq!(excluded_event.insert_exception(200), true);

        assert_eq!(
            included_event,
            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            excluded_event,
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
        );

        // Testing inserting existing exception into both Include and Exclude

        assert_eq!(included_event.insert_exception(200), false);
        assert_eq!(excluded_event.insert_exception(200), false);

        assert_eq!(
            included_event,
            IndexedConclusion::Include(Some(HashSet::from([100, 200])))
        );

        assert_eq!(
            excluded_event,
            IndexedConclusion::Exclude(Some(HashSet::from([100, 200])))
        );

        // Testing checking existence of both existing and non-existing exceptions from populated Include and Exclude

        assert_eq!(included_event.contains_exception(100), true);
        assert_eq!(included_event.contains_exception(200), true);
        assert_eq!(included_event.contains_exception(300), false);

        assert_eq!(excluded_event.contains_exception(100), true);
        assert_eq!(excluded_event.contains_exception(200), true);
        assert_eq!(excluded_event.contains_exception(300), false);

        // Testing min/max

        assert_eq!(included_event.min_max_exceptions(), Some((100, 200)));
        assert_eq!(excluded_event.min_max_exceptions(), Some((100, 200)));

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

        assert_eq!(
            included_event,
            IndexedConclusion::Include(Some(HashSet::from([100])))
        );

        assert_eq!(
            excluded_event,
            IndexedConclusion::Exclude(Some(HashSet::from([100])))
        );

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
