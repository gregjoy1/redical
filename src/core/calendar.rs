use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::core::inverted_index::{IndexedConclusion, InvertedCalendarIndex};

use crate::core::utils::{KeyValuePair, UpdatedHashMapMembers};

use crate::core::geo_index::{GeoPoint, GeoSpatialCalendarIndex};

use crate::core::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uid: String,
    pub events: HashMap<String, Event>,
    pub indexed_categories: InvertedCalendarIndex<String>,
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,
    pub indexed_geo: GeoSpatialCalendarIndex,
    pub indexed_class: InvertedCalendarIndex<String>,
}

impl Calendar {
    pub fn new(uid: String) -> Self {
        Calendar {
            uid,
            events: HashMap::new(),
            indexed_categories: InvertedCalendarIndex::new(),
            indexed_related_to: InvertedCalendarIndex::new(),
            indexed_geo: GeoSpatialCalendarIndex::new(),
            indexed_class: InvertedCalendarIndex::new(),
        }
    }
}

#[derive(Debug)]
pub struct CalendarIndexUpdater<'a> {
    pub event_uid: String,
    pub calendar: &'a mut Calendar,
}

impl<'a> CalendarIndexUpdater<'a> {
    pub fn new(event_uid: String, calendar: &'a mut Calendar) -> Self {
        CalendarIndexUpdater {
            event_uid,
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
