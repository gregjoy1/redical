use std::collections::{HashSet, HashMap};

use rrule::{RRuleSet, RRuleError};

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedValue};

use crate::data_types::occurrence_index::{OccurrenceIndex, OccurrenceIndexValue};

use crate::data_types::event_occurrence_override::EventOccurrenceOverride;

use crate::data_types::inverted_index::InvertedEventIndex;

use crate::data_types::event_diff::EventDiff;

use crate::data_types::calendar::{CalendarIndexUpdater, CalendarCategoryIndexUpdater, CalendarRelatedToIndexUpdater};

fn property_option_set_or_insert<'a>(property_option: &mut Option<HashSet<String>>, content: &'a str) {
    let content = String::from(content);

    match property_option {
        Some(properties) => { properties.insert(content); },
        None => { *property_option = Some(HashSet::from([content])); }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverrides {
    pub detached: OccurrenceIndex<EventOccurrenceOverride>,
    pub current:  OccurrenceIndex<EventOccurrenceOverride>,
}

impl EventOccurrenceOverrides {
    pub fn new() -> EventOccurrenceOverrides {
        EventOccurrenceOverrides {
            detached: OccurrenceIndex::new(),
            current:  OccurrenceIndex::new(),
        }
    }

    // Rebase all detached and current overrides with added/removed EventDiff properties.
    //
    // This is when an existing event with overrides is updated, and we want to update all the base
    // categories/related_to/properties on each one.
    //
    // If we don't do this, then when we update an existing event to have an additional category,
    // each overridden occurrence with not include that category.
    //
    // TODO: Look into storing diffs in the overrides as opposed to the current state of all overridden properties.
    pub fn rebase_overrides(&mut self, event_diff: &EventDiff) -> Result<&Self, String> {
        for (_timestamp, event_occurrence_override) in self.current.iter_mut() {
            Self::rebase_override(event_occurrence_override, event_diff);
        }

        for (_timestamp, event_occurrence_override) in self.detached.iter_mut() {
            Self::rebase_override(event_occurrence_override, event_diff);
        }

        Ok(self)
    }

    // Rebase specified override with added/removed EventDiff properties.
    fn rebase_override(event_occurrence_override: &mut EventOccurrenceOverride, event_diff: &EventDiff) {
        if let Some(indexed_categories) = &event_diff.indexed_categories {
            match event_occurrence_override.categories.as_mut() {
                Some(overridden_categories) => {
                    for removed_category in indexed_categories.removed.iter() {
                        overridden_categories.remove(removed_category);
                    }

                    for added_category in indexed_categories.added.iter() {
                        overridden_categories.insert(added_category.clone());
                    }
                },

                None => {
                    event_occurrence_override.categories = Some(
                        indexed_categories.added.clone()
                    );
                }
            };
        }

        if let Some(indexed_related_to) = &event_diff.indexed_related_to {
            match event_occurrence_override.related_to.as_mut() {
                Some(overridden_related_to) => {
                    for (removed_reltype, removed_reltype_uuid) in indexed_related_to.removed.iter() {
                        if let Some(reltype_uuids) = overridden_related_to.get_mut(removed_reltype) {
                            reltype_uuids.remove(removed_reltype_uuid);
                        }
                    }

                    for (added_reltype, added_reltype_uuid) in indexed_related_to.added.iter() {
                        overridden_related_to.entry(added_reltype.clone())
                                             .and_modify(|reltype_uuids| { reltype_uuids.insert(added_reltype_uuid.clone()); })
                                             .or_insert(HashSet::from([added_reltype_uuid.clone()]));
                    }
                },

                None => {
                    let mut overridden_related_to = HashMap::new();

                    for (added_reltype, added_reltype_uuid) in indexed_related_to.added.iter() {
                        overridden_related_to.entry(added_reltype.clone())
                                             .and_modify(|reltype_uuids: &mut HashSet<String>| { reltype_uuids.insert(added_reltype_uuid.clone()); })
                                             .or_insert(HashSet::from([added_reltype_uuid.clone()]));
                    }

                    event_occurrence_override.related_to = Some(overridden_related_to);
                }
            };
        }

        if let Some(indexed_passive_properties) = &event_diff.passive_properties {
            match event_occurrence_override.properties.as_mut() {
                Some(overridden_passive_properties) => {
                    for (removed_property, removed_property_value) in indexed_passive_properties.removed.iter() {
                        if let Some(property_uuids) = overridden_passive_properties.get_mut(removed_property) {
                            property_uuids.remove(removed_property_value);
                        }
                    }

                    for (added_property, added_property_value) in indexed_passive_properties.added.iter() {
                        overridden_passive_properties.entry(added_property.clone())
                                                     .and_modify(|property_uuids| { property_uuids.insert(added_property_value.clone()); })
                                                     .or_insert(HashSet::from([added_property_value.clone()]));
                    }
                },

                None => {
                    let mut overridden_passive_properties = HashMap::new();

                    for (added_property, added_property_value) in indexed_passive_properties.added.iter() {
                        overridden_passive_properties.entry(added_property.clone())
                                                     .and_modify(|property_values: &mut HashSet<String>| { property_values.insert(added_property_value.clone()); })
                                                     .or_insert(HashSet::from([added_property_value.clone()]));
                    }

                    event_occurrence_override.properties = Some(overridden_passive_properties);
                }
            };
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ScheduleProperties {
    pub rrule:       Option<HashSet<String>>,
    pub exrule:      Option<HashSet<String>>,
    pub rdate:       Option<HashSet<String>>,
    pub exdate:      Option<HashSet<String>>,
    pub duration:    Option<HashSet<String>>,
    pub dtstart:     Option<HashSet<String>>,
    pub dtend:       Option<HashSet<String>>,
}

impl ScheduleProperties {
    pub fn new() -> ScheduleProperties {
        ScheduleProperties {
            rrule:       None,
            exrule:      None,
            rdate:       None,
            exdate:      None,
            duration:    None,
            dtstart:     None,
            dtend:       None,
        }
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::RRule(content)    => { property_option_set_or_insert(&mut self.rrule, content.content_line); },
            ParsedProperty::ExRule(content)   => { property_option_set_or_insert(&mut self.exrule, content.content_line); },
            ParsedProperty::RDate(content)    => { property_option_set_or_insert(&mut self.rdate, content.content_line); },
            ParsedProperty::ExDate(content)   => { property_option_set_or_insert(&mut self.exdate, content.content_line); },
            ParsedProperty::Duration(content) => { property_option_set_or_insert(&mut self.duration, content.content_line); },
            ParsedProperty::DtStart(content)  => { property_option_set_or_insert(&mut self.dtstart, content.content_line); },
            ParsedProperty::DtEnd(content)    => { property_option_set_or_insert(&mut self.dtend, content.content_line); },

            _ => {
                return Err(String::from("Expected schedule property (RRULE, EXRULE, RDATE, EXDATE, DURATION, DTSTART, DTEND), received: {property.content_line}"))
            }
        }

        Ok(self)
    }

    fn parse_rrule(&self) -> Result<RRuleSet, RRuleError> {
        let mut ical_parts = vec![];

        if self.dtstart.is_some() {
            self.dtstart.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.rrule.is_some() {
            self.rrule.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.exrule.is_some() {
            self.exrule.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.rdate.is_some() {
            self.rdate.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.exdate.is_some() {
            self.exdate.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        ical_parts.join("\n").parse::<RRuleSet>()
    }

    fn validate_rrule(&self) -> bool {
        self.parse_rrule().is_ok()
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct IndexedProperties {
    pub related_to:  Option<HashMap<String, HashSet<String>>>,
    pub categories:  Option<HashSet<String>>
}

impl IndexedProperties {
    pub fn new() -> IndexedProperties {
        IndexedProperties {
            related_to:  None,
            categories:  None
        }
    }

    pub fn get_indexed_calendars(&self) -> Option<HashSet<String>> {
        if let Some(related_to_hashmap) = &self.related_to {
            if let Some(connected_indexed_calendars) = related_to_hashmap.get("X-IDX-CAL") {
                return Some(connected_indexed_calendars.clone());
            }
        }

        None
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::Categories(content)  => {
                match content.value {
                    ParsedValue::List(list) => {
                        list.iter().for_each(|category| {
                            let category = String::from(*category);

                            match &mut self.categories {
                                Some(categories) => {
                                    categories.insert(category);
                                },

                                None => {
                                    self.categories = Some(
                                        HashSet::from([
                                            category
                                        ])
                                    );
                                }
                            };
                        });

                        Ok(self)
                    },
                    _ => {
                        Err(String::from("Expected category to have list value."))
                    }
                }
            },

            // TODO: Break into pieces so that it can be indexed like categories.
            ParsedProperty::RelatedTo(content)  => {
                // TODO: improve
                let default_reltype = String::from("PARENT");

                let reltype: String = match content.params {
                    Some(params) => {
                        match params.get(&"RELTYPE") {
                            Some(values) => {
                                if values.is_empty() {
                                    default_reltype
                                } else if values.len() == 1 {
                                    String::from(values[0])
                                } else {
                                    return Err(String::from("Expected related_to RELTYPE to be a single value."))
                                }
                            },

                            None => default_reltype
                        }
                    },

                    None => default_reltype
                };

                match content.value {
                    ParsedValue::List(list) => {
                        list.iter().for_each(|related_to_uuid| {
                            match &mut self.related_to {
                                Some(related_to_map) => {
                                    related_to_map.entry(reltype.clone())
                                                  .and_modify(|reltype_uuids| { reltype_uuids.insert(String::from(*related_to_uuid)); })
                                                  .or_insert(HashSet::from([String::from(*related_to_uuid)]));
                                },

                                None => {
                                    self.related_to = Some(
                                        HashMap::from(
                                            [
                                                (
                                                    reltype.clone(),
                                                    HashSet::from([
                                                        String::from(*related_to_uuid)
                                                    ])
                                                )
                                            ]
                                        )
                                    );
                                }
                            }
                        });

                        Ok(self)
                    },
                    _ => {
                        Err(String::from("Expected related_to to have list value."))
                    }
                }
            },

            _ => {
                Err(String::from("Expected indexable property (CATEGORIES, RELATED_TO), received: {property.content_line}"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PassiveProperties {
    pub properties: HashMap<String, HashSet<String>>
}

impl PassiveProperties {
    pub fn new() -> PassiveProperties {
        PassiveProperties {
            properties:  HashMap::new(),
        }
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::Description(content) | ParsedProperty::Other(content)  => {
                self.properties.entry(String::from(content.name.unwrap()))
                               .and_modify(|content_lines| {
                                   content_lines.insert(String::from(content.content_line));
                               })
                               .or_insert(HashSet::from([String::from(content.content_line)]));

                Ok(self)
            },

            _ => {
                Err(String::from("Expected passive property, received: {property.content_line}"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event {
    pub uuid:                String,

    pub schedule_properties: ScheduleProperties,
    pub indexed_properties:  IndexedProperties,

    pub passive_properties:  PassiveProperties,

    pub overrides:           EventOccurrenceOverrides,
    pub occurrence_cache:    Option<OccurrenceIndex<OccurrenceIndexValue>>,
    pub indexed_categories:  Option<InvertedEventIndex>,
    pub indexed_related_to:  Option<InvertedEventIndex>,
}

impl Event {
    pub fn new(uuid: String) -> Event {
        Event {
            uuid,

            schedule_properties: ScheduleProperties::new(),
            indexed_properties:  IndexedProperties::new(),

            passive_properties:  PassiveProperties::new(),

            overrides:           EventOccurrenceOverrides::new(),
            occurrence_cache:    None,
            indexed_categories:  None,
            indexed_related_to:  None,
        }
    }

    pub fn parse_ical(uuid: &str, input: &str) -> Result<Event, String> {
        match parse_properties(input) {
            Ok((_, parsed_properties)) => {
                let new_event: &mut Event = &mut Event::new(String::from(uuid));

                parsed_properties.into_iter()
                    .try_for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Categories(_) | ParsedProperty::RelatedTo(_) => {
                                if let Err(error) = new_event.indexed_properties.insert(parsed_property) {
                                    return Err(error);
                                }
                            },

                            ParsedProperty::Description(_) | ParsedProperty::Other(_) => {
                                if let Err(error) = new_event.passive_properties.insert(parsed_property) {
                                    return Err(error);
                                }
                            },

                            // Assumed to be any of:
                            //   - ParsedProperty::RRule
                            //   - ParsedProperty::ExRule
                            //   - ParsedProperty::RDate
                            //   - ParsedProperty::ExDate
                            //   - ParsedProperty::Duration
                            //   - ParsedProperty::DtStart
                            //   - ParsedProperty::DtEnd
                            _ => {
                                if let Err(error) = new_event.schedule_properties.insert(parsed_property) {
                                    return Err(error);
                                }
                            }
                        }

                        Ok(())
                    })?;

                Ok(new_event.clone())
            },
            Err(err) => Err(err.to_string())
        }
    }

    pub fn rebuild_indexed_categories(&mut self, calendar_index_updater: &mut CalendarIndexUpdater) -> Result<&mut Self, String> {
        let mut calendar_category_index_updater = CalendarCategoryIndexUpdater::new(calendar_index_updater);

        self.indexed_categories = Some(
            InvertedEventIndex::new_from_event_categories(
                self,
                &mut calendar_category_index_updater
            )
        );

        Ok(self)
    }

    pub fn rebuild_indexed_related_to(&mut self, calendar_index_updater: &mut CalendarIndexUpdater) -> Result<&mut Self, String> {
        let mut calendar_related_to_index_updater = CalendarRelatedToIndexUpdater::new(calendar_index_updater);

        self.indexed_related_to = Some(
            InvertedEventIndex::new_from_event_related_to(
                self,
                &mut calendar_related_to_index_updater
            )
        );

        Ok(self)
    }

    pub fn override_occurrence(&mut self, timestamp: i64, event_occurrence_override: &EventOccurrenceOverride, calendar_index_updater: &mut CalendarIndexUpdater) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {

                match occurrence_cache.get(timestamp) {
                    Some(OccurrenceIndexValue::Occurrence) => {
                        occurrence_cache.insert(timestamp, OccurrenceIndexValue::Override);

                        self.overrides.current.insert(timestamp, event_occurrence_override.clone());
                    },
                    Some(OccurrenceIndexValue::Override) => {
                        self.overrides.current.insert(timestamp, event_occurrence_override.clone());
                    },
                    None => {
                        return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
                    }
                }

                if let Some(ref mut indexed_categories) = self.indexed_categories {

                    if let Some(overridden_categories) = &event_occurrence_override.categories {
                        let mut calendar_category_index_updater = CalendarCategoryIndexUpdater::new(calendar_index_updater);

                        indexed_categories.insert_override(
                            timestamp,
                            overridden_categories,
                            &mut calendar_category_index_updater
                        );
                    }

                } else {
                    self.rebuild_indexed_categories(calendar_index_updater)?;
                }

                if let Some(ref mut indexed_related_to) = self.indexed_related_to {

                    if let Some(overridden_related_to_set) = &event_occurrence_override.build_override_related_to_set() {
                        let mut calendar_related_to_index_updater = CalendarRelatedToIndexUpdater::new(calendar_index_updater);

                        indexed_related_to.insert_override(
                            timestamp,
                            overridden_related_to_set,
                            &mut calendar_related_to_index_updater
                        );
                    }

                } else {
                    self.rebuild_indexed_related_to(calendar_index_updater)?;
                }
            },
            None => {
                return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
            }
        }

        Ok(self)
    }

    pub fn remove_occurrence_override(&mut self, timestamp: i64, calendar_index_updater: &mut CalendarIndexUpdater) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {

                match occurrence_cache.get(timestamp) {
                    Some(OccurrenceIndexValue::Occurrence) => {
                        return Err(format!("No occurrence override exists for timestamp: {timestamp}"));
                    },
                    Some(OccurrenceIndexValue::Override) => {
                        occurrence_cache.insert(timestamp, OccurrenceIndexValue::Occurrence);

                        self.overrides.current.remove(timestamp);

                        let mut calendar_category_index_updater = CalendarCategoryIndexUpdater::new(calendar_index_updater);

                        if let Some(ref mut indexed_categories) = self.indexed_categories {
                            indexed_categories.remove_override(
                                timestamp,
                                &mut calendar_category_index_updater
                            );
                        } else {
                            self.indexed_categories = Some(
                                InvertedEventIndex::new_from_event_categories(
                                    &*self,
                                    &mut calendar_category_index_updater
                                )
                            );
                        }

                        let mut calendar_related_to_index_updater = CalendarRelatedToIndexUpdater::new(calendar_index_updater);

                        if let Some(ref mut indexed_related_to) = self.indexed_related_to {
                            indexed_related_to.remove_override(
                                timestamp,
                                &mut calendar_related_to_index_updater
                            );
                        } else {
                            self.indexed_related_to = Some(
                                InvertedEventIndex::new_from_event_related_to(
                                    &*self,
                                    &mut calendar_related_to_index_updater
                                )
                            );
                        }
                    },

                    None => {
                        return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
                    }
                }
            },
            None => {
                return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
            }
        }

        Ok(self)
    }

    pub fn rebuild_occurrence_cache(&mut self, max_count: usize) -> Result<&mut Self, RRuleError> {
        let rrule_set = self.schedule_properties.parse_rrule()?;
        let rrule_set_iter = rrule_set.into_iter();

        let mut occurrence_cache: OccurrenceIndex<OccurrenceIndexValue> = OccurrenceIndex::new();

        let max_datetime = self.get_max_datetime();

        for next_datetime in rrule_set_iter.take(max_count) {
            if next_datetime.gt(&max_datetime) {
                break;
            }

            occurrence_cache.insert(next_datetime.timestamp(), OccurrenceIndexValue::Occurrence);
        }

        self.occurrence_cache = Some(occurrence_cache);

        Ok(self)
    }

    fn get_max_datetime(&self) -> DateTime<Utc> {
        // TODO: Get max extrapolation window from redis module config.
        Utc::now().checked_add_months(Months::new(12))
                  .and_then(|date_time| date_time.checked_add_days(Days::new(1)))
                  .and_then(|date_time| date_time.with_hour(0))
                  .and_then(|date_time| date_time.with_minute(0))
                  .and_then(|date_time| date_time.with_second(0))
                  .and_then(|date_time| date_time.with_nanosecond(0))
                  .unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::data_types::{InvertedIndexListener, IndexedConclusion};

    use crate::data_types::utils::UpdatedSetMembers;

    use std::collections::BTreeMap;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_indexed_categories() {
        let event = Event {
            uuid: String::from("event_UUID"),

            schedule_properties: ScheduleProperties {
                rrule:    None,
                exrule:   None,
                rdate:    None,
                exdate:   None,
                duration: None,
                dtstart:  None,
                dtend:    None,
            },

            indexed_properties: IndexedProperties {
                related_to: None,
                categories: Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY_THREE")
                    ])
                ),
            },

            passive_properties: PassiveProperties {
                properties: HashMap::new()
            },

            overrides: EventOccurrenceOverrides {
                detached: OccurrenceIndex::new(),
                current:  OccurrenceIndex::new_with_values(
                    vec![
                        // Override 100 has all event categories plus CATEGORY_FOUR
                        (
                            100,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                        String::from("CATEGORY_THREE"),
                                        String::from("CATEGORY_FOUR"),
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: None,
                                related_to:  None
                            }
                        ),

                        // Override 200 has only some event categories (missing CATEGORY_THREE)
                        (
                            200,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: None,
                                related_to:  None
                            }
                        ),

                        // Override 300 has no overridden categories
                        (
                            300,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  None,
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: None,
                                related_to:  None
                            }
                        ),

                        // Override 400 has removed all categories
                        (
                            400,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  Some(HashSet::new()),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: None,
                                related_to:  None
                            }
                        ),

                        // Override 500 has no base event categories, but does have CATEGORY_FOUR
                        (
                            500,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_FOUR"),
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: None,
                                related_to:  None
                            }
                        ),
                    ]
                ),
            },
            occurrence_cache:   None,
            indexed_categories: None,
            indexed_related_to: None,
        };

        struct CallbackContainer {
            handle_update_values: Vec<(String, Option<IndexedConclusion>)>
        }

        impl InvertedIndexListener for CallbackContainer {
            fn handle_update(&mut self, category: &String, indexed_conclusion: Option<&IndexedConclusion>) {
                self.handle_update_values.push(
                    (
                        category.clone(),
                        indexed_conclusion.map(|value_pointer| value_pointer.clone())
                    )
                );
            }
        }

        impl CallbackContainer {
            pub fn clear(&mut self) {
                self.handle_update_values.clear();
            }
        }

        let mut callback_container = CallbackContainer {
            handle_update_values: Vec::new()
        };

        let mut indexed_categories = InvertedEventIndex::new_from_event_categories(&event, &mut callback_container);

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

        assert_eq!(callback_container.handle_update_values.len(), 12);

        fn sort_by_category_name(array: Vec<(String, Option<IndexedConclusion>)>) -> Vec<(String, Option<IndexedConclusion>)> {
            let mut sorted_array = array.clone();

            sorted_array.sort_by_key(|(category_name, _)| category_name.clone());

            sorted_array
        }

        // Assert that base event categories added...
        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values[0..=2].to_vec()),
            vec![
                (
                    String::from("CATEGORY_ONE"),
                    Some(IndexedConclusion::Include(None))
                ),
                (
                    String::from("CATEGORY_THREE"),
                    Some(IndexedConclusion::Include(None))
                ),
                (
                    String::from("CATEGORY_TWO"),
                    Some(IndexedConclusion::Include(None))
                ),
            ]
        );

        // Assert that override epoch 100 with all base categories + CATEGORY_FOUR
        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values[3..=3].to_vec()),
            vec![
                (
                    String::from("CATEGORY_FOUR"),
                    Some(IndexedConclusion::Exclude(Some(HashSet::from([100]))))
                ),
            ]
        );

        // Assert that override epoch 200 with only CATEGORY_ONE + CATEGORY_TWO - CATEGORY_THREE
        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values[4..=4].to_vec()),
            vec![
                (
                    String::from("CATEGORY_THREE"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([200]))))
                ),
            ]
        );

        // Assert that:
        // * Override epoch 300 with no category overrides -- skipped
        // * Override epoch 400 overridden to include no category at all
        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values[5..=7].to_vec()),
            vec![
                (
                    String::from("CATEGORY_ONE"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400]))))
                ),
                (
                    String::from("CATEGORY_THREE"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400, 200]))))
                ),
                (
                    String::from("CATEGORY_TWO"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400]))))
                ),
            ]
        );

        // Assert that override epoch 500 overridden to only include CATEGORY_FOUR
        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values[8..=11].to_vec()),
            vec![
                (
                    String::from("CATEGORY_FOUR"),
                    Some(IndexedConclusion::Exclude(Some(HashSet::from([500, 100]))))
                ),
                (
                    String::from("CATEGORY_ONE"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400, 500]))))
                ),
                (
                    String::from("CATEGORY_THREE"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400, 200, 500]))))
                ),
                (
                    String::from("CATEGORY_TWO"),
                    Some(IndexedConclusion::Include(Some(HashSet::from([400, 500]))))
                ),
            ]
        );

        // Clear all recorded handle_update_values
        callback_container.clear();

        indexed_categories.insert_override(
            600,
            &HashSet::from([
                String::from("CATEGORY_ONE"),
                String::from("CATEGORY_FIVE"),
            ]),
            &mut callback_container
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

        assert_eq!(
            sort_by_category_name(callback_container.handle_update_values.clone()),
            vec![
                (String::from("CATEGORY_FIVE"),  Some(IndexedConclusion::Exclude(Some(HashSet::from([600]))))),
                (String::from("CATEGORY_THREE"), Some(IndexedConclusion::Include(Some(HashSet::from([500, 600, 200, 400]))))),
                (String::from("CATEGORY_TWO"),   Some(IndexedConclusion::Include(Some(HashSet::from([500, 400, 600]))))),
            ]
        );

        // Clear all recorded handle_update_values
        callback_container.clear();

        indexed_categories.remove_override(100, &mut callback_container);

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

        // Clear all recorded handle_update_values
        callback_container.clear();

        indexed_categories.remove_override(500, &mut callback_container);

        assert_eq!(
            callback_container.handle_update_values,
            vec![
                (String::from("CATEGORY_FOUR"), None)
            ]
        );

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

        assert_eq!(
            callback_container.handle_update_values,
            vec![
                (String::from("CATEGORY_FOUR"), None)
            ]
        );

    }

    #[test]
    fn test_parse_ical() {
        let ical: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            Event::parse_ical("event_UUID", ical).unwrap(),
            Event {
                uuid: String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          None,
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties {
                    categories:       Some(
                        HashSet::from([
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                            String::from("CATEGORY THREE")
                        ])
                    ),
                    related_to:       None,
                },

                passive_properties:  PassiveProperties {
                    properties: HashMap::from(
                                    [
                                        (
                                            String::from("DESCRIPTION"),
                                            HashSet::from([
                                                String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                                            ])
                                        )
                                    ]
                                )
                },

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );
    }

    #[test]
    fn retest_build_occurrence_cache() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU DTSTART:20201231T183000Z";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            String::from("DTSTART:20201231T183000Z")
                        ])
                    ),
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );

        assert!(
            parsed_event.rebuild_occurrence_cache(100).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_cache,
            Some(
                OccurrenceIndex {
                    base_timestamp: Some(1609871400),
                    timestamp_offsets: BTreeMap::from(
                        [
                            (0, OccurrenceIndexValue::Occurrence),
                            (604800, OccurrenceIndexValue::Occurrence),
                            (1209600, OccurrenceIndexValue::Occurrence),
                            (1814400, OccurrenceIndexValue::Occurrence),
                            (2419200, OccurrenceIndexValue::Occurrence),
                            (3024000, OccurrenceIndexValue::Occurrence),
                            (3628800, OccurrenceIndexValue::Occurrence),
                            (4233600, OccurrenceIndexValue::Occurrence),
                            (4838400, OccurrenceIndexValue::Occurrence),
                            (5443200, OccurrenceIndexValue::Occurrence),
                            (6048000, OccurrenceIndexValue::Occurrence),
                            (6652800, OccurrenceIndexValue::Occurrence),
                            (7257600, OccurrenceIndexValue::Occurrence),
                        ]
                    )
                }
            )
        );

        assert!(
            parsed_event.rebuild_occurrence_cache(2).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_cache,
            Some(
                OccurrenceIndex {
                    base_timestamp: Some(1609871400),
                    timestamp_offsets: BTreeMap::from(
                        [
                            (0, OccurrenceIndexValue::Occurrence),
                            (604800, OccurrenceIndexValue::Occurrence),
                        ]
                    )
                }
            )
        );
    }

    #[test]
    fn test_validate_rrule() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH DTSTART:16010101T020000";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            String::from("DTSTART:16010101T020000")
                        ])
                    ),
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );

        assert!(parsed_event.schedule_properties.validate_rrule());

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          None,
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );

        assert_eq!(parsed_event.schedule_properties.validate_rrule(), false);
    }

    #[test]
    fn test_occurrence_override_insertion_and_deletion() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU DTSTART:20201231T183000Z";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert!(
            parsed_event.rebuild_occurrence_cache(2).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_cache,
            Some(
                OccurrenceIndex {
                    base_timestamp: Some(1609871400),
                    timestamp_offsets: BTreeMap::from(
                        [
                            (0, OccurrenceIndexValue::Occurrence),
                            (604800, OccurrenceIndexValue::Occurrence),
                        ]
                    )
                }
            )
        );

        let event_occurrence_override = EventOccurrenceOverride {
            properties:  None,
            categories:  Some(
                HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                    String::from("CATEGORY_THREE")
                ])
            ),
            duration:    None,
            dtstart:     None,
            dtend:       None,
            description: Some(String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")),
            related_to:  None
        };

        // TODO: potentially test this interaction?!
        let mut calendar_index_updater = CalendarIndexUpdater::new(parsed_event.uuid.clone(), vec![], vec![]);

        assert_eq!(
            parsed_event.override_occurrence(1234, &event_occurrence_override, &mut calendar_index_updater),
            Err(
                String::from("No overridable occurrence exists for timestamp: 1234")
            )
        );

        assert_eq!(
            parsed_event.override_occurrence(1610476200, &event_occurrence_override, &mut calendar_index_updater),
            Ok(
                &Event {
                    uuid:                String::from("event_UUID"),

                    schedule_properties: ScheduleProperties {
                        rrule:            Some(
                            HashSet::from([
                                String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                            ])
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            HashSet::from([
                                String::from("DTSTART:20201231T183000Z")
                            ])
                        ),
                        dtend:            None,
                    },

                    indexed_properties:  IndexedProperties::new(),

                    passive_properties:  PassiveProperties::new(),

                    overrides:           EventOccurrenceOverrides {
                        detached: OccurrenceIndex::new(),
                        current:  OccurrenceIndex::new_with_value(
                            1610476200,
                            EventOccurrenceOverride {
                                properties:  None,
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                        String::from("CATEGORY_THREE")
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                description: Some(String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")),
                                related_to:  None
                            }
                        ),
                    },
                    occurrence_cache:    Some(
                        OccurrenceIndex {
                            base_timestamp: Some(1609871400),
                            timestamp_offsets: BTreeMap::from(
                                [
                                    (0, OccurrenceIndexValue::Occurrence),
                                    (604800, OccurrenceIndexValue::Override),
                                ]
                            )
                        }
                    ),
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
                    )
                }
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1234, &mut calendar_index_updater),
            Err(
                String::from("No overridable occurrence exists for timestamp: 1234")
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1609871400, &mut calendar_index_updater),
            Err(
                String::from("No occurrence override exists for timestamp: 1609871400")
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1610476200, &mut calendar_index_updater),
            Ok(
                &Event {
                    uuid:                String::from("event_UUID"),

                    schedule_properties: ScheduleProperties {
                        rrule:            Some(
                            HashSet::from([
                                String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                            ])
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            HashSet::from([
                                String::from("DTSTART:20201231T183000Z")
                            ])
                        ),
                        dtend:            None,
                    },

                    indexed_properties:  IndexedProperties::new(),

                    passive_properties:  PassiveProperties::new(),

                    overrides:           EventOccurrenceOverrides {
                        detached: OccurrenceIndex::new(),
                        current:  OccurrenceIndex::new(),
                    },
                    occurrence_cache:    Some(
                        OccurrenceIndex {
                            base_timestamp: Some(1609871400),
                            timestamp_offsets: BTreeMap::from(
                                [
                                    (0, OccurrenceIndexValue::Occurrence),
                                    (604800, OccurrenceIndexValue::Occurrence),
                                ]
                            )
                        }
                    ),
                    indexed_categories:  Some(
                        InvertedEventIndex {
                            terms: HashMap::new()
                        }
                    ),
                    indexed_related_to:  Some(
                        InvertedEventIndex {
                            terms: HashMap::new()
                        }
                    ),
                }
            )
        );
    }

    #[test]
    fn test_related_to() {
        let ical: &str = "RELATED-TO:ParentUUID_One RELATED-TO;RELTYPE=PARENT:ParentUUID_Two RELATED-TO;RELTYPE=CHILD:ChildUUID";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event.indexed_properties.get_indexed_calendars(),
            None
        );

        let ical: &str = "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One,redical//IndexedCalendar_Two RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three,redical//IndexedCalendar_Two RELATED-TO:ParentUUID_One RELATED-TO;RELTYPE=PARENT:ParentUUID_Two RELATED-TO;RELTYPE=CHILD:ChildUUID";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            None,
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          None,
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties {
                    related_to: Some(
                                    HashMap::from([
                                        (
                                            String::from("X-IDX-CAL"),
                                            HashSet::from([
                                                String::from("redical//IndexedCalendar_One"),
                                                String::from("redical//IndexedCalendar_Two"),
                                                String::from("redical//IndexedCalendar_Three"),
                                            ])
                                        ),
                                        (
                                            String::from("PARENT"),
                                            HashSet::from([
                                                String::from("ParentUUID_One"),
                                                String::from("ParentUUID_Two"),
                                            ])
                                        ),
                                        (
                                            String::from("CHILD"),
                                            HashSet::from([
                                                String::from("ChildUUID"),
                                            ])
                                        )
                                    ])
                                ),
                    categories: None
                },

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );

        assert_eq!(
            parsed_event.indexed_properties.get_indexed_calendars(),
            Some(
                HashSet::from([
                    String::from("redical//IndexedCalendar_One"),
                    String::from("redical//IndexedCalendar_Two"),
                    String::from("redical//IndexedCalendar_Three"),
                ])
            )
        );
    }

    #[test]
    fn benchmark_build_occurrence_cache() {
        let ical: &str = "RRULE:FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1 DTSTART:20201231T183000Z";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            String::from("RRULE:FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1")
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            String::from("DTSTART:20201231T183000Z")
                        ])
                    ),
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
            }
        );

        let start = std::time::Instant::now();

        assert!(
            // parsed_event.rebuild_occurrence_cache(65535).is_ok()
            parsed_event.rebuild_occurrence_cache(8760).is_ok()
        );

        let duration = start.elapsed();

        println!("Time elapsed in rebuild_occurrence_cache() is: {:?}", duration);
    }

    #[test]
    fn test_event_occurrence_overrides_rebase_overrides() {

        let mut event_occurrence_overrides = EventOccurrenceOverrides {
            detached: OccurrenceIndex::new_with_value(
                1610476300,
                EventOccurrenceOverride {
                    properties:  None,
                    categories:  None,
                    duration:    None,
                    dtstart:     None,
                    dtend:       None,
                    description: None,
                    related_to:  None,
                }
            ),
            current:  OccurrenceIndex::new_with_value(
                1610476200,
                EventOccurrenceOverride {
                    properties:  Some(
                        HashMap::from([
                            (
                                String::from("X-PROPERTY-ONE"),
                                HashSet::from([
                                    String::from("PROPERTY_VALUE_ONE"),
                                    String::from("PROPERTY_VALUE_TWO"),
                                ])
                            ),
                            (
                                String::from("X-PROPERTY-TWO"),
                                HashSet::from([
                                    String::from("PROPERTY_VALUE_ONE"),
                                    String::from("PROPERTY_VALUE_TWO"),
                                ])
                            )
                        ])
                    ),
                    categories:  Some(
                        HashSet::from([
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                        ])
                    ),
                    duration:    None,
                    dtstart:     None,
                    dtend:       None,
                    description: None,
                    related_to:  Some(
                        HashMap::from([
                            (
                                String::from("PARENT"),
                                HashSet::from([
                                    String::from("PARENT_UUID_ONE"),
                                    String::from("PARENT_UUID_TWO"),
                                ])
                            ),
                            (
                                String::from("CHILD"),
                                HashSet::from([
                                    String::from("CHILD_UUID_ONE"),
                                    String::from("CHILD_UUID_TWO"),
                                ])
                            )
                        ])
                    )
                }
            ),
        };

        let event_diff = EventDiff {
            indexed_calendars:   None,
            indexed_categories:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([
                        String::from("CATEGORY_THREE"),
                        String::from("CATEGORY_FIVE"),
                    ]),
                    maintained: HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                    ]),
                    added:      HashSet::from([
                        String::from("CATEGORY_FOUR"),
                    ])
                }
            ),
            indexed_related_to:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([
                        (
                            String::from("PARENT"),
                            String::from("PARENT_UUID_ONE"),
                        ),
                    ]),
                    maintained: HashSet::from([
                        (
                            String::from("PARENT"),
                            String::from("PARENT_UUID_TWO"),
                        ),
                        (
                            String::from("CHILD"),
                            String::from("CHILD_UUID_ONE"),
                        ),
                        (
                            String::from("CHILD"),
                            String::from("CHILD_UUID_TWO"),
                        ),
                    ]),
                    added:      HashSet::from([
                        (
                            String::from("X-IDX-CAL"),
                            String::from("INDEXED_CALENDAR_UUID"),
                        ),
                    ])
                }
            ),
            passive_properties:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([
                        (
                            String::from("X-PROPERTY-TWO"),
                            String::from("PROPERTY_VALUE_TWO")
                        )
                    ]),
                    maintained: HashSet::from([
                        (
                            String::from("X-PROPERTY-ONE"),
                            String::from("PROPERTY_VALUE_ONE")
                        ),
                        (
                            String::from("X-PROPERTY-ONE"),
                            String::from("PROPERTY_VALUE_TWO")
                        ),
                        (
                            String::from("X-PROPERTY-TWO"),
                            String::from("PROPERTY_VALUE_ONE")
                        ),
                    ]),
                    added:      HashSet::from([
                        (
                            String::from("X-PROPERTY-THREE"),
                            String::from("PROPERTY_VALUE_ONE")
                        )
                    ])
                }
            ),
            schedule_properties: None,
        };

        // Assert that:
        // * Missing event diff properties marked as maintained are silently ignored
        // * Missing overrides properties marked as removed in the event diff are silently ignored
        // * Existing overrides properties marked as added in the event diff are silently ignored
        // * It applies the diff to the event overrides.
        assert_eq_sorted!(
            event_occurrence_overrides.rebase_overrides(&event_diff),
            Ok(
                &EventOccurrenceOverrides {
                    detached: OccurrenceIndex::new_with_value(
                        1610476300,
                        EventOccurrenceOverride {
                            properties: Some(
                                HashMap::from([
                                    (
                                        String::from("X-PROPERTY-THREE"),
                                        HashSet::from([
                                            String::from("PROPERTY_VALUE_ONE"),
                                        ])
                                    ),
                                ])
                            ),
                            categories:  Some(
                                HashSet::from([
                                    String::from("CATEGORY_FOUR"),
                                ])
                            ),
                            duration:    None,
                            dtstart:     None,
                            dtend:       None,
                            description: None,
                            related_to:  Some(
                                HashMap::from([
                                    (
                                        String::from("X-IDX-CAL"),
                                        HashSet::from([
                                            String::from("INDEXED_CALENDAR_UUID"),
                                        ])
                                    ),
                                ])
                            )
                        }
                    ),
                    current:  OccurrenceIndex::new_with_value(
                        1610476200,
                        EventOccurrenceOverride {
                            properties: Some(
                                HashMap::from([
                                    (
                                        String::from("X-PROPERTY-ONE"),
                                        HashSet::from([
                                            String::from("PROPERTY_VALUE_ONE"),
                                            String::from("PROPERTY_VALUE_TWO"),
                                        ])
                                    ),
                                    (
                                        String::from("X-PROPERTY-TWO"),
                                        HashSet::from([
                                            String::from("PROPERTY_VALUE_ONE"),
                                        ])
                                    ),
                                    (
                                        String::from("X-PROPERTY-THREE"),
                                        HashSet::from([
                                            String::from("PROPERTY_VALUE_ONE"),
                                        ])
                                    ),
                                ])
                            ),
                            categories:  Some(
                                HashSet::from([
                                    String::from("CATEGORY_ONE"),
                                    String::from("CATEGORY_TWO"),
                                    String::from("CATEGORY_FOUR"),
                                ])
                            ),
                            duration:    None,
                            dtstart:     None,
                            dtend:       None,
                            description: None,
                            related_to:  Some(
                                HashMap::from([
                                    (
                                        String::from("PARENT"),
                                        HashSet::from([
                                            String::from("PARENT_UUID_TWO"),
                                        ])
                                    ),
                                    (
                                        String::from("CHILD"),
                                        HashSet::from([
                                            String::from("CHILD_UUID_ONE"),
                                            String::from("CHILD_UUID_TWO"),
                                        ])
                                    ),
                                    (
                                        String::from("X-IDX-CAL"),
                                        HashSet::from([
                                            String::from("INDEXED_CALENDAR_UUID"),
                                        ])
                                    ),
                                ])
                            )
                        }
                    ),
                }
            )
        );
    }
}
