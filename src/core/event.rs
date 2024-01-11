use std::str::FromStr;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use rrule::{RRuleError, RRuleSet};

use crate::core::ical::serializer::SerializableICalProperty;
use crate::core::ical::properties::{Property, Properties, RRuleProperty, ExRuleProperty, RDateProperty, ExDateProperty, DurationProperty, DTStartProperty, DTEndProperty, GeoProperty, RelatedToProperty, CategoriesProperty, ClassProperty};

use crate::core::parsers::datetime::{datestring_to_date, ParseError};

use crate::core::event_occurrence_override::EventOccurrenceOverride;

use crate::core::inverted_index::InvertedEventIndex;

use crate::core::geo_index::GeoPoint;

use crate::core::event_diff::EventDiff;

use crate::core::utils::KeyValuePair;

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
    for (_timestamp, event_occurrence_override) in overrides.iter_mut() {
        rebase_override(event_occurrence_override, event_diff);
    }

    Ok(())
}

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

    pub fn extract_rrule_key_value_pair(&self) -> Option<KeyValuePair> {
        self.rrule.and_then(|property| Some(property.to_key_value_pair()))
    }

    pub fn extract_exrule_key_value_pair(&self) -> Option<KeyValuePair> {
        self.exrule.and_then(|property| Some(property.to_key_value_pair()))
    }

    pub fn extract_rdates_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.rdates.and_then(|properties| {
            let mut key_value_pairs = HashSet::new();

            for property in properties {
                key_value_pairs.insert(property.to_key_value_pair());
            }

            Some(key_value_pairs)
        })
    }

    pub fn extract_exdates_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.exdates.and_then(|properties| {
            let mut key_value_pairs = HashSet::new();

            for property in properties {
                key_value_pairs.insert(property.to_key_value_pair());
            }

            Some(key_value_pairs)
        })
    }

    pub fn extract_duration_key_value_pair(&self) -> Option<KeyValuePair> {
        self.duration.and_then(|property| Some(property.to_key_value_pair()))
    }

    pub fn extract_dtstart_key_value_pair(&self) -> Option<KeyValuePair> {
        self.dtstart.and_then(|property| Some(property.to_key_value_pair()))
    }

    pub fn extract_dtend_key_value_pair(&self) -> Option<KeyValuePair> {
        self.dtend.and_then(|property| Some(property.to_key_value_pair()))
    }

    pub fn insert(&mut self, property: Property) -> Result<&Self, String> {
        match property {
            Property::RRule(property) => { self.rrule = Some(property); },
            Property::ExRule(property) => { self.exrule = Some(property); },
            Property::DTStart(property) => { self.dtstart = Some(property); },
            Property::DTEnd(property) => { self.dtend = Some(property); },

            Property::RDate(property) => {
                match &mut self.rdates {
                    Some(rdates) => { rdates.insert(property); },
                    None => { self.rdates = Some(HashSet::from([property])); }
                }
            },

            Property::ExDate(property) => {
                match &mut self.exdates {
                    Some(exdates) => { exdates.insert(property); },
                    None => { self.exdates = Some(HashSet::from([property])); }
                }
            },

            Property::Duration(property) => {
                self.duration = Some(property);
            },

            _ => {
                return Err(format!("Expected schedule property (RRULE, EXRULE, RDATE, EXDATE, DURATION, DTSTART, DTEND), received: {:#?}", property))
            }
        }

        Ok(self)
    }

    pub fn parse_rrule(&self) -> Result<RRuleSet, RRuleError> {
        let mut is_missing_rules = true;
        let mut ical_parts = vec![];

        if let Some(rrule) = &self.rrule {
            is_missing_rules = false;

            ical_parts.push(rrule.serialize_to_ical());
        }

        if let Some(exrule) = &self.exrule {
            is_missing_rules = false;

            ical_parts.push(exrule.serialize_to_ical());
        }

        if let Some(rdatesss) = &self.rdates {
            rdatesss.iter().for_each(|rdates| {
                is_missing_rules = false;

                ical_parts.push(rdates.serialize_to_ical());
            });
        }

        if let Some(exdatesss) = &self.exdates {
            exdatesss
                .iter()
                .for_each(|exdates| {
                    is_missing_rules = false;

                    ical_parts.push(exdates.serialize_to_ical());
                });
        }

        if let Some(dtstart) = &self.dtstart {
            ical_parts.push(dtstart.serialize_to_ical());

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
            if is_missing_rules {
                ical_parts.push(String::from("RRULE:FREQ=MINUTELY;COUNT=1"));
            }
        }

        ical_parts.join("\n").parse::<RRuleSet>()
    }

    pub fn get_dtstart_timestamp(&self) -> Result<Option<i64>, ParseError> {
        if let Some(dtstart) = self.dtstart.as_ref() {
            // TODO: properly parse this so TZID is catered to.
            let parsed_datetime = dtstart
                .serialize_to_ical()
                .replace(&String::from("DTSTART:"), &String::from(""));

            return match datestring_to_date(&parsed_datetime, None, "DTSTART") {
                Ok(datetime) => Ok(Some(datetime.timestamp())),
                Err(error) => Err(error),
            };
        }

        Ok(None)
    }

    pub fn get_dtend_timestamp(&self) -> Result<Option<i64>, ParseError> {
        if let Some(dtend) = self.dtend.as_ref() {
            // TODO: properly parse this so TZID is catered to.
            let parsed_datetime = dtend
                .serialize_to_ical()
                .replace(&String::from("DTEND:"), &String::from(""));

            return match datestring_to_date(&parsed_datetime, None, "DTEND") {
                Ok(datetime) => Ok(Some(datetime.timestamp())),
                Err(error) => Err(error),
            };
        }

        Ok(None)
    }

    pub fn get_duration(&self) -> Result<Option<i64>, ParseError> {
        if let Some(parsed_duration) = self.duration.as_ref() {
            return Ok(Some(parsed_duration.get_duration_in_seconds()));
        }

        match (self.get_dtstart_timestamp(), self.get_dtend_timestamp()) {
            (Ok(Some(dtstart_timestamp)), Ok(Some(dtend_timestamp))) => {
                Ok(Some(dtend_timestamp - dtstart_timestamp))
            }

            _ => Ok(None),
        }
    }

    pub fn build_parsed_rrule_set(&mut self) -> Result<(), rrule::RRuleError> {
        let parsed_rrule_set = self.parse_rrule()?;

        self.parsed_rrule_set = Some(parsed_rrule_set);

        Ok(())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IndexedProperties {
    pub geo: Option<GeoProperty>,
    pub related_to: Option<HashSet<RelatedToProperty>>,
    pub categories: Option<HashSet<CategoriesProperty>>,
    pub class: Option<ClassProperty>,
}

impl IndexedProperties {
    pub fn new() -> IndexedProperties {
        IndexedProperties {
            geo: None,
            related_to: None,
            categories: None,
            class: None,
        }
    }

    pub fn extract_all_category_strings(&self) -> Option<HashSet<String>> {
        self.categories.and_then(|categories_properties| {
            let mut categories: HashSet<String> = HashSet::new();

            for categories_property in categories_properties {
                for category in categories_property.categories {
                    categories.insert(category);
                }
            }

            Some(categories)
        })
    }

    pub fn extract_all_related_to_key_value_pairs(&self) -> Option<HashSet<KeyValuePair>> {
        self.related_to.and_then(|related_to_properties| {
            let mut related_to_key_value_pairs: HashSet<KeyValuePair> = HashSet::new();

            for related_to_property in related_to_properties {
                related_to_key_value_pairs.insert(related_to_property.to_key_value_pair());
            }

            Some(related_to_key_value_pairs)
        })
    }

    pub fn extract_geo_point(&self) -> Option<GeoPoint> {
        self.geo.and_then(|geo_property| Some(GeoPoint::from(geo_property)))
    }

    pub fn extract_class(&self) -> Option<String> {
        self.class.and_then(|class_property| Some(class_property.class))
    }

    pub fn insert(&mut self, property: Property) -> Result<&Self, String> {
        match property {
            Property::Class(property) => {
                self.class = Some(property);
            },

            Property::Geo(property) => {
                self.geo = Some(property);
            },

            Property::Categories(property) => {
                self.categories.get_or_insert(HashSet::new()).insert(property);
            },

            Property::RelatedTo(property) => {
                self.related_to.get_or_insert(HashSet::new()).insert(property);
            },

            _ => {
                return Err(format!("Expected indexable property (CATEGORIES, RELATED_TO), received: {}", property.serialize_to_ical()));
            }
        };

        Ok(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PassiveProperties {
    pub properties: BTreeSet<Property>,
}

impl PassiveProperties {
    pub fn new() -> PassiveProperties {
        PassiveProperties {
            properties: BTreeSet::new(),
        }
    }

    pub fn extract_properties_key_value_pairs(&self) -> HashSet<KeyValuePair> {
        let mut key_value_pairs = HashSet::new();

        for property in self.properties {
            key_value_pairs.insert(property.to_key_value_pair());
        }

        key_value_pairs
    }

    pub fn insert(&mut self, property: Property) -> Result<&Self, String> {
        match property {
            Property::Class(_) |
            Property::Geo(_) |
            Property::Categories(_) |
            Property::RelatedTo(_) |
            Property::RRule(_) |
            Property::ExRule(_) |
            Property::DTStart(_) |
            Property::DTEnd(_) |
            Property::RDate(_) |
            Property::ExDate(_) |
            Property::Duration(_) => {
                return Err(format!("Expected passive property, received: {}", property.serialize_to_ical()));
            },

            _ => {
                self.properties.insert(property);
            },
        };

        Ok(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Event {
    pub uid: String,

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
            uid,

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

    pub fn parse_ical(uid: &str, input: &str) -> Result<Event, String> {
        Properties::from_str(input).and_then(|Properties(parsed_properties)| {
            let mut new_event = Event::new(String::from(uid));

            for parsed_property in parsed_properties {
                match parsed_property {
                    Property::Class(_) |
                    Property::Geo(_) |
                    Property::Categories(_) |
                    Property::RelatedTo(_) => {
                        new_event.indexed_properties.insert(parsed_property)?;
                    },

                    Property::RRule(_) |
                    Property::ExRule(_) |
                    Property::DTStart(_) |
                    Property::DTEnd(_) |
                    Property::RDate(_) |
                    Property::ExDate(_) |
                    Property::Duration(_) => {
                        new_event.schedule_properties.insert(parsed_property)?;
                    },

                    _ => {
                        new_event.passive_properties.insert(parsed_property)?;
                    },
                }
            }

            Ok(new_event)
        })
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
        timestamp: i64,
        event_occurrence_override: &EventOccurrenceOverride,
    ) -> Result<&Self, String> {
        self.overrides
            .insert(timestamp, event_occurrence_override.clone());

        if let Some(ref mut indexed_categories) = self.indexed_categories {
            if let Some(overridden_categories) = &event_occurrence_override.categories {
                indexed_categories.insert_override(timestamp, overridden_categories);
            }
        } else {
            self.rebuild_indexed_categories()?;
        }

        if let Some(ref mut indexed_related_to) = self.indexed_related_to {
            if let Some(overridden_related_to_set) =
                &event_occurrence_override.build_override_related_to_set()
            {
                indexed_related_to.insert_override(timestamp, overridden_related_to_set);
            }
        } else {
            self.rebuild_indexed_related_to()?;
        }

        if let Some(ref mut indexed_geo) = self.indexed_geo {
            if let Some(overridden_geo) = &event_occurrence_override.geo {
                indexed_geo.insert_override(timestamp, &HashSet::from([overridden_geo.clone()]));
            }
        } else {
            self.rebuild_indexed_geo()?;
        }

        if let Some(ref mut indexed_class) = self.indexed_class {
            if let Some(overridden_class) = &event_occurrence_override.class {
                indexed_class
                    .insert_override(timestamp, &HashSet::from([overridden_class.clone()]));
            }
        } else {
            self.rebuild_indexed_class()?;
        }

        Ok(self)
    }

    pub fn remove_occurrence_override(&mut self, timestamp: i64) -> Result<&Self, String> {
        self.overrides.remove(&timestamp);

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

        Ok(self)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::core::IndexedConclusion;

    use crate::core::utils::UpdatedSetMembers;
    use crate::testing::macros::build_property_from_ical;

    use crate::core::ical::properties::DescriptionProperty;

    use std::collections::BTreeMap;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_indexed_categories() {
        let event = Event {
            uid: String::from("event_UID"),

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
                categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")])),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::new(),
            },

            overrides: BTreeMap::from([
                // Override 100 has all event categories plus CATEGORY_FOUR
                (
                    100,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: None,
                        categories: Some(HashSet::from([
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                            String::from("CATEGORY_THREE"),
                            String::from("CATEGORY_FOUR"),
                        ])),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: None,
                    },
                ),
                // Override 200 has only some event categories (missing CATEGORY_THREE)
                (
                    200,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: None,
                        categories: Some(HashSet::from([
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                        ])),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: None,
                    },
                ),
                // Override 300 has no overridden categories
                (
                    300,
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
                // Override 400 has removed all categories
                (
                    400,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: None,
                        categories: Some(HashSet::new()),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: None,
                    },
                ),
                // Override 500 has no base event categories, but does have CATEGORY_FOUR
                (
                    500,
                    EventOccurrenceOverride {
                        geo: None,
                        class: None,
                        properties: None,
                        categories: Some(HashSet::from([String::from("CATEGORY_FOUR")])),
                        duration: None,
                        dtstart: None,
                        dtend: None,
                        related_to: None,
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

        fn sort_by_category_name(
            array: Vec<(String, Option<IndexedConclusion>)>,
        ) -> Vec<(String, Option<IndexedConclusion>)> {
            let mut sorted_array = array.clone();

            sorted_array.sort_by_key(|(category_name, _)| category_name.clone());

            sorted_array
        }

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
        let ical: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            Event::parse_ical("event_UID", ical).unwrap(),
            Event {
                uid: String::from("event_UID"),

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
                    categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")])),
                    related_to: None,
                },

                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([Property::Description(build_property_from_ical!(DescriptionProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"))]),
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
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH DTSTART:16010101T020000";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID"),

                schedule_properties: ScheduleProperties {
                    rrule: Some(build_property_from_ical!(RRuleProperty, "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")),
                    exrule: None,
                    rdates: None,
                    exdates: None,
                    duration: None,
                    dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:16010101T020000")),
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

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID"),

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
            "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU DTSTART:20201231T183000Z";

        let mut parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        let event_occurrence_override = EventOccurrenceOverride {
            geo:              None,
            class:            None,
            properties:       Some(
                BTreeSet::from([
                    KeyValuePair::new(
                        String::from("DESCRIPTION"),
                        String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                    )
                ])
            ),
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
            related_to:  None
        };

        assert_eq!(
            parsed_event.override_occurrence(1610476200, &event_occurrence_override),
            Ok(
                &Event {
                    uid:                String::from("event_UID"),

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
                                geo:         None,
                                class:       None,
                                properties:  Some(
                                    BTreeSet::from([
                                        KeyValuePair::new(
                                            String::from("DESCRIPTION"),
                                            String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                                        )
                                    ])
                                ),
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
                                related_to:  None
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
                }
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1610476200),
            Ok(&Event {
                uid: String::from("event_UID"),

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
            })
        );
    }

    #[test]
    fn test_related_to() {
        let ical: &str = "RELATED-TO:ParentUID_One RELATED-TO;RELTYPE=PARENT:ParentUID_Two RELATED-TO;RELTYPE=CHILD:ChildUID";

        assert!(Event::parse_ical("event_UID", ical).is_ok());

        let ical: &str = "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One,redical//IndexedCalendar_Two RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three,redical//IndexedCalendar_Two RELATED-TO:ParentUID_One RELATED-TO;RELTYPE=PARENT:ParentUID_Two RELATED-TO;RELTYPE=CHILD:ChildUID";

        let parsed_event = Event::parse_ical("event_UID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uid: String::from("event_UID"),

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
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL;redical//IndexedCalendar_One"),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL;redical//IndexedCalendar_Two"),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL;redical//IndexedCalendar_Three"),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=PARENT;ParentUID_One"),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=PARENT;ParentUID_Two"),
                        build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=CHILD;ChildUID"),
                    ])),
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
}
