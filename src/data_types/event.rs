use std::collections::HashMap;

use rrule::{RRuleSet, RRuleError, RRuleSetIter};

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

use crate::data_types::occurrence_index::{OccurrenceIndex, OccurrenceIndexValue, OccurrenceIndexIter};

use crate::data_types::event_occurrence_override::{EventOccurrenceOverride};

use std::collections::BTreeMap;

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
    pub detached: Option<OccurrenceIndex<EventOccurrenceOverride<'a>>>,

    #[serde(borrow)]
    pub current:  Option<OccurrenceIndex<EventOccurrenceOverride<'a>>>,
}

impl<'a> EventOccurrenceOverrides<'a> {
    pub fn new() -> EventOccurrenceOverrides<'a> {
        EventOccurrenceOverrides {
            detached: None,
            current:  None,
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

    pub overrides:           Option<EventOccurrenceOverrides<'a>>,
    pub occurrence_cache:    Option<OccurrenceIndex<OccurrenceIndexValue>>,
}

impl<'a> Event<'a> {
    pub fn new(uuid: String) -> Event<'a> {
        Event {
            uuid,

            schedule_properties: ScheduleProperties::new(),
            indexed_properties:  IndexedProperties::new(),

            passive_properties:  PassiveProperties::new(),

            overrides:           None,
            occurrence_cache:    None,
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

    fn append_to(attribute: &mut Option<Vec<String>>, content: &'a str) {
        let content = String::from(content);

        match attribute {
            Some(properties) => { properties.push(content) },
            None => { *attribute = Some(vec![content]) }
        }
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

mod test {
    use super::*;

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

                overrides:           None,
                occurrence_cache:    None,
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

                overrides:           None,
                occurrence_cache:    None,
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

                overrides:           None,
                occurrence_cache:    None,
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

                overrides:           None,
                occurrence_cache:    None,
            }
        );

        assert_eq!(parsed_event.schedule_properties.validate_rrule(), false);
    }
}
