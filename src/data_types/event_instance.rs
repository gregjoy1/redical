use chrono::{TimeZone, Utc};

use std::collections::{HashMap, HashSet, BTreeSet};

use crate::data_types::{Event, EventOccurrenceOverride, KeyValuePair, IndexedConclusion};

use crate::data_types::occurrence_index::{OccurrenceIndexIter, OccurrenceIndexValue};

#[derive(Debug, PartialEq)]
pub struct EventInstance {
    uuid:               String,
    dtstart_timestamp:  i64,
    dtend_timestamp:    i64,
    duration:           i64,
    categories:         Option<HashSet<String>>,
    related_to:         Option<HashMap<String, HashSet<String>>>,
    passive_properties: BTreeSet<KeyValuePair>,
}

impl EventInstance {

    pub fn new(dtstart_timestamp: &i64, event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> Self {
        EventInstance {
            uuid:               Self::get_uuid(dtstart_timestamp, event),
            dtstart_timestamp:  dtstart_timestamp.to_owned(),
            dtend_timestamp:    Self::get_dtend_timestamp(dtstart_timestamp, event, event_occurrence_override),
            duration:           Self::get_duration(dtstart_timestamp, event, event_occurrence_override),
            categories:         Self::get_categories(event, event_occurrence_override),
            related_to:         Self::get_related_to(event, event_occurrence_override),
            passive_properties: Self::get_passive_properties(event, event_occurrence_override),
        }
    }

    pub fn serialize_to_ical(&self) -> Vec<String> {
        self.serialize_to_ical_set()
            .iter()
            .map(|key_value_pair| key_value_pair.to_string())
            .collect()
    }

    pub fn serialize_to_ical_set(&self) -> BTreeSet<KeyValuePair> {
        let mut serialized_output = self.passive_properties.clone();

        serialized_output.insert(
            KeyValuePair::new(
                String::from("UUID"),
                format!(":{}", self.uuid),
            )
        );

        // TODO: handle the error case...
        let dtstart_datetime = Utc.timestamp_opt(self.dtstart_timestamp, 0).unwrap();

        serialized_output.insert(
            KeyValuePair::new(
                String::from("DTSTART"),
                format!(":{}", dtstart_datetime.to_rfc3339()),
            )
        );

        // TODO: handle the error case...
        let dtend_datetime = Utc.timestamp_opt(self.dtend_timestamp, 0).unwrap();

        serialized_output.insert(
            KeyValuePair::new(
                String::from("DTEND"),
                format!(":{}", dtend_datetime.to_rfc3339()),
            )
        );

        if let Some(categories) = &self.categories {
            let mut categories: Vec<String> = Vec::from_iter(
                categories.iter()
                          .map(|element| element.to_owned())
            );

            categories.sort();

            if categories.len() > 0 {
                serialized_output.insert(
                    KeyValuePair::new(
                        String::from("CATEGORIES"),
                        format!(":{}", categories.join(","))
                    )
                );
            }
        }

        if let Some(related_to) = &self.related_to {
            for (reltype, reltype_uuids) in related_to {
                if reltype_uuids.is_empty() {
                    continue;
                }

                let mut reltype_uuids: Vec<String> = Vec::from_iter(
                    reltype_uuids.iter()
                                 .map(|element| element.to_owned())
                );

                reltype_uuids.sort();

                reltype_uuids.iter().for_each(|reltype_uuid| {
                    serialized_output.insert(
                        KeyValuePair::new(
                            String::from("RELATED_TO"),
                            format!(";RELTYPE={}:{}", reltype, reltype_uuid)
                        )
                    );
                });
            }
        }

        serialized_output
    }

    fn get_uuid(dtstart_timestamp: &i64, event: &Event) -> String {
        format!("{}-{}", event.uuid, dtstart_timestamp)
    }

    fn get_dtend_timestamp(dtstart_timestamp: &i64, event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> i64 {
        dtstart_timestamp + Self::get_duration(dtstart_timestamp, event, event_occurrence_override)
    }

    fn get_duration(dtstart_timestamp: &i64, event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> i64 {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Ok(Some(overridden_duration)) = event_occurrence_override.get_duration(&dtstart_timestamp) {
                return overridden_duration;
            }
        }

       if let Ok(Some(event_duration)) = event.schedule_properties.get_duration() {
           return event_duration;
       }

       0
    }

    fn get_categories(event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> Option<HashSet<String>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Some(overridden_categories) = &event_occurrence_override.categories {
                return Some(overridden_categories.clone());
            }
        }

        event.indexed_properties.categories.clone()
    }

    fn get_related_to(event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> Option<HashMap<String, HashSet<String>>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Some(overridden_related_to) = &event_occurrence_override.related_to {
                return Some(overridden_related_to.clone());
            }
        }

        event.indexed_properties.related_to.clone()
    }

    // This gets all the product of all the passive properties overridden by property name.
    // As these are stored in an ordered set of KeyValuePairs we get the overridden passive
    // properties and then iterate over the base event passive properties, checking for the
    // presence of the base event passive property name key, and inserting it if it is not found.
    fn get_passive_properties(event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> BTreeSet<KeyValuePair> {
        let mut passive_properties = event_occurrence_override
                                                                 .and_then(|event_occurrence_override| event_occurrence_override.properties.clone())
                                                                 .unwrap_or(BTreeSet::new());

        // This searches for the presence of the base event passsive property name key in all the overrides:
        // If found:
        //  Skip
        //
        // If not found:
        //  Add the base event property
        for base_property in &event.passive_properties.properties {
            match passive_properties.iter().find(|passive_property| passive_property.key == base_property.key) {
                Some(_) => {
                    continue;
                },

                None => {
                    passive_properties.insert(base_property.clone());
                }
            }
        }

        passive_properties
    }
}

#[derive(Debug)]
pub struct EventInstanceIterator<'a> {
    event:                       &'a Event,
    internal_iter:               Option<OccurrenceIndexIter<'a, OccurrenceIndexValue>>,
    filtered_indexed_conclusion: Option<&'a IndexedConclusion>,
}

impl<'a> EventInstanceIterator<'a> {
    pub fn new(event: &'a Event, filtered_indexed_conclusion: Option<&'a IndexedConclusion>) -> Self {
        let internal_iter = match &event.occurrence_cache {
            Some(occurrence_cache) => Some(occurrence_cache.iter()),
            None => None,
        };

        EventInstanceIterator {
            event,
            internal_iter,
            filtered_indexed_conclusion,
        }
    }

}

impl<'a> Iterator for EventInstanceIterator<'a> {
    type Item = EventInstance;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.internal_iter {
            Some(iterator) => {
                // Filter occurrence index iterator timestamps according to IndexedConclusion if
                // present, else include all.
                iterator.filter(|&(dtstart_timestamp, _)| {
                            if let Some(indexed_conclusion) = self.filtered_indexed_conclusion {
                                indexed_conclusion.include_event_occurrence(dtstart_timestamp)
                            } else {
                                true
                            }
                        })
                        .next()
                        .and_then(
                            |(dtstart_timestamp, occurrence_index_value)| {
                                match occurrence_index_value {
                                    OccurrenceIndexValue::Occurrence => {
                                        Some(EventInstance::new(&dtstart_timestamp, self.event, None))
                                    },

                                    OccurrenceIndexValue::Override => {
                                        let event_occurrence_override = self.event.overrides.current.get(dtstart_timestamp.to_owned());

                                        Some(EventInstance::new(&dtstart_timestamp, self.event, event_occurrence_override))
                                    },
                                }
                            }
                        )
            },

            None => None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::BTreeMap;

    use crate::data_types::{PassiveProperties, IndexedProperties, ScheduleProperties, EventOccurrenceOverrides, OccurrenceIndex, OccurrenceIndexValue};

    use crate::parsers::datestring_to_date;

    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    #[test]
    fn test_event_instance_without_override() {
        let event = Event {
            uuid: String::from("event_UUID"),

            // Ends 60 seconds after it starts.
            schedule_properties: ScheduleProperties {
                rrule:            None,
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
                dtend:            Some(
                    HashSet::from([
                        KeyValuePair::new(
                            String::from("DTEND"),
                            String::from(":20201231T183100Z"),
                        )
                    ])
                ),
            },

            indexed_properties:  IndexedProperties {
                categories:       Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ])
                ),
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
            },

            passive_properties:  PassiveProperties {
                properties: BTreeSet::from([
                                KeyValuePair::new(
                                    String::from("DESCRIPTION"),
                                    String::from(":Event description text."),
                                ),

                                KeyValuePair::new(
                                    String::from("LOCATION"),
                                    String::from(":Event address text."),
                                ),
                ])
            },

            overrides:           EventOccurrenceOverrides::new(),
            occurrence_cache:    None,
            indexed_categories:  None,
            indexed_related_to:  None,
        };

        let event_instance = EventInstance::new(&100, &event, None);

        assert_eq!(
            event_instance,
            EventInstance {
                uuid:               String::from("event_UUID-100"),
                dtstart_timestamp:  100,
                dtend_timestamp:    160,
                duration:           60,
                categories:         Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ])
                ),
                related_to:         Some(
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
                passive_properties: BTreeSet::from([
                    KeyValuePair::new(
                        String::from("DESCRIPTION"),
                        String::from(":Event description text."),
                    ),

                    KeyValuePair::new(
                        String::from("LOCATION"),
                        String::from(":Event address text."),
                    ),
                ])
            }
        );

        assert_eq!(
            event_instance.serialize_to_ical(),
            vec![
                String::from("CATEGORIES:CATEGORY THREE,CATEGORY_ONE,CATEGORY_TWO"),
                String::from("DESCRIPTION:Event description text."),
                String::from("DTEND:1970-01-01T00:02:40+00:00"),
                String::from("DTSTART:1970-01-01T00:01:40+00:00"),
                String::from("LOCATION:Event address text."),
                String::from("RELATED_TO;RELTYPE=CHILD:ChildUUID"),
                String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_One"),
                String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_Two"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"),
                String::from("UUID:event_UUID-100"),
            ]
        );
    }

    #[test]
    fn test_event_instance_with_override() {
        let event = Event {
            uuid: String::from("event_UUID"),

            // Ends 60 seconds after it starts.
            schedule_properties: ScheduleProperties {
                rrule:            None,
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
                dtend:            Some(
                    HashSet::from([
                        KeyValuePair::new(
                            String::from("DTEND"),
                            String::from(":20201231T183100Z"),
                        )
                    ])
                ),
            },

            indexed_properties:  IndexedProperties {
                categories:       Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ])
                ),
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
            },

            passive_properties:  PassiveProperties {
                properties: BTreeSet::from([
                                KeyValuePair::new(
                                    String::from("DESCRIPTION"),
                                    String::from(":Event description text."),
                                ),

                                KeyValuePair::new(
                                    String::from("LOCATION"),
                                    String::from(":Event address text."),
                                ),
                ])
            },

            overrides:           EventOccurrenceOverrides::new(),
            occurrence_cache:    None,
            indexed_categories:  None,
            indexed_related_to:  None,
        };

        let event_occurrence_override = EventOccurrenceOverride {
            properties:       Some(
                BTreeSet::from([
                    KeyValuePair::new(
                        String::from("LOCATION"),
                        String::from(":Overridden Event address text."),
                    )
                ])
            ),
            categories:       Some(
                HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_FOUR"),
                ])
            ),
            duration:         None,
            dtstart:          None,
            dtend:            None,
            description:      None,
            related_to:       Some(
                HashMap::from([
                    (
                        String::from("X-IDX-CAL"),
                        HashSet::from([
                            String::from("redical//IndexedCalendar_One"),
                            String::from("redical//IndexedCalendar_Four"),
                        ])
                    ),
                    (
                        String::from("PARENT"),
                        HashSet::from([
                            String::from("ParentUUID_Three"),
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
        };

        let event_instance = EventInstance::new(&100, &event, Some(&event_occurrence_override));

        assert_eq!(
            event_instance,
            EventInstance {
                uuid:               String::from("event_UUID-100"),
                dtstart_timestamp:  100,
                dtend_timestamp:    160,
                duration:           60,
                categories:         Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_FOUR"),
                    ])
                ),
                related_to:         Some(
                    HashMap::from([
                        (
                            String::from("X-IDX-CAL"),
                            HashSet::from([
                                String::from("redical//IndexedCalendar_One"),
                                String::from("redical//IndexedCalendar_Four"),
                            ])
                        ),
                        (
                            String::from("PARENT"),
                            HashSet::from([
                                String::from("ParentUUID_Three"),
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
                passive_properties: BTreeSet::from([
                    KeyValuePair::new(
                        String::from("DESCRIPTION"),
                        String::from(":Event description text."),
                    ),

                    KeyValuePair::new(
                        String::from("LOCATION"),
                        String::from(":Overridden Event address text."),
                    ),
                ])
            }
        );

        assert_eq!(
            event_instance.serialize_to_ical(),
            vec![
                 String::from("CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE"),
                 String::from("DESCRIPTION:Event description text."),
                 String::from("DTEND:1970-01-01T00:02:40+00:00"),
                 String::from("DTSTART:1970-01-01T00:01:40+00:00"),
                 String::from("LOCATION:Overridden Event address text."),
                 String::from("RELATED_TO;RELTYPE=CHILD:ChildUUID"),
                 String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_Three"),
                 String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four"),
                 String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                 String::from("UUID:event_UUID-100"),
            ]
        );
    }

    #[test]
    fn test_event_instance_iterator() {
        let mut event = Event::parse_ical(
            "event_UUID",
            [
                "DESCRIPTION:BASE description text.",
                "DTSTART:20210105T183000Z",
                "DTEND:20210105T190000Z",
                "RRULE:FREQ=WEEKLY;UNTIL=20210202T183000Z;INTERVAL=1",
                "CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO",
                "RELATED-TO;RELTYPE=PARENT:BASE_ParentdUUID",
                "RELATED-TO;RELTYPE=CHILD:BASE_ChildUUID",
            ].join(" ").as_str()
        ).unwrap();

        assert!(event.rebuild_occurrence_cache(65_535).is_ok());

        assert!(
            event.override_occurrence(
                datestring_to_date("20210105T183000Z", None, "").unwrap().timestamp(),
                &EventOccurrenceOverride::parse_ical(
                    [
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID",
                    ].join(" ").as_str()
                ).unwrap()
            ).is_ok()
        );

        assert!(
            event.override_occurrence(
                datestring_to_date("20210112T183000Z", None, "").unwrap().timestamp(),
                &EventOccurrenceOverride::parse_ical(
                    [
                        "RELATED-TO;RELTYPE=CHILD:BASE_ChildUUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID",
                    ].join(" ").as_str()
                ).unwrap()
            ).is_ok()
        );

        assert!(
            event.override_occurrence(
                datestring_to_date("20210126T183000Z", None, "").unwrap().timestamp(),
                &EventOccurrenceOverride::parse_ical(
                    [
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID",
                    ].join(" ").as_str()
                ).unwrap()
            ).is_ok()
        );

        assert!(event.rebuild_indexed_categories().is_ok());
        assert!(event.rebuild_indexed_related_to().is_ok());

        let expected_event_instances_ical = HashMap::from([
            (
                1609871400,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-05T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-05T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID"),
                    String::from("UUID:event_UUID-1609871400"),
                ]
            ),

            (
                1610476200,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-12T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-12T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID"),
                    String::from("UUID:event_UUID-1610476200"),
                ]
            ),

            (
                1611081000,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-19T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-19T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:BASE_ParentdUUID"),
                    String::from("UUID:event_UUID-1611081000"),
                ]
            ),

            (
                1611685800,
                vec![
                    String::from("CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-26T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-26T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID"),
                    String::from("UUID:event_UUID-1611685800"),
                ]
            ),

            (
                1612290600,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-02-02T19:00:00+00:00"),
                    String::from("DTSTART:2021-02-02T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:BASE_ParentdUUID"),
                    String::from("UUID:event_UUID-1612290600"),
                ]
            ),
        ]);

        fn assert_iterator_returns_all_event_instances(mut event_instance_iterator: EventInstanceIterator, expected_event_instances_ical: &HashMap<i64, Vec<String>>) {
            assert_eq!(
                event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
                Some(expected_event_instances_ical[&1609871400].clone())
            );

            assert_eq!(
                event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
                Some(expected_event_instances_ical[&1610476200].clone())
            );

            assert_eq!(
                event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
                Some(expected_event_instances_ical[&1611081000].clone())
            );

            assert_eq!(
                event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
                Some(expected_event_instances_ical[&1611685800].clone())
            );

            assert_eq!(
                event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
                Some(expected_event_instances_ical[&1612290600].clone())
            );

            assert_eq!(event_instance_iterator.next(), None);
        }

        // Testing without any filtered index conclusion
        assert_iterator_returns_all_event_instances(EventInstanceIterator::new(&event, None), &expected_event_instances_ical);

        // Testing with filtered IndexedConclusion::Include without exceptions
        assert_iterator_returns_all_event_instances(EventInstanceIterator::new(&event, Some(&IndexedConclusion::Include(None))), &expected_event_instances_ical);

        // Testing with filtered IndexedConclusion::Include with exceptions
        let indexed_conclusion = IndexedConclusion::Include(Some(HashSet::from([1609871400, 1611081000, 1612290600])));

        let mut event_instance_iterator = EventInstanceIterator::new(&event, Some(&indexed_conclusion));

        assert_eq!(
            event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
            Some(expected_event_instances_ical[&1611685800].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);

        // Testing with filtered IndexedConclusion::Exclude without exceptions
        assert_eq!(
            EventInstanceIterator::new(&event, Some(&IndexedConclusion::Exclude(None))).next(),
            None
        );

        // Testing with filtered IndexedConclusion::Exclude with exceptions
        let indexed_conclusion = IndexedConclusion::Exclude(Some(HashSet::from([1609871400, 1611081000, 1612290600])));

        let mut event_instance_iterator = EventInstanceIterator::new(&event, Some(&indexed_conclusion));

        assert_eq!(
            event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator.next().and_then(|event_instance| Some(event_instance.serialize_to_ical())),
            Some(expected_event_instances_ical[&1612290600].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);
    }
}
