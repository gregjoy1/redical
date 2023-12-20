use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use crate::core::inverted_index::{IndexedConclusion, InvertedCalendarIndex};

use crate::core::utils::{KeyValuePair, UpdatedAttribute, UpdatedHashMapMembers};

use crate::core::geo_index::{GeoPoint, GeoSpatialCalendarIndex};

use crate::core::event::Event;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Calendar {
    pub uuid: String,
    pub events: HashMap<String, Event>,
    pub indexed_categories: InvertedCalendarIndex<String>,
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,
    pub indexed_geo: GeoSpatialCalendarIndex,
}

impl Calendar {
    pub fn new(uuid: String) -> Self {
        Calendar {
            uuid,
            events: HashMap::new(),
            indexed_categories: InvertedCalendarIndex::new(),
            indexed_related_to: InvertedCalendarIndex::new(),
            indexed_geo: GeoSpatialCalendarIndex::new(),
        }
    }
}

#[derive(Debug)]
pub struct CalendarIndexUpdater<'a> {
    pub event_uuid: String,
    pub calendar: &'a mut Calendar,
}

impl<'a> CalendarIndexUpdater<'a> {
    pub fn new(event_uuid: String, calendar: &'a mut Calendar) -> Self {
        CalendarIndexUpdater {
            event_uuid,
            calendar,
        }
    }

    pub fn update_indexed_categories(
        &mut self,
        updated_event_categories_diff: &UpdatedHashMapMembers<String, IndexedConclusion>,
    ) -> Result<bool, String> {
        let indexed_categories = &mut self.calendar.indexed_categories;

        for (removed_category, _) in updated_event_categories_diff.removed.iter() {
            indexed_categories.remove(self.event_uuid.clone(), removed_category.clone())?;
        }

        for (updated_category, updated_indexed_conclusion) in
            updated_event_categories_diff.updated.iter()
        {
            indexed_categories.insert(
                self.event_uuid.clone(),
                updated_category.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_category, added_indexed_conclusion) in updated_event_categories_diff.added.iter()
        {
            indexed_categories.insert(
                self.event_uuid.clone(),
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
            indexed_related_to.remove(self.event_uuid.clone(), removed_related_to.clone())?;
        }

        for (updated_related_to, updated_indexed_conclusion) in
            updated_event_related_to_diff.updated.iter()
        {
            indexed_related_to.insert(
                self.event_uuid.clone(),
                updated_related_to.clone(),
                updated_indexed_conclusion,
            )?;
        }

        for (added_related_to, added_indexed_conclusion) in
            updated_event_related_to_diff.added.iter()
        {
            indexed_related_to.insert(
                self.event_uuid.clone(),
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
            indexed_geo.remove(self.event_uuid.clone(), removed_long_lat_coord)?;
        }

        for (updated_long_lat_coord, updated_indexed_conclusion) in
            updated_event_geo_diff.updated.iter()
        {
            indexed_geo.insert(
                self.event_uuid.clone(),
                updated_long_lat_coord,
                updated_indexed_conclusion,
            )?;
        }

        for (added_long_lat_coord, added_indexed_conclusion) in updated_event_geo_diff.added.iter()
        {
            indexed_geo.insert(
                self.event_uuid.clone(),
                added_long_lat_coord,
                added_indexed_conclusion,
            )?;
        }

        Ok(true)
    }
}
