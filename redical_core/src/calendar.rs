use std::collections::{BTreeSet, BTreeMap, HashMap};

use crate::inverted_index::{IndexedConclusion, InvertedCalendarIndex};

use crate::utils::{KeyValuePair, UpdatedHashMapMembers};

use crate::geo_index::{GeoPoint, GeoSpatialCalendarIndex};

use crate::event::Event;

use redical_ical::{
    ICalendarComponent,
    RenderingContext,
    content_line::ContentLine,
    properties::{
        ICalendarProperty,
        CalendarProperty,
        UIDProperty,
    },
};

#[derive(Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uid: UIDProperty,
    pub events: BTreeMap<String, Box<Event>>,
    pub indexes_active: bool,
    pub indexed_categories: InvertedCalendarIndex<String>,
    pub indexed_location_type: InvertedCalendarIndex<String>,
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,
    pub indexed_geo: GeoSpatialCalendarIndex,
    pub indexed_class: InvertedCalendarIndex<String>,
}

impl Calendar {
    pub fn new(uid: String) -> Self {
        Calendar {
            uid: uid.into(),
            events: BTreeMap::new(),
            indexes_active: true,
            indexed_categories: InvertedCalendarIndex::new(),
            indexed_location_type: InvertedCalendarIndex::new(),
            indexed_related_to: InvertedCalendarIndex::new(),
            indexed_geo: GeoSpatialCalendarIndex::new(),
            indexed_class: InvertedCalendarIndex::new(),
        }
    }

    pub fn insert(&mut self, property: CalendarProperty) -> Result<&Self, String> {
        match property {
            CalendarProperty::UID(uid_property) => {
                if self.uid.uid != uid_property.uid {
                    return Err(
                        format!("Inserted calendar UID: {} does not match existing UID: {}", uid_property.uid, self.uid.uid)
                    );
                }
            },
        }

        Ok(self)
    }

    pub fn get_event(&self, event_uid: &String) -> Option<&Event> {
        self.events.get(event_uid).map(|boxed_event| boxed_event.as_ref())
    }

    pub fn insert_event(&mut self, event: Event) -> Option<Event> {
        use std::collections::btree_map::Entry;

        match self.events.entry(event.uid.uid.to_string()) {
            Entry::Occupied(mut entry) => {
                let boxed_event = entry.get_mut();

                // Swap boxed event value out with new one to avoid copying
                // the entire Calendar everytime we want to make an update.
                Some(
                    std::mem::replace(&mut **boxed_event, event)
                )
            },

            Entry::Vacant(entry) => {
                entry.insert(Box::new(event));

                None
            },
        }
    }

    pub fn remove_event(&mut self, event_uid: &String) -> Option<Box<Event>> {
        self.events.remove(event_uid)
    }

    fn clear_indexes(&mut self) {
        self.indexed_categories = InvertedCalendarIndex::new();
        self.indexed_related_to = InvertedCalendarIndex::new();
        self.indexed_geo = GeoSpatialCalendarIndex::new();
        self.indexed_class = InvertedCalendarIndex::new();
    }

    // Disable and clear the indexes on the Calendar.
    // This is useful when performing bulk data imports where we want to ingest the
    // Event's and EventOccurrenceOverride's as quickly as possible and build the indexes
    // at the end instead of slowing down down the process by ineffiently rebuilding throughout.
    pub fn disable_indexes(&mut self) {
        self.indexes_active = false;

        // Clear the indexes on disabling indexing on the Calendar
        // to keep the memory footprint efficient.
        self.clear_indexes();
    }

    // Rebuild the Calendar indexes from scratch, very helpful to perform at the tail
    // end of a bulk data import.
    pub fn rebuild_indexes(&mut self) -> Result<bool, String> {
        // Clear the indexes first to ensure full clean rebuild.
        self.clear_indexes();

        // Ensure indexes are re-enabled.
        self.indexes_active = true;

        let indexed_categories = &mut self.indexed_categories;
        let indexed_related_to = &mut self.indexed_related_to;
        let indexed_location_type = &mut self.indexed_location_type;
        let indexed_geo = &mut self.indexed_geo;
        let indexed_class = &mut self.indexed_class;

        for event in self.events.values_mut() {
            let event_uid = event.uid.uid.to_string();

            event.rebuild_indexes()?;

            if let Some(indexed_event_categories) = &event.indexed_categories {
                for (indexed_term, indexed_conclusion) in &indexed_event_categories.terms {
                    indexed_categories.insert(event_uid.to_owned(), indexed_term.to_owned(), indexed_conclusion)?;
                }
            }

            if let Some(indexed_event_related_to) = &event.indexed_related_to {
                for (indexed_term, indexed_conclusion) in &indexed_event_related_to.terms {
                    indexed_related_to.insert(event_uid.to_owned(), indexed_term.to_owned(), indexed_conclusion)?;
                }
            }

            if let Some(indexed_event_location_type) = &event.indexed_location_type {
                for (indexed_term, indexed_conclusion) in &indexed_event_location_type.terms {
                    indexed_location_type.insert(event_uid.to_owned(), indexed_term.to_owned(), indexed_conclusion)?;
                }
            }

            if let Some(indexed_event_geo) = &event.indexed_geo {
                for (indexed_long_lat_coord, indexed_conclusion) in &indexed_event_geo.terms {
                    indexed_geo.insert(event_uid.to_owned(), indexed_long_lat_coord, indexed_conclusion)?;
                }
            }

            if let Some(indexed_event_class) = &event.indexed_class {
                for (indexed_term, indexed_conclusion) in &indexed_event_class.terms {
                    indexed_class.insert(event_uid.to_owned(), indexed_term.to_owned(), indexed_conclusion)?;
                }
            }
        }

        Ok(true)
    }

    // Iterates through associated events and finds those that have their last occurrence between
    // the from and until timestamps.
    pub fn prune_events(&mut self, from: i64, until: i64) -> Result<HashMap<String, Box<Event>>, String> {
        let mut pruned_events = HashMap::new();

        let event_uids_to_prune = self.events.iter()
            .filter(|(_, event)| event.is_last_occurrence_between(from, until).unwrap_or(false))
            .map(|(uid, _)| uid.to_string())
            .collect::<Vec<String>>();

        for uid in event_uids_to_prune.iter() {
            if let Some(pruned_event) = self.remove_event(uid) {
                pruned_events.insert(uid.to_string(), pruned_event);
            }
        }

        Ok(pruned_events)
    }
}

impl ICalendarComponent for Calendar {
    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine> {
        let mut serializable_properties: BTreeSet<ContentLine> = BTreeSet::new();

        serializable_properties.insert(self.uid.to_content_line_with_context(context));

        serializable_properties
    }
}

#[derive(Debug)]
pub struct CalendarIndexUpdater<'a> {
    pub event_uid: String,
    pub calendar: &'a mut Calendar,
}

impl<'a> CalendarIndexUpdater<'a> {
    pub fn new(event_uid: &String, calendar: &'a mut Calendar) -> Self {
        CalendarIndexUpdater {
            event_uid: event_uid.to_owned(),
            calendar,
        }
    }

    pub fn update_indexed_categories(
        &mut self,
        updated_event_categories_diff: &UpdatedHashMapMembers<String, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_categories = &mut self.calendar.indexed_categories;

        for (removed_category, _) in updated_event_categories_diff.removed.iter() {
            indexed_categories.remove(self.event_uid.clone(), removed_category.clone())?;
        }

        for (updated_category, updated_indexed_conclusion) in
            updated_event_categories_diff.updated.iter()
        {
            indexed_categories.insert(
                self.event_uid.clone(),
                updated_category.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_category, added_indexed_conclusion) in updated_event_categories_diff.added.iter()
        {
            indexed_categories.insert(
                self.event_uid.clone(),
                added_category.clone(),
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }

    pub fn update_indexed_location_type(
        &mut self,
        updated_event_location_type_diff: &UpdatedHashMapMembers<String, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_location_type = &mut self.calendar.indexed_location_type;

        for (removed_location_type, _) in updated_event_location_type_diff.removed.iter() {
            indexed_location_type.remove(self.event_uid.clone(), removed_location_type.clone())?;
        }

        for (updated_location_type, updated_indexed_conclusion) in
            updated_event_location_type_diff.updated.iter()
        {
            indexed_location_type.insert(
                self.event_uid.clone(),
                updated_location_type.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_location_type, added_indexed_conclusion) in updated_event_location_type_diff.added.iter()
        {
            indexed_location_type.insert(
                self.event_uid.clone(),
                added_location_type.clone(),
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }

    pub fn update_indexed_related_to(
        &mut self,
        updated_event_related_to_diff: &UpdatedHashMapMembers<KeyValuePair, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_related_to = &mut self.calendar.indexed_related_to;

        for (removed_related_to, _) in updated_event_related_to_diff.removed.iter() {
            indexed_related_to.remove(self.event_uid.clone(), removed_related_to.clone())?;
        }

        for (updated_related_to, updated_indexed_conclusion) in
            updated_event_related_to_diff.updated.iter()
        {
            indexed_related_to.insert(
                self.event_uid.clone(),
                updated_related_to.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_related_to, added_indexed_conclusion) in
            updated_event_related_to_diff.added.iter()
        {
            indexed_related_to.insert(
                self.event_uid.clone(),
                added_related_to.clone(),
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }

    pub fn update_indexed_geo(
        &mut self,
        updated_event_geo_diff: &UpdatedHashMapMembers<GeoPoint, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_geo = &mut self.calendar.indexed_geo;

        for (removed_long_lat_coord, _) in updated_event_geo_diff.removed.iter() {
            indexed_geo.remove(self.event_uid.clone(), removed_long_lat_coord)?;
        }

        for (updated_long_lat_coord, updated_indexed_conclusion) in
            updated_event_geo_diff.updated.iter()
        {
            indexed_geo.insert(
                self.event_uid.clone(),
                updated_long_lat_coord,
                updated_indexed_conclusion,
            )?;
        }

        for (added_long_lat_coord, added_indexed_conclusion) in updated_event_geo_diff.added.iter()
        {
            indexed_geo.insert(
                self.event_uid.clone(),
                added_long_lat_coord,
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }


    pub fn update_indexed_class(
        &mut self,
        updated_event_class_diff: &UpdatedHashMapMembers<String, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_class = &mut self.calendar.indexed_class;

        for (removed_class, _) in updated_event_class_diff.removed.iter() {
            indexed_class.remove(self.event_uid.clone(), removed_class.clone())?;
        }

        for (updated_class, updated_indexed_conclusion) in updated_event_class_diff.updated.iter() {
            indexed_class.insert(
                self.event_uid.clone(),
                updated_class.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_class, added_indexed_conclusion) in updated_event_class_diff.added.iter() {
            indexed_class.insert(
                self.event_uid.clone(),
                added_class.clone(),
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;
    use redical_ical::values::date_time::DateTime;

    fn build_and_associate_event(calendar: &mut Calendar, uid: &str, rrule_ical: &str) -> Event {
        let mut event = Event::new(uid.to_string());
        let rrule_set = rrule_ical.parse().unwrap();

        event.schedule_properties.parsed_rrule_set = Some(rrule_set);

        calendar.insert_event(event.clone());

        event
    }

    #[test]
    fn it_prunes_events_between_timestamps() {
        let from  = DateTime::from_str("20250101T090000Z").unwrap().get_utc_timestamp(None);
        let until = DateTime::from_str("20250102T090000Z").unwrap().get_utc_timestamp(None);

        let mut calendar = Calendar::new("CALENDAR_UID".to_string());

        // Recurring event that does not terminate
        let event_one = build_and_associate_event(
            &mut calendar,
            "EVENT_ONE",
            "DTSTART:20241231T163000Z\nRRULE:FREQ=DAILY",
        );

        // Recurring event that terminates after the prune range
        let event_two = build_and_associate_event(
            &mut calendar,
            "EVENT_TWO",
            "DTSTART:20241231T163000Z\nRRULE:FREQ=DAILY;COUNT=10",
        );

        // Recurring event that terminates inside prune range
        let event_three = build_and_associate_event(
            &mut calendar,
            "EVENT_THREE",
            "DTSTART:20241231T163000Z\nRRULE:FREQ=DAILY;COUNT=2",
        );

        // Single event that terminates before prune range
        let event_four = build_and_associate_event(
            &mut calendar,
            "EVENT_FOUR",
            "DTSTART:20241231T163000Z\nRDATE:20241231T163000Z",
        );

        // Single event that terminates inside prune range
        let event_five = build_and_associate_event(
            &mut calendar,
            "EVENT_FIVE",
            "DTSTART:20250101T123000Z\nRDATE:20250101T123000Z",
        );

        assert_eq!(calendar.events.len(), 5);

        let pruned_events = calendar.prune_events(from, until).unwrap();
        
        assert_eq!(calendar.events.len(), 3);
        assert_eq!(pruned_events.len(), 2);
        
        // Expected pruned events are removed
        assert_eq!(
            pruned_events,
            HashMap::from([
                ("EVENT_THREE".to_string(), Box::new(event_three)),
                ("EVENT_FIVE".to_string(), Box::new(event_five)),
            ]),
        );

        // Other events are still in the Calendar
        assert_eq!(calendar.events.get("EVENT_ONE"), Some(&Box::new(event_one)));
        assert_eq!(calendar.events.get("EVENT_TWO"), Some(&Box::new(event_two)));
        assert_eq!(calendar.events.get("EVENT_FOUR"), Some(&Box::new(event_four)));
    }
}
