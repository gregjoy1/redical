use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::str::FromStr;

use rrule::{RRuleError, RRuleSet};

use redical_ical::{
    ICalendarComponent,
    ICalendarEntity,
    RenderingContext,
    content_line::ContentLine,
    properties::{
        ICalendarProperty,
        ICalendarDateTimeProperty,
        EventProperty,
        EventProperties,
        UIDProperty,
        LastModifiedProperty,
        RRuleProperty,
        ExRuleProperty,
        RDateProperty,
        ExDateProperty,
        DTStartProperty,
        DTEndProperty,
        DurationProperty,
        CategoriesProperty,
        LocationTypeProperty,
        RelatedToProperty,
        ClassProperty,
        GeoProperty,
        PassiveProperty,
    },
};

use crate::event_occurrence_override::EventOccurrenceOverride;

use crate::inverted_index::InvertedEventIndex;

use crate::geo_index::GeoPoint;

use crate::event_diff::EventDiff;

use crate::utils::KeyValuePair;

// Rebase all overrides with added/removed EventDiff properties.
//
// This is when an existing event with overrides is updated, and we want to update all the base
// categories/related_to/properties on each one.
//
// If we don't do this, then when we update an existing event to have an additional category,
// each overridden occurrence with not include that category.
//
// TODO: Look into storing diffs in the overrides as opposed to the current state of all overridden properties.
pub fn rebase_overrides(
    overrides: &mut BTreeMap<i64, EventOccurrenceOverride>,
    event_diff: &EventDiff,
) -> Result<(), String> {
    /*
     * TODO: come back to this:
    for (_timestamp, event_occurrence_override) in overrides.iter_mut() {
        rebase_override(event_occurrence_override, event_diff);
    }
    */

    Ok(())
}

/*
 * TODO: Come back to this and add class and geo properties...
 *
// Rebase specified override with added/removed EventDiff properties.
fn rebase_override(
    event_occurrence_override: &mut EventOccurrenceOverride,
    event_diff: &EventDiff,
) {
    if let Some(indexed_categories) = &event_diff.indexed_categories {
        match event_occurrence_override.categories.as_mut() {
            Some(overridden_categories) => {
                for removed_category in indexed_categories.removed.iter() {
                    overridden_categories.remove(removed_category);
                }

                for added_category in indexed_categories.added.iter() {
                    overridden_categories.insert(added_category.clone());
                }
            }

            None => {
                event_occurrence_override.categories = Some(indexed_categories.added.clone());
            }
        };
    }

    if let Some(indexed_related_to) = &event_diff.indexed_related_to {
        match event_occurrence_override.related_to.as_mut() {
            Some(overridden_related_to) => {
                for removed_reltype_uid_pair in indexed_related_to.removed.iter() {
                    if let Some(reltype_uids) =
                        overridden_related_to.get_mut(&removed_reltype_uid_pair.key)
                    {
                        reltype_uids.remove(&removed_reltype_uid_pair.value);
                    }
                }

                for added_reltype_uid_pair in indexed_related_to.added.iter() {
                    overridden_related_to
                        .entry(added_reltype_uid_pair.key.clone())
                        .and_modify(|reltype_uids| {
                            reltype_uids.insert(added_reltype_uid_pair.value.clone());
                        })
                        .or_insert(HashSet::from([added_reltype_uid_pair.value.clone()]));
                }
            }

            None => {
                let mut overridden_related_to = HashMap::new();

                for added_reltype_uid_pair in indexed_related_to.added.iter() {
                    overridden_related_to
                        .entry(added_reltype_uid_pair.key.clone())
                        .and_modify(|reltype_uids: &mut HashSet<String>| {
                            reltype_uids.insert(added_reltype_uid_pair.value.clone());
                        })
                        .or_insert(HashSet::from([added_reltype_uid_pair.value.clone()]));
                }

                event_occurrence_override.related_to = Some(overridden_related_to);
            }
        };
    }

    if let Some(indexed_passive_properties) = &event_diff.passive_properties {
        match event_occurrence_override.properties.as_mut() {
            Some(overridden_passive_properties) => {
                for removed_property_pair in indexed_passive_properties.removed.iter() {
                    overridden_passive_properties.remove(&removed_property_pair);
                }

                for added_property_pair in indexed_passive_properties.added.iter() {
                    overridden_passive_properties.insert(added_property_pair.clone());
                }
            }

            None => {
                event_occurrence_override.properties = Some(BTreeSet::from_iter(
                    indexed_passive_properties
                        .added
                        .iter()
                        .map(|added_key_value_pair| added_key_value_pair.clone()),
                ));
            }
        };
    }
}
*/

#[derive(Debug, PartialEq, Clone)]
pub struct ScheduleProperties {
    pub rrule: Option<RRuleProperty>,
    pub exrule: Option<ExRuleProperty>,
    pub rdates: Option<HashSet<RDateProperty>>,
    pub exdates: Option<HashSet<ExDateProperty>>,
    pub duration: Option<DurationProperty>,
    pub dtstart: Option<DTStartProperty>,
    pub dtend: Option<DTEndProperty>,
    pub parsed_rrule_set: Option<rrule::RRuleSet>,
}

impl ScheduleProperties {
    pub fn new() -> ScheduleProperties {
        ScheduleProperties {
            rrule: None,
            exrule: None,
            rdates: None,
            exdates: None,
            duration: None,
            dtstart: None,
            dtend: None,
            parsed_rrule_set: None,
        }
    }

    pub fn extract_serialized_rrule_ical_key_value_pair(&self) -> Option<KeyValuePair> {
        self.rrule
            .as_ref()
            .and_then(|property| Some(property.to_content_line().into()))
    }

    pub fn extract_serialized_exrule_ical_key_value_pair(&self) -> Option<KeyValuePair> {
        self.exrule
            .as_ref()
            .and_then(|property| Some(property.to_content_line().into()))
    }

    pub fn extract_serialized_rdates_ical_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.rdates.as_ref().and_then(|properties| {
            let mut key_value_pairs = HashSet::new();

            for property in properties {
                key_value_pairs.insert(property.to_content_line().into());
            }

            Some(key_value_pairs)
        })
    }

    pub fn extract_serialized_exdates_ical_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.exdates.as_ref().and_then(|properties| {
            let mut key_value_pairs = HashSet::new();

            for property in properties {
                key_value_pairs.insert(property.to_content_line().into());
            }

            Some(key_value_pairs)
        })
    }

    pub fn extract_serialized_duration_ical_key_value_pair(&self) -> Option<KeyValuePair> {
        self.duration
            .as_ref()
            .and_then(|property| Some(property.to_content_line().into()))
    }

    pub fn extract_serialized_dtstart_ical_key_value_pair(&self) -> Option<KeyValuePair> {
        self.dtstart
            .as_ref()
            .and_then(|property| Some(property.to_content_line().into()))
    }

    pub fn extract_serialized_dtend_ical_key_value_pair(&self) -> Option<KeyValuePair> {
        self.dtend
            .as_ref()
            .and_then(|property| Some(property.to_content_line().into()))
    }

    pub fn insert(&mut self, property: EventProperty) -> Result<&Self, String> {
        match property {
            EventProperty::RRule(property) => { self.rrule = Some(property); },
            EventProperty::ExRule(property) => { self.exrule = Some(property); },
            EventProperty::DTStart(property) => { self.dtstart = Some(property); },
            EventProperty::DTEnd(property) => { self.dtend = Some(property); },

            EventProperty::RDate(property) => {
                match &mut self.rdates {
                    Some(rdates) => { rdates.insert(property); },
                    None => { self.rdates = Some(HashSet::from([property])); }
                }
            },

            EventProperty::ExDate(property) => {
                match &mut self.exdates {
                    Some(exdates) => { exdates.insert(property); },
                    None => { self.exdates = Some(HashSet::from([property])); }
                }
            },

            EventProperty::Duration(property) => {
                self.duration = Some(property);
            },

            _ => {
                return Err(format!("Expected schedule property (RRULE, EXRULE, RDATE, EXDATE, DURATION, DTSTART, DTEND), received: {:#?}", property))
            }
        }

        Ok(self)
    }

    pub fn parse_rrule(&self) -> Result<RRuleSet, RRuleError> {
        let mut ical_parts = vec![];

        if let Some(rrule) = &self.rrule {
            ical_parts.push(rrule.render_ical());
        }

        if let Some(exrule) = &self.exrule {
            ical_parts.push(exrule.render_ical());
        }

        if let Some(rdates) = &self.rdates {
            rdates.iter().for_each(|rdates| {
                ical_parts.push(rdates.render_ical());
            });
        }

        if let Some(exdates) = &self.exdates {
            exdates.iter().for_each(|exdates| {
                ical_parts.push(exdates.render_ical());
            });
        }

        if let Some(dtstart) = &self.dtstart {
            ical_parts.push(dtstart.render_ical());

            // If parsed ical does not contain any RRULE or RDATE properties, we need to
            // artifically create them based on the specified DTSTART properties so that the
            // rrule_set date extrapolation works, even for a single date.
            //
            // NOTE: This has to be a single recurring RRULE instead of RDATE because the rrule
            // crate does not serialize the rdatess -
            // https://github.com/fmeringdal/rust-rrule/blob/main/rrule/src/core/rruleset.rs#L264
            //
            // This means that when the Calendar struct is persisted to disk as a string, it panics
            // when rehydrated with the following error:
            //
            // "RRule parsing error: Missing date generation property. There needs to be at least
            // one `RRULE` or `RDATE` to generate occurrences."
            if self.rrule.is_none() && (self.rdates.is_none() || self.rdates.as_ref().is_some_and(|rdates| rdates.is_empty())) {
                ical_parts.push(RDateProperty::new_from(dtstart).render_ical());
            }
        }

        ical_parts.join("\n").parse::<RRuleSet>()
    }

    pub fn get_dtstart_timestamp(&self) -> Option<i64> {
        self.dtstart
            .as_ref()
            .and_then(|dtstart| Some(dtstart.get_utc_timestamp()))
    }

    pub fn get_dtend_timestamp(&self) -> Option<i64> {
        self.dtend
            .as_ref()
            .and_then(|dtend| Some(dtend.get_utc_timestamp()))
    }

    pub fn get_duration_in_seconds(&self) -> Option<i64> {
        if let Some(parsed_duration) = self.duration.as_ref() {
            return Some(parsed_duration.duration.get_duration_in_seconds());
        }

        match (self.get_dtstart_timestamp(), self.get_dtend_timestamp()) {
            (Some(dtstart_timestamp), Some(dtend_timestamp)) => {
                Some(dtend_timestamp - dtstart_timestamp)
            }

            _ => None,
        }
    }

    pub fn build_parsed_rrule_set(&mut self) -> Result<(), rrule::RRuleError> {
        let parsed_rrule_set = self.parse_rrule()?;

        self.parsed_rrule_set = Some(parsed_rrule_set);

        Ok(())
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct IndexedProperties {
    pub geo: Option<GeoProperty>,
    pub related_to: Option<HashSet<RelatedToProperty>>,
    pub categories: Option<HashSet<CategoriesProperty>>,
    pub location_type: Option<LocationTypeProperty>,
    pub class: Option<ClassProperty>,
}

impl IndexedProperties {
    pub fn new() -> IndexedProperties {
        IndexedProperties {
            geo: None,
            related_to: None,
            categories: None,
            location_type: None,
            class: None,
        }
    }

    pub fn extract_all_location_type_strings(&self) -> Option<HashSet<String>> {
        self.location_type.as_ref().and_then(|location_type_property| {
            let mut location_types: HashSet<String> = HashSet::new();

            for location_type in &location_type_property.types {
                location_types.insert(location_type.to_string());
            }

            Some(location_types)
        })
    }

    pub fn extract_all_category_strings(&self) -> Option<HashSet<String>> {
        self.categories.as_ref().and_then(|categories_properties| {
            let mut categories: HashSet<String> = HashSet::new();

            for categories_property in categories_properties {
                for category in &categories_property.categories {
                    categories.insert(category.to_string());
                }
            }

            Some(categories)
        })
    }

    pub fn extract_all_related_to_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.related_to.as_ref().and_then(|related_to_properties| {
            let mut related_to_key_value_pairs: HashSet<KeyValuePair> = HashSet::new();

            for related_to_property in related_to_properties {
                related_to_key_value_pairs.insert(related_to_property.to_reltype_uid_pair().into());
            }

            Some(related_to_key_value_pairs)
        })
    }

    pub fn extract_all_related_to_key_value_map(&self) -> Option<HashMap<String, HashSet<String>>> {
        self.related_to.as_ref().and_then(|related_to_properties| {
            let mut related_to_map = HashMap::new();

            for related_to_property in related_to_properties {
                related_to_map
                    .entry(related_to_property.get_reltype().to_string())
                    .and_modify(|uid_set: &mut HashSet<String>| {
                        uid_set.insert(related_to_property.uid.to_string());
                    })
                    .or_insert(HashSet::from([related_to_property.uid.to_string()]));
            }

            Some(related_to_map)
        })
    }

    pub fn extract_geo_point(&self) -> Option<GeoPoint> {
        self.geo
            .as_ref()
            .and_then(|geo_property| Some(GeoPoint::from(geo_property)))
    }

    pub fn extract_class(&self) -> Option<String> {
        self.class
            .as_ref()
            .and_then(|class_property| Some(class_property.class.to_string()))
    }

    pub fn insert(&mut self, property: EventProperty) -> Result<&Self, String> {
        match property {
            EventProperty::Class(property) => {
                self.class = Some(property);
            }

            EventProperty::Geo(property) => {
                self.geo = Some(property);
            }

            EventProperty::Categories(property) => {
                self.categories
                    .get_or_insert(HashSet::new())
                    .insert(property);
            }

            EventProperty::LocationType(property) => {
                self.location_type = Some(property);
            }

            EventProperty::RelatedTo(property) => {
                self.related_to
                    .get_or_insert(HashSet::new())
                    .insert(property);
            }

            _ => {
                return Err(format!(
                    "Expected indexable property (CATEGORIES, RELATED_TO), received: {}",
                    property.render_ical()
                ));
            }
        };

        Ok(self)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct PassiveProperties {
    pub properties: BTreeSet<PassiveProperty>,
}

impl PassiveProperties {
    pub fn new() -> PassiveProperties {
        PassiveProperties {
            properties: BTreeSet::new(),
        }
    }

    pub fn extract_properties_serialized_ical_key_value_pairs(&self) -> HashSet<KeyValuePair> {
        let mut key_value_pairs = HashSet::new();

        for property in &self.properties {
            key_value_pairs.insert(property.to_content_line().into());
        }

        key_value_pairs
    }

    pub fn insert(&mut self, property: EventProperty) -> Result<&Self, String> {
        match property {
            EventProperty::UID(_)
            | EventProperty::LastModified(_)
            | EventProperty::Class(_)
            | EventProperty::Geo(_)
            | EventProperty::Categories(_)
            | EventProperty::LocationType(_)
            | EventProperty::RelatedTo(_)
            | EventProperty::RRule(_)
            | EventProperty::ExRule(_)
            | EventProperty::DTStart(_)
            | EventProperty::DTEnd(_)
            | EventProperty::RDate(_)
            | EventProperty::ExDate(_)
            | EventProperty::Duration(_) => {
                return Err(format!(
                    "Expected passive property, received: {}",
                    property.render_ical()
                ));
            }

            EventProperty::Passive(passive_property) => {
                self.properties.insert(passive_property);
            }
        };

        Ok(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Event {
    pub uid: UIDProperty,
    pub last_modified: LastModifiedProperty,

    pub schedule_properties: ScheduleProperties,
    pub indexed_properties: IndexedProperties,

    pub passive_properties: PassiveProperties,

    pub overrides: BTreeMap<i64, EventOccurrenceOverride>,
    pub indexed_categories: Option<InvertedEventIndex<String>>,
    pub indexed_related_to: Option<InvertedEventIndex<KeyValuePair>>,
    pub indexed_geo: Option<InvertedEventIndex<GeoPoint>>,
    pub indexed_class: Option<InvertedEventIndex<String>>,
}

impl Event {
    pub fn new(uid: String) -> Event {
        Event {
            uid: UIDProperty::from(uid),
            last_modified: LastModifiedProperty::new_from_now(false),

            schedule_properties: ScheduleProperties::new(),
            indexed_properties: IndexedProperties::new(),

            passive_properties: PassiveProperties::new(),

            overrides: BTreeMap::new(),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo: None,
            indexed_class: None,
        }
    }

    pub fn validate(&mut self) -> Result<bool, String> {
        self
            .schedule_properties
            .build_parsed_rrule_set()
            .map_err(|error| error.to_string())?;

        Ok(true)
    }

    pub fn rebuild_indexes(&mut self) -> Result<bool, String> {
        self.rebuild_indexed_categories()?;
        self.rebuild_indexed_related_to()?;
        self.rebuild_indexed_geo()?;
        self.rebuild_indexed_class()?;

        Ok(true)
    }

    pub fn parse_ical(uid: &str, input: &str) -> Result<Event, String> {
        EventProperties::from_str(input).and_then(|EventProperties(parsed_properties)| {
            let mut new_event = Event::new(String::from(uid));

            for parsed_property in parsed_properties {
                new_event.insert(parsed_property)?;
            }

            Ok(new_event)
        })
    }

    pub fn insert(&mut self, property: EventProperty) -> Result<&Self, String> {
        match property {
            EventProperty::UID(property) => {
                if self.uid != property {
                    return Err(
                        format!("Inserted event UID: {} does not match existing UID: {}", property.uid.to_string(), self.uid.uid.to_string())
                    );
                }
            },

            EventProperty::LastModified(property) => {
                self.last_modified = property;
            },

            EventProperty::Class(_)
            | EventProperty::Geo(_)
            | EventProperty::Categories(_)
            | EventProperty::LocationType(_)
            | EventProperty::RelatedTo(_) => {
                self.indexed_properties.insert(property)?;
            }

            EventProperty::RRule(_)
            | EventProperty::ExRule(_)
            | EventProperty::DTStart(_)
            | EventProperty::DTEnd(_)
            | EventProperty::RDate(_)
            | EventProperty::ExDate(_)
            | EventProperty::Duration(_) => {
                self.schedule_properties.insert(property)?;
            }

            _ => {
                self.passive_properties.insert(property)?;
            }
        }

        Ok(self)
    }

    pub fn rebuild_indexed_categories(&mut self) -> Result<&mut Self, String> {
        self.indexed_categories = Some(InvertedEventIndex::<String>::new_from_event_categories(
            self,
        ));

        Ok(self)
    }

    pub fn rebuild_indexed_related_to(&mut self) -> Result<&mut Self, String> {
        self.indexed_related_to =
            Some(InvertedEventIndex::<KeyValuePair>::new_from_event_related_to(self));

        Ok(self)
    }

    // TODO: Add tests...
    pub fn rebuild_indexed_geo(&mut self) -> Result<&mut Self, String> {
        self.indexed_geo = Some(InvertedEventIndex::<GeoPoint>::new_from_event_geo(self));

        Ok(self)
    }

    // TODO: Add tests...
    pub fn rebuild_indexed_class(&mut self) -> Result<&mut Self, String> {
        self.indexed_class = Some(InvertedEventIndex::<String>::new_from_event_class(self));

        Ok(self)
    }

    pub fn override_occurrence(
        &mut self,
        event_occurrence_override: &EventOccurrenceOverride,
        update_indexes: bool,
    ) -> Result<bool, String> {
        let Some(timestamp) = event_occurrence_override.get_dtstart_timestamp() else {
            return Err(String::from("Expected event occurrence override to have dtstart defined."));
        };

        self.overrides
            .insert(timestamp, event_occurrence_override.clone());

        // Only proceed with updating the indexes of the event if required.
        if update_indexes == false {
            return Ok(true);
        }

        if let Some(ref mut indexed_categories) = self.indexed_categories {
            if let Some(overridden_categories) = &event_occurrence_override
                .indexed_properties
                .extract_all_category_strings()
            {
                indexed_categories.insert_override(timestamp, overridden_categories);
            }
        } else {
            self.rebuild_indexed_categories()?;
        }

        if let Some(ref mut indexed_related_to) = self.indexed_related_to {
            if let Some(overridden_related_to_set) = &event_occurrence_override
                .indexed_properties
                .extract_all_related_to_key_value_pairs()
            {
                indexed_related_to.insert_override(timestamp, overridden_related_to_set);
            }
        } else {
            self.rebuild_indexed_related_to()?;
        }

        if let Some(ref mut indexed_geo) = self.indexed_geo {
            if let Some(overridden_geo) = &event_occurrence_override
                .indexed_properties
                .extract_geo_point()
            {
                indexed_geo.insert_override(timestamp, &HashSet::from([overridden_geo.clone()]));
            }
        } else {
            self.rebuild_indexed_geo()?;
        }

        if let Some(ref mut indexed_class) = self.indexed_class {
            if let Some(overridden_class) =
                &event_occurrence_override.indexed_properties.extract_class()
            {
                indexed_class
                    .insert_override(timestamp, &HashSet::from([overridden_class.clone()]));
            }
        } else {
            self.rebuild_indexed_class()?;
        }

        Ok(true)
    }

    pub fn remove_occurrence_override(&mut self, timestamp: i64, update_indexes: bool) -> Result<bool, String> {
        let override_removed = self.overrides.remove(&timestamp).is_some();

        // Only proceed with updating the indexes of the event if required.
        if update_indexes == false {
            return Ok(override_removed);
        }

        if let Some(ref mut indexed_categories) = self.indexed_categories {
            indexed_categories.remove_override(timestamp);
        } else {
            self.indexed_categories = Some(
                InvertedEventIndex::<String>::new_from_event_categories(&*self),
            );
        }

        if let Some(ref mut indexed_related_to) = self.indexed_related_to {
            indexed_related_to.remove_override(timestamp);
        } else {
            self.indexed_related_to =
                Some(InvertedEventIndex::<KeyValuePair>::new_from_event_related_to(&*self));
        }

        if let Some(ref mut indexed_geo) = self.indexed_geo {
            indexed_geo.remove_override(timestamp);
        } else {
            self.indexed_geo =
                Some(InvertedEventIndex::<GeoPoint>::new_from_event_geo(&*self));
        }

        if let Some(ref mut indexed_class) = self.indexed_class {
            indexed_class.remove_override(timestamp);
        } else {
            self.indexed_class =
                Some(InvertedEventIndex::<String>::new_from_event_class(&*self));
        }

        Ok(override_removed)
    }
}

impl ICalendarComponent for Event {
    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine> {
        let mut serializable_properties: BTreeSet<ContentLine> = BTreeSet::new();

        serializable_properties.insert(self.uid.to_content_line_with_context(context));
        serializable_properties.insert(self.last_modified.to_content_line_with_context(context));

        if let Some(rrule_property) = &self.schedule_properties.rrule {
            serializable_properties.insert(rrule_property.to_content_line_with_context(context));
        }

        if let Some(exrule_property) = &self.schedule_properties.exrule {
            serializable_properties.insert(exrule_property.to_content_line_with_context(context));
        }

        if let Some(rdates_properties) = &self.schedule_properties.rdates {
            for rdate_property in rdates_properties {
                serializable_properties.insert(rdate_property.to_content_line_with_context(context));
            }
        }

        if let Some(exdates_properties) = &self.schedule_properties.exdates {
            for exdate_property in exdates_properties {
                serializable_properties.insert(exdate_property.to_content_line_with_context(context));
            }
        }

        if let Some(duration_property) = &self.schedule_properties.duration {
            serializable_properties.insert(duration_property.to_content_line_with_context(context));
        }

        if let Some(dtstart_property) = &self.schedule_properties.dtstart {
            serializable_properties.insert(dtstart_property.to_content_line_with_context(context));
        }

        if let Some(dtend_property) = &self.schedule_properties.dtend {
            serializable_properties.insert(dtend_property.to_content_line_with_context(context));
        }

        if let Some(geo_property) = &self.indexed_properties.geo {
            serializable_properties.insert(geo_property.to_content_line_with_context(context));
        }

        if let Some(location_type_property) = &self.indexed_properties.location_type {
            serializable_properties.insert(location_type_property.to_content_line_with_context(context));
        }

        if let Some(class_property) = &self.indexed_properties.class {
            serializable_properties.insert(class_property.to_content_line_with_context(context));
        }

        if let Some(related_to_properties) = &self.indexed_properties.related_to {
            for related_to_property in related_to_properties {
                serializable_properties.insert(related_to_property.to_content_line_with_context(context));
            }
        }

        if let Some(categories_properties) = &self.indexed_properties.categories {
            for categories_property in categories_properties {
                serializable_properties.insert(categories_property.to_content_line_with_context(context));
            }
        }

        for passive_property in &self.passive_properties.properties {
            serializable_properties.insert(passive_property.to_content_line_with_context(context));
        }

        serializable_properties
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::IndexedConclusion;

    use crate::testing::macros::build_property_from_ical;

    use std::collections::BTreeMap;

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_indexed_categories() {
        let event = Event {
            uid: String::from("event_UID").into(),
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

            schedule_properties: ScheduleProperties {
                rrule: None,
                exrule: None,
                rdates: None,
                exdates: None,
                duration: None,
                dtstart: None,
                dtend: None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE"
                )])),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::new(),
            },

            overrides: BTreeMap::from([
                // Override 100 has all event categories plus CATEGORY_FOUR
                (
                    100,
                    EventOccurrenceOverride {
                        last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                        indexed_properties: IndexedProperties {
                            geo: None,
                            related_to: None,
                            location_type: None,
                            categories: Some(HashSet::from([build_property_from_ical!(
                                CategoriesProperty,
                                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE,CATEGORY_FOUR"
                            )])),
                            class: None,
                        },
                        passive_properties: PassiveProperties::new(),
                        dtstart: None,
                        dtend: None,
                        duration: None,
                    },
                ),
                // Override 200 has only some event categories (missing CATEGORY_THREE)
                (
                    200,
                    EventOccurrenceOverride {
                        last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                        indexed_properties: IndexedProperties {
                            geo: None,
                            related_to: None,
                            location_type: None,
                            categories: Some(HashSet::from([build_property_from_ical!(
                                CategoriesProperty,
                                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO"
                            )])),
                            class: None,
                        },
                        passive_properties: PassiveProperties::new(),
                        dtstart: None,
                        dtend: None,
                        duration: None,
                    },
                ),
                // Override 300 has no overridden categories
                (
                    300,
                    EventOccurrenceOverride {
                        last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                        indexed_properties: IndexedProperties::new(),
                        passive_properties: PassiveProperties::new(),
                        dtstart: None,
                        dtend: None,
                        duration: None,
                    },
                ),
                // Override 400 has removed all categories
                (
                    400,
                    EventOccurrenceOverride {
                        last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                        indexed_properties: IndexedProperties {
                            geo: None,
                            related_to: None,
                            location_type: None,
                            categories: Some(HashSet::new()),
                            class: None,
                        },
                        passive_properties: PassiveProperties::new(),
                        dtstart: None,
                        dtend: None,
                        duration: None,
                    },
                ),
                // Override 500 has no base event categories, but does have CATEGORY_FOUR
                (
                    500,
                    EventOccurrenceOverride {
                        last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                        indexed_properties: IndexedProperties {
                            geo: None,
                            related_to: None,
                            location_type: None,
                            categories: Some(HashSet::from([build_property_from_ical!(
                                CategoriesProperty,
                                "CATEGORIES:CATEGORY_FOUR"
                            )])),
                            class: None,
                        },
                        passive_properties: PassiveProperties::new(),
                        dtstart: None,
                        dtend: None,
                        duration: None,
                    },
                ),
            ]),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo: None,
            indexed_class: None,
        };

        let mut indexed_categories =
            InvertedEventIndex::<String>::new_from_event_categories(&event);

        assert_eq!(
            indexed_categories,
            InvertedEventIndex {
                terms: HashMap::from([
                    (
                        String::from("CATEGORY_ONE"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500]))),
                    ),
                    (
                        String::from("CATEGORY_TWO"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500]))),
                    ),
                    (
                        String::from("CATEGORY_THREE"),
                        IndexedConclusion::Include(Some(HashSet::from([200, 400, 500]))),
                    ),
                    (
                        String::from("CATEGORY_FOUR"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 500]))),
                    ),
                ])
            }
        );

        indexed_categories.insert_override(
            600,
            &HashSet::from([String::from("CATEGORY_ONE"), String::from("CATEGORY_FIVE")]),
        );

        assert_eq!(
            indexed_categories,
            InvertedEventIndex {
                terms: HashMap::from([
                    (
                        String::from("CATEGORY_ONE"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500]))),
                    ),
                    (
                        String::from("CATEGORY_TWO"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500, 600]))),
                    ),
                    (
                        String::from("CATEGORY_THREE"),
                        IndexedConclusion::Include(Some(HashSet::from([200, 400, 500, 600]))),
                    ),
                    (
                        String::from("CATEGORY_FOUR"),
                        IndexedConclusion::Exclude(Some(HashSet::from([100, 500]))),
                    ),
                    (
                        String::from("CATEGORY_FIVE"),
                        IndexedConclusion::Exclude(Some(HashSet::from([600]))),
                    ),
                ])
            }
        );

        indexed_categories.remove_override(100);

        assert_eq!(
            indexed_categories,
            InvertedEventIndex {
                terms: HashMap::from([
                    (
                        String::from("CATEGORY_ONE"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500]))),
                    ),
                    (
                        String::from("CATEGORY_TWO"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 500, 600]))),
                    ),
                    (
                        String::from("CATEGORY_THREE"),
                        IndexedConclusion::Include(Some(HashSet::from([200, 400, 500, 600]))),
                    ),
                    (
                        String::from("CATEGORY_FOUR"),
                        IndexedConclusion::Exclude(Some(HashSet::from([500]))),
                    ),
                    (
                        String::from("CATEGORY_FIVE"),
                        IndexedConclusion::Exclude(Some(HashSet::from([600]))),
                    ),
                ])
            }
        );

        indexed_categories.remove_override(500);

        assert_eq!(
            indexed_categories,
            InvertedEventIndex {
                terms: HashMap::from([
                    (
                        String::from("CATEGORY_ONE"),
                        IndexedConclusion::Include(Some(HashSet::from([400]))),
                    ),
                    (
                        String::from("CATEGORY_TWO"),
                        IndexedConclusion::Include(Some(HashSet::from([400, 600]))),
                    ),
                    (
                        String::from("CATEGORY_THREE"),
                        IndexedConclusion::Include(Some(HashSet::from([200, 400, 600]))),
                    ),
                    (
                        String::from("CATEGORY_FIVE"),
                        IndexedConclusion::Exclude(Some(HashSet::from([600]))),
                    ),
                ])
            }
        );
    }

    #[test]
    fn test_parse_ical() {
        let ical: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY (THREE)\" LAST-MODIFIED:20201230T173000Z";

        assert_eq!(
            Event::parse_ical("event_UID", ical).unwrap(),
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(RRuleProperty, "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: None,
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties {
                    geo: None,
                    class: None,
                    location_type: None,
                    categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY (THREE)\"")])),
                    related_to: None,
                },

                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([build_property_from_ical!(PassiveProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA")]),
                },

                overrides: BTreeMap::new(),
                indexed_categories: None,
                indexed_related_to: None,
                indexed_geo: None,
                indexed_class: None,
            }
        );
    }

    #[test]
    fn test_build_parsed_rrule_set() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH DTSTART:16010101T020000 LAST-MODIFIED:20201230T173000Z";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(
                        RRuleProperty,
                        "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
                    )),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: Some(build_property_from_ical!(
                        DTStartProperty,
                        "DTSTART:16010101T020000"
                    )),
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties::new(),

                passive_properties: PassiveProperties::new(),

                overrides: BTreeMap::new(),
                indexed_categories: None,
                indexed_related_to: None,
                indexed_geo: None,
                indexed_class: None,
            }
        );

        assert!(parsed_event
            .schedule_properties
            .build_parsed_rrule_set()
            .is_ok());

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH LAST-MODIFIED:20201230T173000Z";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(
                        RRuleProperty,
                        "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
                    )),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: None,
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties::new(),

                passive_properties: PassiveProperties::new(),

                overrides: BTreeMap::new(),
                indexed_categories: None,
                indexed_geo: None,
                indexed_related_to: None,
                indexed_class: None,
            }
        );

        assert!(parsed_event
            .schedule_properties
            .build_parsed_rrule_set()
            .is_err());
    }

    #[test]
    fn test_occurrence_override_insertion_and_deletion() {
        let ical: &str =
            "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU DTSTART:20201231T183000Z LAST-MODIFIED:20201230T173000Z";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        let event_occurrence_override = EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")])),
                class: None,
            },
            passive_properties: PassiveProperties {
                properties: BTreeSet::from([build_property_from_ical!(PassiveProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA")]),
            },
            dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:20210112T183000Z")),
            dtend: None,
            duration: None,
        };

        assert_eq!(
            parsed_event.override_occurrence(&event_occurrence_override, true),
            Ok(
                true,
            )
        );

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(RRuleProperty, "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:20201231T183000Z")),
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties::new(),

                passive_properties: PassiveProperties::new(),

                overrides: BTreeMap::from([
                    (
                        1610476200,
                        EventOccurrenceOverride {
                            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
                            indexed_properties: IndexedProperties {
                                geo: None,
                                related_to: None,
                                location_type: None,
                                categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")])),
                                class: None,
                            },
                            passive_properties: PassiveProperties {
                                properties: BTreeSet::from([build_property_from_ical!(PassiveProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA")]),
                            },
                            dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:20210112T183000Z")),
                            dtend: None,
                            duration: None,
                        }
                    )
                ]),
                indexed_categories:  Some(
                    InvertedEventIndex {
                        terms: HashMap::from([
                                        (
                                            String::from("CATEGORY_ONE"),
                                            IndexedConclusion::Exclude(Some(HashSet::from([1610476200]))),
                                        ),
                                        (
                                            String::from("CATEGORY_TWO"),
                                            IndexedConclusion::Exclude(Some(HashSet::from([1610476200]))),
                                        ),
                                        (
                                            String::from("CATEGORY_THREE"),
                                            IndexedConclusion::Exclude(Some(HashSet::from([1610476200]))),
                                        ),
                                    ])
                    }
                ),
                indexed_related_to:  Some(
                    InvertedEventIndex {
                        terms: HashMap::from([])
                    }
                ),
                indexed_geo:         Some(
                    InvertedEventIndex {
                        terms: HashMap::from([])
                    }
                ),
                indexed_class:       Some(
                    InvertedEventIndex {
                        terms: HashMap::from([])
                    }
                ),
            },
        );

        // Assert returns Ok(true) if actually removed existing override.
        assert_eq!(parsed_event.remove_occurrence_override(1610476200, true), Ok(true));

        // Assert override now no longer present in the Event.
        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(
                        RRuleProperty,
                        "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU"
                    )),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: Some(build_property_from_ical!(
                        DTStartProperty,
                        "DTSTART:20201231T183000Z"
                    )),
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties::new(),

                passive_properties: PassiveProperties::new(),

                overrides: BTreeMap::new(),
                indexed_categories: Some(InvertedEventIndex {
                    terms: HashMap::new()
                }),
                indexed_related_to: Some(InvertedEventIndex {
                    terms: HashMap::new()
                }),
                indexed_geo: Some(InvertedEventIndex {
                    terms: HashMap::new()
                }),
                indexed_class: Some(InvertedEventIndex {
                    terms: HashMap::new()
                }),
            }
        );

        // Assert returns Ok(false) if the override did not exist and nothing was removed.
        assert_eq!(parsed_event.remove_occurrence_override(1610476200, true), Ok(false));
    }

    #[test]
    fn test_related_to() {
        let ical: &str = "RELATED-TO:ParentUID_One RELATED-TO;RELTYPE=PARENT:ParentUID_Two RELATED-TO;RELTYPE=CHILD:ChildUID LAST-MODIFIED:20201230T173000Z";

        assert!(Event::parse_ical("event_UID", ical).is_ok());

        let ical: &str = "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two RELATED-TO:ParentUID_One RELATED-TO;RELTYPE=PARENT:ParentUID_Two RELATED-TO;RELTYPE=CHILD:ChildUID LAST-MODIFIED:20201230T173000Z";

        let parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID").into(),
                last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

                schedule_properties: ScheduleProperties {
                    rrule: None,
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: None,
                    dtend: None,
                    parsed_rrule_set: None,
                },

                indexed_properties: IndexedProperties {
                    geo: None,
                    class: None,
                    related_to: Some(HashSet::from([
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"
                        ),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO:ParentUID_One"),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=PARENT:ParentUID_Two"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=CHILD:ChildUID"
                        ),
                    ])),
                    location_type: None,
                    categories: None
                },

                passive_properties: PassiveProperties::new(),

                overrides: BTreeMap::new(),
                indexed_categories: None,
                indexed_related_to: None,
                indexed_geo: None,
                indexed_class: None,
            }
        );
    }

    /*
     * TODO: Come back to this and add class and geo properties...
    #[test]
    fn test_event_occurrence_overrides_rebase_overrides() {
        let mut event_occurrence_overrides = BTreeMap::from([
            (
                1610476300,
                EventOccurrenceOverride {
                    geo: None,
                    class: None,
                    properties: None,
                    categories: None,
                    duration: None,
                    dtstart: None,
                    dtend: None,
                    related_to: None,
                },
            ),
            (
                1610476200,
                EventOccurrenceOverride {
                    geo: None,
                    class: None,
                    properties: Some(BTreeSet::from([
                        KeyValuePair::new(
                            String::from("X-PROPERTY-ONE"),
                            String::from(":PROPERTY_VALUE_ONE"),
                        ),
                        KeyValuePair::new(
                            String::from("X-PROPERTY-ONE"),
                            String::from(":PROPERTY_VALUE_TWO"),
                        ),
                        KeyValuePair::new(
                            String::from("X-PROPERTY-TWO"),
                            String::from(":PROPERTY_VALUE_ONE"),
                        ),
                        KeyValuePair::new(
                            String::from("X-PROPERTY-TWO"),
                            String::from(":PROPERTY_VALUE_TWO"),
                        ),
                    ])),
                    categories: Some(HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                    ])),
                    duration: None,
                    dtstart: None,
                    dtend: None,
                    related_to: Some(HashMap::from([
                        (
                            String::from("PARENT"),
                            HashSet::from([
                                String::from("PARENT_UID_ONE"),
                                String::from("PARENT_UID_TWO"),
                            ]),
                        ),
                        (
                            String::from("CHILD"),
                            HashSet::from([
                                String::from("CHILD_UID_ONE"),
                                String::from("CHILD_UID_TWO"),
                            ]),
                        ),
                    ])),
                },
            ),
        ]);

        let event_diff = EventDiff {
            indexed_categories: Some(UpdatedSetMembers {
                removed: HashSet::from([
                    String::from("CATEGORY_THREE"),
                    String::from("CATEGORY_FIVE"),
                ]),
                maintained: HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                ]),
                added: HashSet::from([String::from("CATEGORY_FOUR")]),
            }),
            indexed_related_to: Some(UpdatedSetMembers {
                removed: HashSet::from([KeyValuePair::new(
                    String::from("PARENT"),
                    String::from("PARENT_UID_ONE"),
                )]),
                maintained: HashSet::from([
                    KeyValuePair::new(String::from("PARENT"), String::from("PARENT_UID_TWO")),
                    KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UID_ONE")),
                    KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UID_TWO")),
                ]),
                added: HashSet::from([KeyValuePair::new(
                    String::from("X-IDX-CAL"),
                    String::from("INDEXED_CALENDAR_UID"),
                )]),
            }),
            indexed_geo: None,
            indexed_class: None,
            passive_properties: Some(UpdatedSetMembers {
                removed: HashSet::from([KeyValuePair {
                    key: String::from("X-PROPERTY-TWO"),
                    value: String::from(":PROPERTY_VALUE_TWO"),
                }]),
                maintained: HashSet::from([
                    KeyValuePair {
                        key: String::from("X-PROPERTY-ONE"),
                        value: String::from(":PROPERTY_VALUE_ONE"),
                    },
                    KeyValuePair {
                        key: String::from("X-PROPERTY-ONE"),
                        value: String::from(":PROPERTY_VALUE_TWO"),
                    },
                    KeyValuePair {
                        key: String::from("X-PROPERTY-TWO"),
                        value: String::from(":PROPERTY_VALUE_ONE"),
                    },
                ]),
                added: HashSet::from([KeyValuePair {
                    key: String::from("X-PROPERTY-THREE"),
                    value: String::from(":PROPERTY_VALUE_ONE"),
                }]),
            }),
            schedule_properties: None,
        };

        // Assert that:
        // * Missing event diff properties marked as maintained are silently ignored
        // * Missing overrides properties marked as removed in the event diff are silently ignored
        // * Existing overrides properties marked as added in the event diff are silently ignored
        // * It applies the diff to the event overrides.
        assert!(rebase_overrides(&mut event_occurrence_overrides, &event_diff).is_ok());

        assert_eq_sorted!(
            event_occurrence_overrides,
            BTreeMap::from([
                (
                    1610476300,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: Some(BTreeSet::from([KeyValuePair::new(
                            String::from("X-PROPERTY-THREE"),
                            String::from(":PROPERTY_VALUE_ONE"),
                        )])),
                        categories: Some(HashSet::from([String::from("CATEGORY_FOUR"),])),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: Some(HashMap::from([(
                            String::from("X-IDX-CAL"),
                            HashSet::from([String::from("INDEXED_CALENDAR_UID"),])
                        ),]))
                    }
                ),
                (
                    1610476200,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: Some(BTreeSet::from([
                            KeyValuePair::new(
                                String::from("X-PROPERTY-ONE"),
                                String::from(":PROPERTY_VALUE_ONE"),
                            ),
                            KeyValuePair::new(
                                String::from("X-PROPERTY-ONE"),
                                String::from(":PROPERTY_VALUE_TWO"),
                            ),
                            KeyValuePair::new(
                                String::from("X-PROPERTY-THREE"),
                                String::from(":PROPERTY_VALUE_ONE"),
                            ),
                            KeyValuePair::new(
                                String::from("X-PROPERTY-TWO"),
                                String::from(":PROPERTY_VALUE_ONE"),
                            ),
                        ])),
                        categories: Some(HashSet::from([
                            String::from("CATEGORY_FOUR"),
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                        ])),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: Some(HashMap::from([
                            (
                                String::from("PARENT"),
                                HashSet::from([String::from("PARENT_UID_TWO"),])
                            ),
                            (
                                String::from("CHILD"),
                                HashSet::from([
                                    String::from("CHILD_UID_ONE"),
                                    String::from("CHILD_UID_TWO"),
                                ])
                            ),
                            (
                                String::from("X-IDX-CAL"),
                                HashSet::from([String::from("INDEXED_CALENDAR_UID"),])
                            ),
                        ]))
                    }
                )
            ])
        );
    }
    */
}
