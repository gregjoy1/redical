use serde::{Serialize, Deserialize};

use crate::data_types::inverted_index::{InvertedCalendarIndex, InvertedCalendarIndexTerm, IndexedEvent, InvertedIndexListener};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uuid:               String,
    pub indexed_categories: InvertedCalendarIndex,
    pub indexed_related_to: InvertedCalendarIndex,
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
    pub event_uuid: String,
    pub calendars:  Vec<Box<Calendar>>,
}

impl CalendarIndexUpdater {

    pub fn new(event_uuid: String, calendars: Vec<Box<Calendar>>) -> Self {
        CalendarIndexUpdater {
            event_uuid,
            calendars,
        }
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

    fn update_calendar(calendar: &mut Calendar, event_uuid: String, updated_term: String, indexed_event: Option<&IndexedEvent>) -> Result<bool, String> {
        match indexed_event {
            Some(indexed_event) => {
                calendar.indexed_categories.insert(event_uuid, updated_term, indexed_event);
            },

            None => {
                calendar.indexed_categories.remove(event_uuid, updated_term);
            }
        };

        Ok(true)
    }
}

impl<'a> InvertedIndexListener for CalendarCategoryIndexUpdater<'a> {

    fn handle_update(&mut self, updated_term: &String, indexed_event: Option<&IndexedEvent>) {
        for calendar in self.calendar_index_updater.calendars.iter_mut() {
            Self::update_calendar(calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_event);
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
            calendar_index_updater
        }
    }

    fn update_calendar(calendar: &mut Calendar, event_uuid: String, updated_term: String, indexed_event: Option<&IndexedEvent>) -> Result<bool, String> {
        match indexed_event {
            Some(indexed_event) => {
                calendar.indexed_related_to.insert(event_uuid, updated_term, indexed_event);
            },

            None => {
                calendar.indexed_related_to.remove(event_uuid, updated_term);
            }
        };

        Ok(true)
    }
}

impl<'a> InvertedIndexListener for CalendarRelatedToIndexUpdater<'a> {

    fn handle_update(&mut self, updated_term: &String, indexed_event: Option<&IndexedEvent>) {
        for calendar in self.calendar_index_updater.calendars.iter_mut() {
            Self::update_calendar(calendar, self.calendar_index_updater.event_uuid.clone(), updated_term.clone(), indexed_event);
        }
    }
}
