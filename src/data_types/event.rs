use std::collections::{HashSet, HashMap};

use rrule::{RRuleSet, RRuleError, RRuleSetIter};

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

use crate::data_types::occurrence_index::{OccurrenceIndex, OccurrenceIndexValue, OccurrenceIndexIter};

use crate::data_types::event_occurrence_override::{EventOccurrenceOverride};

use std::collections::BTreeMap;

use crate::data_types::inverted_index::IndexedEvent;

fn property_option_set_or_insert<'a>(property_option: &mut Option<Vec<String>>, content: &'a str) {
    let content = String::from(content);

    match property_option {
        Some(properties) => { properties.push(content) },
        None => { *property_option = Some(vec![content]) }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverrides<'a> {
    #[serde(borrow)]
    pub detached: OccurrenceIndex<EventOccurrenceOverride<'a>>,

    #[serde(borrow)]
    pub current:  OccurrenceIndex<EventOccurrenceOverride<'a>>,
}

impl<'a> EventOccurrenceOverrides<'a> {
    pub fn new() -> EventOccurrenceOverrides<'a> {
        EventOccurrenceOverrides {
            detached: OccurrenceIndex::new(),
            current:  OccurrenceIndex::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct ScheduleProperties {
    pub rrule:       Option<Vec<String>>,
    pub exrule:      Option<Vec<String>>,
    pub rdate:       Option<Vec<String>>,
    pub exdate:      Option<Vec<String>>,
    pub duration:    Option<Vec<String>>,
    pub dtstart:     Option<Vec<String>>,
    pub dtend:       Option<Vec<String>>,
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
    pub related_to:  Option<Vec<String>>,
    pub categories:  Option<Vec<String>>
}

impl IndexedProperties {
    pub fn new() -> IndexedProperties {
        IndexedProperties {
            related_to:  None,
            categories:  None
        }
    }

    pub fn insert(&mut self, property: ParsedProperty) -> Result<&Self, String> {
        match property {
            ParsedProperty::Categories(content)  => {
                match content.value {
                    ParsedValue::List(list) => {
                        list.iter().for_each(|category| {
                            property_option_set_or_insert(&mut self.categories, *category);
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
                property_option_set_or_insert(&mut self.related_to, content.content_line);

                Ok(self)
            },

            _ => {
                Err(String::from("Expected indexable property (CATEGORIES, RELATED_TO), received: {property.content_line}"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct PassiveProperties<'a> {
    #[serde(borrow)]
    pub properties: HashMap<&'a str, Vec<String>>
}

impl<'a> PassiveProperties<'a> {
    pub fn new() -> PassiveProperties<'a> {
        PassiveProperties {
            properties:  HashMap::new(),
        }
    }

    pub fn insert(&mut self, property: ParsedProperty<'a>) -> Result<&Self, String> {
        match property {
            ParsedProperty::Description(content) | ParsedProperty::Other(content)  => {
                self.properties.entry(&content.name.unwrap())
                               .and_modify(|content_lines| content_lines.push(String::from(content.content_line)))
                               .or_insert(vec![String::from(content.content_line)]);

                Ok(self)
            },

            _ => {
                Err(String::from("Expected passive property, received: {property.content_line}"))
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event<'a> {
    pub uuid:                String,

    pub schedule_properties: ScheduleProperties,
    pub indexed_properties:  IndexedProperties,

    #[serde(borrow)]
    pub passive_properties:  PassiveProperties<'a>,

    pub overrides:           EventOccurrenceOverrides<'a>,
    pub occurrence_cache:    Option<OccurrenceIndex<OccurrenceIndexValue>>,
    pub indexed_categories:  Option<IndexedCategories>,
}

impl<'a> Event<'a> {
    pub fn new(uuid: String) -> Event<'a> {
        Event {
            uuid,

            schedule_properties: ScheduleProperties::new(),
            indexed_properties:  IndexedProperties::new(),

            passive_properties:  PassiveProperties::new(),

            overrides:           EventOccurrenceOverrides::new(),
            occurrence_cache:    None,
            indexed_categories:  None
        }
    }

    pub fn parse_ical<'de: 'a>(uuid: &str, input: &str) -> Result<Event<'a>, String> {
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

    pub fn override_occurrence(&mut self, timestamp: i64, event_occurrence_override: &'a EventOccurrenceOverride) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {

                match occurrence_cache.get(timestamp) {
                    Some(OccurrenceIndexValue::Occurrence) => {
                        // TODO: update indexes
                        occurrence_cache.insert(timestamp, OccurrenceIndexValue::Override);

                        self.overrides.current.insert(timestamp, event_occurrence_override.clone());
                    },
                    Some(OccurrenceIndexValue::Override) => {
                        // TODO: update indexes

                        self.overrides.current.insert(timestamp, event_occurrence_override.clone());
                    },
                    None => {
                        return Err(String::from(format!("No overridable occurrence exists for timestamp: {timestamp}")));
                    }
                }

            },
            None => {
                return Err(String::from(format!("No overridable occurrence exists for timestamp: {timestamp}")));
            }
        }

        Ok(self)
    }

    pub fn remove_occurrence_override(&mut self, timestamp: i64) -> Result<&Self, String> {
        match &mut self.occurrence_cache {
            Some(occurrence_cache) => {

                match occurrence_cache.get(timestamp) {
                    Some(OccurrenceIndexValue::Occurrence) => {
                        // TODO: update indexes

                        return Err(String::from(format!("No occurrence override exists for timestamp: {timestamp}")));
                    },
                    Some(OccurrenceIndexValue::Override) => {
                        // TODO: update indexes

                        occurrence_cache.insert(timestamp, OccurrenceIndexValue::Occurrence);

                        self.overrides.current.remove(timestamp);
                    },
                    None => {
                        return Err(String::from(format!("No overridable occurrence exists for timestamp: {timestamp}")));
                    }
                }

            },
            None => {
                return Err(String::from(format!("No overridable occurrence exists for timestamp: {timestamp}")));
            }
        }

        Ok(self)
    }

    pub fn rebuild_occurrence_cache(&mut self, max_count: usize) -> Result<&Self, RRuleError> {
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct IndexedCategories {
    pub categories: HashMap<String, IndexedEvent>
}

impl<'a> From<&Event<'a>> for IndexedCategories {

    fn from(event: &Event) -> IndexedCategories {
        let mut indexed_categories: HashMap<String, IndexedEvent> = HashMap::new();

        match event.indexed_properties.categories.clone() {
            Some(categories) => {
                for category in categories.iter() {
                    indexed_categories.insert(category.clone(), IndexedEvent::Include(None));
                }
            },
            _ => {}
        }

        let indexed_categories_set: HashSet<String> = indexed_categories.clone().into_keys().collect();

        for (timestamp, event_override) in event.overrides.current.iter() {
            match &event_override.categories {
                Some(override_categories) => {
                    let override_categories_set: HashSet<String> = override_categories.clone().iter().map(|category| category.clone()).collect();

                    for excluded_category in indexed_categories_set.difference(&override_categories_set) {
                        indexed_categories.get_mut(excluded_category).and_then(|indexed_category| Some(indexed_category.insert_exception(timestamp)));
                    }

                    for included_category in override_categories_set.difference(&indexed_categories_set) {
                        indexed_categories.entry(included_category.clone())
                                          .and_modify(|indexed_category| {
                                              indexed_category.insert_exception(timestamp);
                                          })
                                          .or_insert(IndexedEvent::Exclude(Some(HashSet::from([timestamp]))));
                    }
                },
                None => {
                    continue;
                }
            }
        }

        IndexedCategories {
            categories: indexed_categories
        }
    }

}

mod test {
    use super::*;

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
                    vec![
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY_THREE")
                    ]
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
                                properties:  HashMap::from([]),
                                categories:  Some(
                                    vec![
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                        String::from("CATEGORY_THREE"),
                                        String::from("CATEGORY_FOUR"),
                                    ]
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
                                properties:  HashMap::from([]),
                                categories:  Some(
                                    vec![
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                    ]
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
                                properties:  HashMap::from([]),
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
                                properties:  HashMap::from([]),
                                categories:  Some(vec![]),
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
                                properties:  HashMap::from([]),
                                categories:  Some(
                                    vec![
                                        String::from("CATEGORY_FOUR"),
                                    ]
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
        };

        assert_eq!(
            IndexedCategories::from(&event),
            IndexedCategories {
                categories: HashMap::from([
                                (
                                    String::from("CATEGORY_ONE"),
                                    IndexedEvent::Include(Some(HashSet::from([400, 500]))),
                                ),
                                (
                                    String::from("CATEGORY_TWO"),
                                    IndexedEvent::Include(Some(HashSet::from([400, 500]))),
                                ),
                                (
                                    String::from("CATEGORY_THREE"),
                                    IndexedEvent::Include(Some(HashSet::from([200, 400, 500]))),
                                ),
                                (
                                    String::from("CATEGORY_FOUR"),
                                    IndexedEvent::Exclude(Some(HashSet::from([100, 500]))),
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
                        vec![
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ]
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
                        vec![
                            String::from("CATEGORY_ONE"),
                            String::from("CATEGORY_TWO"),
                            String::from("CATEGORY THREE")
                        ]
                    ),
                    related_to:       None,
                },

                passive_properties:  PassiveProperties {
                    properties: HashMap::from(
                                    [
                                        (
                                            "DESCRIPTION",
                                            vec![
                                                String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                                            ]
                                        )
                                    ]
                                )
                },

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
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
                        vec![
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                        ]
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        vec![
                            String::from("DTSTART:20201231T183000Z")
                        ]
                    ),
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
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
                        vec![
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ]
                    ),
                    exrule:           None,
                    rdate:            None,
                    exdate:           None,
                    duration:         None,
                    dtstart:          Some(
                        vec![
                            String::from("DTSTART:16010101T020000")
                        ]
                    ),
                    dtend:            None,
                },

                indexed_properties:  IndexedProperties::new(),

                passive_properties:  PassiveProperties::new(),

                overrides:           EventOccurrenceOverrides::new(),
                occurrence_cache:    None,
                indexed_categories:  None,
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
                        vec![
                            String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                        ]
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
            properties:  HashMap::from([]),
            categories:  Some(
                vec![
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                    String::from("CATEGORY THREE")
                ]
            ),
            duration:    None,
            dtstart:     None,
            dtend:       None,
            description: Some(String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")),
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
                            vec![
                                String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                            ]
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            vec![
                                String::from("DTSTART:20201231T183000Z")
                            ]
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
                                properties:  HashMap::from([]),
                                categories:  Some(
                                    vec![
                                        String::from("CATEGORY_ONE"),
                                        String::from("CATEGORY_TWO"),
                                        String::from("CATEGORY THREE")
                                    ]
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
                    indexed_categories:  None,
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
                            vec![
                                String::from("RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU")
                            ]
                        ),
                        exrule:           None,
                        rdate:            None,
                        exdate:           None,
                        duration:         None,
                        dtstart:          Some(
                            vec![
                                String::from("DTSTART:20201231T183000Z")
                            ]
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
                    indexed_categories:  None,
                }
            )
        );
    }
}
