use serde::{Serialize, Deserialize};

use std::collections::HashMap;

use crate::data_types::inverted_index::{InvertedCalendarIndex, IndexedConclusion};

use crate::data_types::utils::{KeyValuePair, UpdatedHashMapMembers};

use crate::data_types::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uuid:               String,
    pub events:             HashMap<String, Event>,
    pub indexed_categories: InvertedCalendarIndex<String>,
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,
}

impl Calendar {

    pub fn new(uuid: String) -> Self {
        Calendar {
            uuid,
            events:             HashMap::new(),
            indexed_categories: InvertedCalendarIndex::new(),
            indexed_related_to: InvertedCalendarIndex::new(),
        }
    }
}

#[derive(Debug)]
pub struct CalendarIndexUpdater<'a> {
    pub event_uuid:  String,
    pub calendar:    &'a mut Calendar,
}

impl <'a>CalendarIndexUpdater<'a> {

    pub fn new(event_uuid: String, calendar: &'a mut Calendar) -> Self {
        CalendarIndexUpdater {
            event_uuid,
            calendar,
        }
    }

    pub fn update_indexed_categories(&mut self, updated_event_categories_diff: &UpdatedHashMapMembers<String, IndexedConclusion>) -> Result<bool, String> {
        let indexed_categories = &mut self.calendar.indexed_categories;

        for (removed_category, _) in updated_event_categories_diff.removed.iter() {
            indexed_categories.remove(self.event_uuid.clone(), removed_category.clone())?;
        }

        for (updated_category, updated_indexed_conclusion) in updated_event_categories_diff.updated.iter() {
            indexed_categories.insert(self.event_uuid.clone(), updated_category.clone(), updated_indexed_conclusion)?;
        }

        for (added_category, added_indexed_conclusion) in updated_event_categories_diff.added.iter() {
            indexed_categories.insert(self.event_uuid.clone(), added_category.clone(), added_indexed_conclusion)?;
        }

        Ok(true)
    }

    pub fn update_indexed_related_to(&mut self, updated_event_related_to_diff: &UpdatedHashMapMembers<KeyValuePair, IndexedConclusion>) -> Result<bool, String> {
        let indexed_related_to = &mut self.calendar.indexed_related_to;

        for (removed_related_to, _) in updated_event_related_to_diff.removed.iter() {
            indexed_related_to.remove(self.event_uuid.clone(), removed_related_to.clone())?;
        }

        for (updated_related_to, updated_indexed_conclusion) in updated_event_related_to_diff.updated.iter() {
            indexed_related_to.insert(self.event_uuid.clone(), updated_related_to.clone(), updated_indexed_conclusion)?;
        }

        for (added_related_to, added_indexed_conclusion) in updated_event_related_to_diff.added.iter() {
            indexed_related_to.insert(self.event_uuid.clone(), added_related_to.clone(), added_indexed_conclusion)?;
        }

        Ok(true)
    }
}

#[derive(Debug)]
pub struct CalendarCategoryIndexUpdater<'a> {
    pub calendar_index_updater: &'a mut CalendarIndexUpdater<'a>,
}

impl<'a> CalendarCategoryIndexUpdater<'a> {

    pub fn new(calendar_index_updater: &'a mut CalendarIndexUpdater<'a>) -> Self {
        CalendarCategoryIndexUpdater {
            calendar_index_updater
        }
    }

    pub fn remove_event_from_calendar(&mut self, original_event: &Event) -> Result<bool, String> {
        let original_uuid = &original_event.uuid;
        let current_uuid = &self.calendar_index_updater.event_uuid;

        if original_uuid != current_uuid {
            return Err(format!("Cannot remove Event categories from the Calendar because of mismatched UUIDs - original: '{original_uuid}' expected: '{current_uuid}'"));
        }

        if let Some(indexed_categories) = &original_event.indexed_categories {
            for (category, _) in indexed_categories.terms.iter() {
                // Update the calendar with None as indexed_conclusion so that it deletes the
                // related_to associated with the event which has now been removed from
                // the calendar.
                Self::update_calendar(self.calendar_index_updater.calendar, original_uuid.clone(), category.clone(), None)?;
            }
        }

        Ok(true)
    }

    fn update_calendar(calendar: &mut Calendar, event_uuid: String, updated_term: String, indexed_conclusion: Option<&IndexedConclusion>) -> Result<bool, String> {
        match indexed_conclusion {
            Some(indexed_conclusion) => {
                calendar.indexed_categories.insert(event_uuid, updated_term, indexed_conclusion)?;
            },

            None => {
                calendar.indexed_categories.remove(event_uuid, updated_term)?;
            }
        };

        Ok(true)
    }

    fn handle_update(&mut self, updated_term: &String, indexed_conclusion: Option<&IndexedConclusion>) {
        // TODO: handle error...
        let _ = Self::update_calendar(self.calendar_index_updater.calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_conclusion);
    }
}

#[derive(Debug)]
pub struct CalendarRelatedToIndexUpdater<'a> {
    pub calendar_index_updater: &'a mut CalendarIndexUpdater<'a>,
}

impl<'a> CalendarRelatedToIndexUpdater<'a> {

    pub fn new(calendar_index_updater: &'a mut CalendarIndexUpdater<'a>) -> Self {
        CalendarRelatedToIndexUpdater {
            calendar_index_updater,
        }
    }

    pub fn remove_event_from_calendar(&mut self, original_event: &Event) -> Result<bool, String> {
        let original_uuid = &original_event.uuid;
        let current_uuid = &self.calendar_index_updater.event_uuid;

        if original_uuid != current_uuid {
            return Err(format!("Cannot remove Event related_to from disconnected Calendars because of mismatched UUIDs - original: '{original_uuid}' expected: '{current_uuid}'"));
        }

        if let Some(indexed_related_to) = &original_event.indexed_related_to {
            for (related_to, _) in indexed_related_to.terms.iter() {
                // Update the calendar with None as indexed_conclusion so that it deletes the
                // related_to associated with the event which has now been removed from
                // the calendar.
                Self::update_calendar(self.calendar_index_updater.calendar, original_uuid.clone(), related_to.clone(), None)?;
            }
        }

        Ok(true)
    }

    fn update_calendar(calendar: &mut Calendar, event_uuid: String, updated_term: KeyValuePair, indexed_conclusion: Option<&IndexedConclusion>) -> Result<bool, String> {
        match indexed_conclusion {
            Some(indexed_conclusion) => {
                calendar.indexed_related_to.insert(event_uuid, updated_term, indexed_conclusion)?;
            },

            None => {
                calendar.indexed_related_to.remove(event_uuid, updated_term)?;
            }
        };

        Ok(true)
    }

    fn handle_update(&mut self, updated_term: &KeyValuePair, indexed_conclusion: Option<&IndexedConclusion>) {
        // TODO: handle error...
        let _ = Self::update_calendar(self.calendar_index_updater.calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_conclusion);
    }
}
