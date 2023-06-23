use std::collections::HashMap;

use rrule::{RRuleSet, RRuleError, RRuleSetIter};

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

use crate::data_types::occurrence_index::{OccurrenceIndex, OccurrenceIndexValue, OccurrenceIndexIter};

use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event<'a> {
    pub uuid:        String,
    pub categories:  Option<Vec<String>>,
    pub rrule:       Option<Vec<String>>,
    pub exrule:      Option<Vec<String>>,
    pub rdate:       Option<Vec<String>>,
    pub exdate:      Option<Vec<String>>,
    pub duration:    Option<Vec<String>>,
    pub dtstart:     Option<Vec<String>>,
    pub dtend:       Option<Vec<String>>,
    pub description: Option<Vec<String>>,
    pub related_to:  Option<Vec<String>>,

    #[serde(borrow)]
    pub properties:  HashMap<&'a str, Vec<String>>,

    pub occurrence_index: Option<OccurrenceIndex<OccurrenceIndexValue>>,
}

impl<'a> Event<'a> {
    pub fn new(uuid: String) -> Event<'a> {
        Event {
            uuid,
            properties:       HashMap::new(),
            categories:       None,
            rrule:            None,
            exrule:           None,
            rdate:            None,
            exdate:           None,
            duration:         None,
            dtstart:          None,
            dtend:            None,
            description:      None,
            related_to:       None,
            occurrence_index: None,
        }
    }

    pub fn parse_ical<'de: 'a>(uuid: &str, input: &str) -> Result<Event<'a>, String> {
        match parse_properties(input) {
            Ok((_, parsed_properties)) => {
                let new_event: &mut Event = &mut Event::new(String::from(uuid));

                parsed_properties.into_iter()
                    .for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Categories(content)  => {
                                match content.value {
                                    ParsedValue::List(list) => {
                                        list.iter().for_each(|category| {
                                            Event::append_to(&mut new_event.categories, *category)
                                        });
                                    },
                                    _ => {}
                                }
                            },
                            ParsedProperty::RRule(content)       => { Event::append_to(&mut new_event.rrule, content.content_line) },
                            ParsedProperty::ExRule(content)      => { Event::append_to(&mut new_event.exrule, content.content_line) },
                            ParsedProperty::RDate(content)       => { Event::append_to(&mut new_event.rdate, content.content_line) },
                            ParsedProperty::ExDate(content)      => { Event::append_to(&mut new_event.exdate, content.content_line) },
                            ParsedProperty::Duration(content)    => { Event::append_to(&mut new_event.duration, content.content_line) },
                            ParsedProperty::DtStart(content)     => { Event::append_to(&mut new_event.dtstart, content.content_line) },
                            ParsedProperty::DtEnd(content)       => { Event::append_to(&mut new_event.dtend, content.content_line) },
                            ParsedProperty::Description(content) => { Event::append_to(&mut new_event.description, content.content_line) },
                            ParsedProperty::RelatedTo(content)   => { Event::append_to(&mut new_event.related_to, content.content_line) },
                            ParsedProperty::Other(content)       => { } // TODO
                        }
                    });

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

    pub fn rebuild_occurrence_index(&mut self, max_count: usize) -> Result<&Self, RRuleError> {
        let rrule_set = self.parse_rrule()?;
        let rrule_set_iter = rrule_set.into_iter();

        let mut occurrence_index: OccurrenceIndex<OccurrenceIndexValue> = OccurrenceIndex::new();

        let max_datetime = self.get_max_datetime();

        for next_datetime in rrule_set_iter.take(max_count) {
            if next_datetime.gt(&max_datetime) {
                break;
            }

            occurrence_index.insert(next_datetime.timestamp(), OccurrenceIndexValue::Occurrence);
        }

        self.occurrence_index = Some(occurrence_index);

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

    fn validate_rrule(&self) -> bool {
        self.parse_rrule().is_ok()
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
                uuid:             String::from("event_UUID"),
                properties:       HashMap::from([]),
                categories:       Some(
                    vec![
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ]
                ),
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
                description:      Some(
                    vec![
                        String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                    ]
                ),
                related_to:       None,
                occurrence_index: None,
            }
        );
    }

    #[test]
    fn retest_build_occurrence_index() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20210331T183000Z;INTERVAL=1;BYDAY=TU DTSTART:20201231T183000Z";

        let mut parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:             String::from("event_UUID"),
                properties:       HashMap::from([]),
                categories:       None,
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
                description:      None,
                related_to:       None,
                occurrence_index: None,
            }
        );

        assert!(
            parsed_event.rebuild_occurrence_index(100).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_index,
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
            parsed_event.rebuild_occurrence_index(2).is_ok()
        );

        assert_eq!(
            parsed_event.occurrence_index,
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
                uuid:             String::from("event_UUID"),
                properties:       HashMap::from([]),
                categories:       None,
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
                description:      None,
                related_to:       None,
                occurrence_index: None,
            }
        );

        assert!(parsed_event.validate_rrule());

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        let parsed_event = Event::parse_ical("event_UUID", ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                uuid:             String::from("event_UUID"),
                properties:       HashMap::from([]),
                categories:       None,
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
                description:      None,
                related_to:       None,
                occurrence_index: None,
            }
        );

        assert_eq!(parsed_event.validate_rrule(), false);
    }
}
