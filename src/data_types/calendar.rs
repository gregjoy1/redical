use serde::{Serialize, Deserialize};

use crate::data_types::inverted_index::{InvertedCalendarIndex, IndexedConclusion};

use crate::data_types::utils::KeyValuePair;

use crate::data_types::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uuid:               String,
    pub indexed_categories: InvertedCalendarIndex<String>,
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,
}

impl Calendar {

    pub fn new(uuid: String) -> Self {
        Calendar {
            uuid,
            indexed_categories: InvertedCalendarIndex::new(),
            indexed_related_to: InvertedCalendarIndex::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CalendarIndexUpdater {
    pub event_uuid:             String,
    pub connected_calendars:    Vec<Box<Calendar>>,
    pub disconnected_calendars: Vec<Box<Calendar>>,
}

impl CalendarIndexUpdater {

    pub fn new(event_uuid: String, connected_calendars: Vec<Box<Calendar>>, disconnected_calendars: Vec<Box<Calendar>>) -> Self {
        CalendarIndexUpdater {
            event_uuid,
            connected_calendars,
            disconnected_calendars,

        }
    }

    pub fn is_any_connected_calendars(&self) -> bool {
        self.connected_calendars.len() > 0
    }

    pub fn is_any_disconnected_calendars(&self) -> bool {
        self.disconnected_calendars.len() > 0
    }
}

#[derive(Debug)]
pub struct CalendarCategoryIndexUpdater<'a> {
    pub calendar_index_updater: &'a mut CalendarIndexUpdater,
}

impl<'a> CalendarCategoryIndexUpdater<'a> {

    pub fn new(calendar_index_updater: &'a mut CalendarIndexUpdater) -> Self {
        CalendarCategoryIndexUpdater {
            calendar_index_updater
        }
    }

    pub fn remove_event_from_calendar(&mut self, original_event: &Event) -> Result<bool, String> {
        let original_uuid = &original_event.uuid;
        let current_uuid = &self.calendar_index_updater.event_uuid;

        if original_uuid != current_uuid {
            return Err(format!("Cannot remove Event categories from disconnected Calendars because of mismatched UUIDs - original: '{original_uuid}' expected: '{current_uuid}'"));
        }

        if let Some(indexed_categories) = &original_event.indexed_categories {
            for (category, _) in indexed_categories.terms.iter() {
                for calendar in self.calendar_index_updater.disconnected_calendars.iter_mut() {
                    // Update all calendars with None as indexed_conclusion so that it deletes the
                    // categories associated with the event which has now been disconnected from
                    // the calendar(s).
                    Self::update_calendar(calendar, original_uuid.clone(), category.clone(), None)?;
                }
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
        for calendar in self.calendar_index_updater.connected_calendars.iter_mut() {
            // TODO: handle error...
            let _ = Self::update_calendar(calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_conclusion);
        }
    }
}

#[derive(Debug)]
pub struct CalendarRelatedToIndexUpdater<'a> {
    pub calendar_index_updater: &'a mut CalendarIndexUpdater,
}

impl<'a> CalendarRelatedToIndexUpdater<'a> {

    pub fn new(calendar_index_updater: &'a mut CalendarIndexUpdater) -> Self {
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
                for calendar in self.calendar_index_updater.disconnected_calendars.iter_mut() {
                    // Update all calendars with None as indexed_conclusion so that it deletes the
                    // related_to associated with the event which has now been disconnected from
                    // the calendar(s).
                    Self::update_calendar(calendar, original_uuid.clone(), related_to.clone(), None)?;
                }
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
        for calendar in self.calendar_index_updater.connected_calendars.iter_mut() {
            // TODO: handle error...
            let _ = Self::update_calendar(calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_conclusion);
        }
    }
}
