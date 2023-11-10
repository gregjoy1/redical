use std::collections::{HashSet, HashMap, BTreeSet, BTreeMap};

use rrule::{RRuleSet, RRuleError};

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::parsers::ical_properties::{parse_properties, ParsedProperty};
use crate::parsers::ical_common::ParsedValue;

use crate::parsers::datetime::{datestring_to_date, ParseError};

use crate::data_types::occurrence_cache::{OccurrenceCache, OccurrenceCacheValue};

use crate::data_types::event_occurrence_override::EventOccurrenceOverride;

use crate::data_types::inverted_index::InvertedEventIndex;

use crate::data_types::geo_index::GeoPoint;

use crate::data_types::event_diff::EventDiff;

use crate::data_types::utils::KeyValuePair;

fn property_option_set_or_insert<'a>(property_option: &mut Option<HashSet<KeyValuePair>>, content: KeyValuePair) {
    match property_option {
        Some(properties) => { properties.insert(content); },
        None => { *property_option = Some(HashSet::from([content])); }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverrides {
    pub detached: BTreeMap<i64, EventOccurrenceOverride>,
    pub current:  BTreeMap<i64, EventOccurrenceOverride>,
}

impl EventOccurrenceOverrides {
    pub fn new() -> EventOccurrenceOverrides {
        EventOccurrenceOverrides {
            detached: BTreeMap::new(),
            current:  BTreeMap::new(),
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
                    for removed_reltype_uuid_pair in indexed_related_to.removed.iter() {
                        if let Some(reltype_uuids) = overridden_related_to.get_mut(&removed_reltype_uuid_pair.key) {
                            reltype_uuids.remove(&removed_reltype_uuid_pair.value);
                        }
                    }

                    for added_reltype_uuid_pair in indexed_related_to.added.iter() {
                        overridden_related_to.entry(added_reltype_uuid_pair.key.clone())
                                             .and_modify(|reltype_uuids| { reltype_uuids.insert(added_reltype_uuid_pair.value.clone()); })
                                             .or_insert(HashSet::from([added_reltype_uuid_pair.value.clone()]));
                    }
                },

                None => {
                    let mut overridden_related_to = HashMap::new();

                    for added_reltype_uuid_pair in indexed_related_to.added.iter() {
                        overridden_related_to.entry(added_reltype_uuid_pair.key.clone())
                                             .and_modify(|reltype_uuids: &mut HashSet<String>| { reltype_uuids.insert(added_reltype_uuid_pair.value.clone()); })
                                             .or_insert(HashSet::from([added_reltype_uuid_pair.value.clone()]));
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
                },

                None => {
                    event_occurrence_override.properties = Some(
                        BTreeSet::from_iter(
                            indexed_passive_properties.added
                                                      .iter()
                                                      .map(|added_key_value_pair| added_key_value_pair.clone())
                        )
                    );
                }
            };
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ScheduleProperties {
    pub rrule:            Option<HashSet<KeyValuePair>>,
    pub exrule:           Option<HashSet<KeyValuePair>>,
    pub rdate:            Option<HashSet<KeyValuePair>>,
    pub exdate:           Option<HashSet<KeyValuePair>>,
    pub duration:         Option<HashSet<KeyValuePair>>,
    pub dtstart:          Option<HashSet<KeyValuePair>>,
    pub dtend:            Option<HashSet<KeyValuePair>>,
    pub parsed_rrule_set: Option<rrule::RRuleSet>,
}

impl ScheduleProperties {
    pub fn new() -> ScheduleProperties {
        ScheduleProperties {
            rrule:            None,
            exrule:           None,
            rdate:            None,
            exdate:           None,
            duration:         None,
            dtstart:          None,
            dtend:            None,
            parsed_rrule_set: None,
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

    pub fn parse_rrule(&self) -> Result<RRuleSet, RRuleError> {
        let mut is_missing_rules = true;
        let mut ical_parts = vec![];

        if let Some(rrules_content_lines) = &self.rrule {
            rrules_content_lines.iter().for_each(|rrule_content_line| {
                is_missing_rules = false;

                ical_parts.push(rrule_content_line.to_string());
            });
        }

        if let Some(exrules_content_lines) = &self.exrule {
            exrules_content_lines.iter().for_each(|exrule_content_line| {
                is_missing_rules = false;

                ical_parts.push(exrule_content_line.to_string());
            });
        }

        if let Some(rdates_content_lines) = &self.rdate {
            rdates_content_lines.iter().for_each(|rdate_content_line| {
                is_missing_rules = false;

                ical_parts.push(rdate_content_line.to_string());
            });
        }

        if let Some(exdates_content_lines) = &self.exdate {
            exdates_content_lines.iter().for_each(|exdate_content_line| {
                is_missing_rules = false;

                ical_parts.push(exdate_content_line.to_string());
            });
        }

        if let Some(dtstart_content_lines) = &self.dtstart {
            // TODO: There should not be more than one DTSTART properties, raise validation error.
            dtstart_content_lines.iter().for_each(|dtstart_content_line| {
                ical_parts.push(dtstart_content_line.to_string());

                // If parsed ical does not contain any RRULE or RDATE properties, we need to
                // artifically create them based on the specified DTSTART properties so that the
                // rrule_set date extrapolation works, even for a single date.
                if is_missing_rules {
                    let rdate_content_line = KeyValuePair::new(String::from("RDATE"), dtstart_content_line.value.clone());

                    ical_parts.push(rdate_content_line.to_string());
                }
            });
        }

        // TODO: Add COUNT

        ical_parts.join("\n").parse::<RRuleSet>()
    }

    pub fn get_dtstart_timestamp(&self) -> Result<Option<i64>, ParseError> {
        if let Some(properties) = self.dtstart.as_ref() {
            if let Some(datetime) = properties.iter().next() {
                let parsed_datetime = datetime.to_string().replace(&String::from("DTSTART:"), &String::from(""));

                return match datestring_to_date(&parsed_datetime, None, "DTSTART") {
                    Ok(datetime) => Ok(Some(datetime.timestamp())),
                    Err(error) => Err(error),
                };
            }
        }

        Ok(None)
    }

    pub fn get_dtend_timestamp(&self) -> Result<Option<i64>, ParseError> {
        if let Some(properties) = self.dtend.as_ref() {
            if let Some(datetime) = properties.iter().next() {
                let parsed_datetime = datetime.to_string().replace(&String::from("DTEND:"), &String::from(""));

                return match datestring_to_date(&parsed_datetime, None, "DTEND") {
                    Ok(datetime) => Ok(Some(datetime.timestamp())),
                    Err(error) => Err(error),
                };
            }
        }

        Ok(None)
    }

    pub fn get_duration(&self) -> Result<Option<i64>, ParseError> {
        if let Some(properties) = self.duration.as_ref() {
            if let Some(_duration) = properties.iter().next() {
                // TODO: implement this
                return Ok(Some(0));
            }
        }

        match (self.get_dtstart_timestamp(), self.get_dtend_timestamp()) {
            (Ok(Some(dtstart_timestamp)), Ok(Some(dtend_timestamp))) => {
                Ok(Some(dtend_timestamp - dtstart_timestamp))
            },

            _ => Ok(None),
        }
    }

    pub fn build_parsed_rrule_set(&mut self) -> Result<(), rrule::RRuleError> {
        let parsed_rrule_set = self.parse_rrule()?;

        self.parsed_rrule_set = Some(parsed_rrule_set);

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct IndexedProperties {
    pub geo:         Option<GeoPoint>,
    pub related_to:  Option<HashMap<String, HashSet<String>>>,
    pub categories:  Option<HashSet<String>>
}

impl IndexedProperties {
    pub fn new() -> IndexedProperties {
        IndexedProperties {
            geo:         None,
            related_to:  None,
            categories:  None,
        }
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::Geo(content) => {
                if let ParsedValue::Pair((latitude, longitude)) = content.value {
                    match (longitude.parse::<f64>(), latitude.parse::<f64>()) {
                        (Ok(parsed_longitude), Ok(parsed_latitude)) => {
                            let geo_point = GeoPoint::from(
                                (
                                    parsed_longitude,
                                    parsed_latitude,
                                )
                            );

                            geo_point.validate()?;

                            self.geo = Some(geo_point);

                            Ok(self)
                        },

                        _ => {
                            return Err(format!("Expected latitude, longitude. Could not parse float."));
                        }
                    }
                } else {
                    return Err(String::from("Expected latitude, longitude"));
                }
            },

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
    pub properties: BTreeSet<KeyValuePair>
}

impl PassiveProperties {
    pub fn new() -> PassiveProperties {
        PassiveProperties {
            properties:  BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::Description(content) | ParsedProperty::Other(content)  => {
                self.properties.insert(content.content_line);

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
    pub occurrence_cache:    Option<OccurrenceCache>,
    pub indexed_categories:  Option<InvertedEventIndex<String>>,
    pub indexed_related_to:  Option<InvertedEventIndex<KeyValuePair>>,
    pub indexed_geo:         Option<InvertedEventIndex<GeoPoint>>,
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
            indexed_geo:         None,
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

    pub fn rebuild_indexed_categories(&mut self) -> Result<&mut Self, String> {
        self.indexed_categories = Some(
            InvertedEventIndex::<String>::new_from_event_categories(self)
        );

        Ok(self)
    }

    pub fn rebuild_indexed_related_to(&mut self) -> Result<&mut Self, String> {
        self.indexed_related_to = Some(
            InvertedEventIndex::<KeyValuePair>::new_from_event_related_to(self)
        );

        Ok(self)
    }

    pub fn override_occurrence(&mut self, timestamp: i64, event_occurrence_override: &EventOccurrenceOverride) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {
                let occurrences = &mut occurrence_cache.occurrences;
                let overridden_duration = event_occurrence_override.get_duration(&timestamp).unwrap_or(None);

                if occurrences.contains_key(&timestamp) {
                    occurrences.insert(timestamp, OccurrenceCacheValue::Override(overridden_duration));

                    self.overrides.current.insert(timestamp, event_occurrence_override.clone());
                } else {
                    return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
                }

                if let Some(ref mut indexed_categories) = self.indexed_categories {

                    if let Some(overridden_categories) = &event_occurrence_override.categories {
                        indexed_categories.insert_override(timestamp, overridden_categories);
                    }

                } else {
                    self.rebuild_indexed_categories()?;
                }

                if let Some(ref mut indexed_related_to) = self.indexed_related_to {

                    if let Some(overridden_related_to_set) = &event_occurrence_override.build_override_related_to_set() {
                        indexed_related_to.insert_override(timestamp, overridden_related_to_set);
                    }

                } else {
                    self.rebuild_indexed_related_to()?;
                }
            },
            None => {
                return Err(format!("No overridable occurrence exists for timestamp: {timestamp}"));
            }
        }

        Ok(self)
    }

    pub fn remove_occurrence_override(&mut self, timestamp: i64) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {
                let occurrences = &mut occurrence_cache.occurrences;

                match occurrences.get(&timestamp) {
                    Some(OccurrenceCacheValue::Occurrence) => {
                        return Err(format!("No occurrence override exists for timestamp: {timestamp}"));
                    },
                    Some(OccurrenceCacheValue::Override(_)) => {
                        occurrences.insert(timestamp, OccurrenceCacheValue::Occurrence);

                        self.overrides.current.remove(&timestamp);

                        if let Some(ref mut indexed_categories) = self.indexed_categories {
                            indexed_categories.remove_override(timestamp);
                        } else {
                            self.indexed_categories = Some(
                                InvertedEventIndex::<String>::new_from_event_categories(&*self)
                            );
                        }

                        if let Some(ref mut indexed_related_to) = self.indexed_related_to {
                            indexed_related_to.remove_override(timestamp);
                        } else {
                            self.indexed_related_to = Some(
                                InvertedEventIndex::<KeyValuePair>::new_from_event_related_to(&*self)
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
        let base_duration = self.schedule_properties.get_duration().unwrap_or(None);
        let rrule_set = self.schedule_properties.parse_rrule()?;
        let rrule_set_iter = rrule_set.into_iter();

        let mut occurrence_cache: OccurrenceCache = OccurrenceCache::new(base_duration);

        let max_datetime = self.get_max_datetime();

        for next_datetime in rrule_set_iter.take(max_count) {
            if next_datetime.gt(&max_datetime) {
                break;
            }

            occurrence_cache.occurrences.insert(next_datetime.timestamp(), OccurrenceCacheValue::Occurrence);
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

    use crate::data_types::IndexedConclusion;

    use crate::data_types::utils::{UpdatedSetMembers, UpdatedAttribute};

    use std::collections::BTreeMap;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_indexed_categories() {
        let event = Event {
            uuid: String::from("event_UUID"),

            schedule_properties: ScheduleProperties {
                rrule:            None,
                exrule:           None,
                rdate:            None,
                exdate:           None,
                duration:         None,
                dtstart:          None,
                dtend:            None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo:        None,
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
                properties: BTreeSet::new()
            },

            overrides: EventOccurrenceOverrides {
                detached: BTreeMap::new(),
                current:  BTreeMap::from(
                    [
                        // Override 100 has all event categories plus CATEGORY_FOUR
                        (
                            100,
                            EventOccurrenceOverride {
                                geo:         None,
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
                                related_to:  None
                            }
                        ),

                        // Override 200 has only some event categories (missing CATEGORY_THREE)
                        (
                            200,
                            EventOccurrenceOverride {
                                geo:         None,
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
                                related_to:  None
                            }
                        ),

                        // Override 300 has no overridden categories
                        (
                            300,
                            EventOccurrenceOverride {
                                geo:         None,
                                properties:  None,
                                categories:  None,
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                related_to:  None
                            }
                        ),

                        // Override 400 has removed all categories
                        (
                            400,
                            EventOccurrenceOverride {
                                geo:         None,
                                properties:  None,
                                categories:  Some(HashSet::new()),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                related_to:  None
                            }
                        ),

                        // Override 500 has no base event categories, but does have CATEGORY_FOUR
                        (
                            500,
                            EventOccurrenceOverride {
                                geo:         None,
                                properties:  None,
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_FOUR"),
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
                                related_to:  None
                            }
                        ),
                    ]
                ),
            },
            occurrence_cache:   None,
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo:        None,
        };

        let mut indexed_categories = InvertedEventIndex::<String>::new_from_event_categories(&event);

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

        fn sort_by_category_name(array: Vec<(String, Option<IndexedConclusion>)>) -> Vec<(String, Option<IndexedConclusion>)> {
            let mut sorted_array = array.clone();

            sorted_array.sort_by_key(|(category_name, _)| category_name.clone());

            sorted_array
        }

        indexed_categories.insert_override(
            600,
            &HashSet::from([
                String::from("CATEGORY_ONE"),
                String::from("CATEGORY_FIVE"),
            ])
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
            Event::parse_ical("event_UUID", ical).unwrap(),
            Event {
                uuid: String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                            )
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          None,
                    dtend:            None,
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties {
                    geo:              None,
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
                    properties: BTreeSet::from([
                                    KeyValuePair::new(
                                        String::from("DESCRIPTION"),
                                        String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"),
                                    )
                                ])
                },

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
                indexed_geo:         None,
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
                            KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU"),
                            )
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("DTSTART"),
                                String::from(":20201231T183000Z"),
                            )
                        ])
                    ),
                    dtend:            None,
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
                indexed_geo:         None,
            }
        );

        assert!(
            parsed_event.rebuild_occurrence_cache(100).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_cache,
            Some(
                OccurrenceCache {
                    base_duration: 0,
                    occurrences:   BTreeMap::from(
                        [
                            (1609871400, OccurrenceCacheValue::Occurrence),
                            (1610476200, OccurrenceCacheValue::Occurrence),
                            (1611081000, OccurrenceCacheValue::Occurrence),
                            (1611685800, OccurrenceCacheValue::Occurrence),
                            (1612290600, OccurrenceCacheValue::Occurrence),
                            (1612895400, OccurrenceCacheValue::Occurrence),
                            (1613500200, OccurrenceCacheValue::Occurrence),
                            (1614105000, OccurrenceCacheValue::Occurrence),
                            (1614709800, OccurrenceCacheValue::Occurrence),
                            (1615314600, OccurrenceCacheValue::Occurrence),
                            (1615919400, OccurrenceCacheValue::Occurrence),
                            (1616524200, OccurrenceCacheValue::Occurrence),
                            (1617129000, OccurrenceCacheValue::Occurrence),
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
                OccurrenceCache {
                    base_duration: 0,
                    occurrences:   BTreeMap::from(
                        [
                            (1609871400, OccurrenceCacheValue::Occurrence),
                            (1610476200, OccurrenceCacheValue::Occurrence),
                        ]
                    )
                }
            )
        );
    }

    #[test]
    fn test_build_parsed_rrule_set() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH DTSTART:16010101T020000";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                            )
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("DTSTART"),
                                String::from(":16010101T020000"),
                            )
                        ])
                    ),
                    dtend:            None,
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
                indexed_geo:         None,
            }
        );

        assert!(parsed_event.schedule_properties.build_parsed_rrule_set().is_ok());

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:                String::from("event_UUID"),

                schedule_properties: ScheduleProperties {
                    rrule:            Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"),
                            )
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          None,
                    dtend:            None,
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_geo:         None,
                indexed_related_to:  None,
            }
        );

        assert!(parsed_event.schedule_properties.build_parsed_rrule_set().is_err());
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
                OccurrenceCache {
                    base_duration: 0,
                    occurrences:   BTreeMap::from(
                        [
                            (1609871400, OccurrenceCacheValue::Occurrence),
                            (1610476200, OccurrenceCacheValue::Occurrence),
                        ]
                    )
                }
            )
        );

        let event_occurrence_override = EventOccurrenceOverride {
            geo:              None,
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
            parsed_event.override_occurrence(1234, &event_occurrence_override),
            Err(
                String::from("No overridable occurrence exists for timestamp: 1234")
            )
        );

        assert_eq!(
            parsed_event.override_occurrence(1610476200, &event_occurrence_override),
            Ok(
                &Event {
                    uuid:                String::from("event_UUID"),

                    schedule_properties: ScheduleProperties {
                        rrule:            Some(
                            HashSet::from([
                                KeyValuePair::new(
                                    String::from("RRULE"),
                                    String::from(":FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU"),
                                )
                            ])
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            HashSet::from([
                                KeyValuePair::new(
                                    String::from("DTSTART"),
                                    String::from(":20201231T183000Z"),
                                )
                            ])
                        ),
                        dtend:            None,
                        parsed_rrule_set: None,
                    },

                    indexed_properties:  IndexedProperties::new(),

                    passive_properties:  PassiveProperties::new(),

                    overrides:           EventOccurrenceOverrides {
                        detached: BTreeMap::new(),
                        current:  BTreeMap::from([
                            (
                                1610476200,
                                EventOccurrenceOverride {
                                    geo:         None,
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
                    },
                    occurrence_cache:    Some(
                        OccurrenceCache {
                            base_duration: 0,
                            occurrences:   BTreeMap::from(
                                [
                                    (1609871400, OccurrenceCacheValue::Occurrence),
                                    (1610476200, OccurrenceCacheValue::Override(None)),
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
                    ),
                    indexed_geo:         None,
                }
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1234),
            Err(
                String::from("No overridable occurrence exists for timestamp: 1234")
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1609871400),
            Err(
                String::from("No occurrence override exists for timestamp: 1609871400")
            )
        );

        assert_eq!(
            parsed_event.remove_occurrence_override(1610476200),
            Ok(
                &Event {
                    uuid:                String::from("event_UUID"),

                    schedule_properties: ScheduleProperties {
                        rrule:            Some(
                            HashSet::from([
                                KeyValuePair::new(
                                    String::from("RRULE"),
                                    String::from(":FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU"),
                                )
                            ])
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            HashSet::from([
                                KeyValuePair::new(
                                    String::from("DTSTART"),
                                    String::from(":20201231T183000Z"),
                                )
                            ])
                        ),
                        dtend:            None,
                        parsed_rrule_set: None,
                    },

                    indexed_properties:  IndexedProperties::new(),

                    passive_properties:  PassiveProperties::new(),

                    overrides:           EventOccurrenceOverrides {
                        detached: BTreeMap::new(),
                        current:  BTreeMap::new(),
                    },
                    occurrence_cache:    Some(
                        OccurrenceCache {
                            base_duration: 0,
                            occurrences:   BTreeMap::from(
                                [
                                    (1609871400, OccurrenceCacheValue::Occurrence),
                                    (1610476200, OccurrenceCacheValue::Occurrence),
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
                    indexed_geo:         None,
                }
            )
        );
    }

    #[test]
    fn test_related_to() {
        let ical: &str = "RELATED-TO:ParentUUID_One RELATED-TO;RELTYPE=PARENT:ParentUUID_Two RELATED-TO;RELTYPE=CHILD:ChildUUID";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

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
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties {
                    geo:        None,
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
                indexed_geo:         None,
            }
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
                            KeyValuePair::new(
                                String::from("RRULE"),
                                String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                            )
                        ])
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        HashSet::from([
                            KeyValuePair::new(
                                String::from("DTSTART"),
                                String::from(":20201231T183000Z"),
                            )
                        ])
                    ),
                    dtend:            None,
                    parsed_rrule_set: None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
                indexed_related_to:  None,
                indexed_geo:         None,
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
            detached: BTreeMap::from([
                          (
                              1610476300,
                              EventOccurrenceOverride {
                                  geo:         None,
                                  properties:  None,
                                  categories:  None,
                                  duration:    None,
                                  dtstart:     None,
                                  dtend:       None,
                                  related_to:  None,
                              }
                          )
            ]),
            current:  BTreeMap::from([
                (
                    1610476200,
                    EventOccurrenceOverride {
                        geo:         None,
                        properties:  Some(
                            BTreeSet::from([
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
            )
                ])
        };

        let event_diff = EventDiff {
            indexed_categories:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([String::from("CATEGORY_THREE"), String::from("CATEGORY_FIVE")]),
                    maintained: HashSet::from([String::from("CATEGORY_ONE"), String::from("CATEGORY_TWO")]),
                    added:      HashSet::from([String::from("CATEGORY_FOUR")])
                }
            ),
            indexed_related_to:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([KeyValuePair::new(String::from("PARENT"), String::from("PARENT_UUID_ONE"))]),
                    maintained: HashSet::from([
                            KeyValuePair::new(String::from("PARENT"), String::from("PARENT_UUID_TWO")),
                            KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UUID_ONE")),
                            KeyValuePair::new(String::from("CHILD"), String::from("CHILD_UUID_TWO")),
                    ]),
                    added:      HashSet::from([
                        KeyValuePair::new(String::from("X-IDX-CAL"), String::from("INDEXED_CALENDAR_UUID")),
                    ])
                }
            ),
            indexed_geo:         None,
            passive_properties:  Some(
                UpdatedSetMembers {
                    removed:    HashSet::from([
                        KeyValuePair {
                            key:   String::from("X-PROPERTY-TWO"),
                            value: String::from(":PROPERTY_VALUE_TWO")
                        }
                    ]),
                    maintained: HashSet::from([
                        KeyValuePair {
                            key:   String::from("X-PROPERTY-ONE"),
                            value: String::from(":PROPERTY_VALUE_ONE")
                        },
                        KeyValuePair {
                            key:   String::from("X-PROPERTY-ONE"),
                            value: String::from(":PROPERTY_VALUE_TWO")
                        },
                        KeyValuePair {
                            key:   String::from("X-PROPERTY-TWO"),
                            value: String::from(":PROPERTY_VALUE_ONE")
                        },
                    ]),
                    added:      HashSet::from([
                        KeyValuePair {
                            key:   String::from("X-PROPERTY-THREE"),
                            value: String::from(":PROPERTY_VALUE_ONE")
                        },
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
                    detached: BTreeMap::from([
                        (
                            1610476300,
                            EventOccurrenceOverride {
                                geo:        None,
                                properties: Some(
                                    BTreeSet::from([
                                        KeyValuePair::new(
                                            String::from("X-PROPERTY-THREE"),
                                            String::from(":PROPERTY_VALUE_ONE"),
                                        )
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
                        )
                    ]),
                    current:  BTreeMap::from([
                        (
                            1610476200,
                            EventOccurrenceOverride {
                                geo:        None,
                                properties: Some(
                                    BTreeSet::from([
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
                                    ])
                                ),
                                categories:  Some(
                                    HashSet::from([
                                        String::from("CATEGORY_FOUR"),
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                    ])
                                ),
                                duration:    None,
                                dtstart:     None,
                                dtend:       None,
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
                        )
                    ]),
                }
            )
        );
    }
}
