use chrono::{TimeZone, Utc};

use std::collections::{HashMap, HashSet, BTreeSet};

use crate::data_types::{Event, EventOccurrenceOverride, KeyValuePair, IndexedConclusion, GeoPoint};

use crate::data_types::event_occurrence_iterator::{EventOccurrenceIterator, LowerBoundFilterCondition, UpperBoundFilterCondition};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EventInstance {
    pub uuid:               String,
    pub dtstart_timestamp:  i64,
    pub dtend_timestamp:    i64,
    pub duration:           i64,
    pub geo:                Option<GeoPoint>,
    pub categories:         Option<HashSet<String>>,
    pub related_to:         Option<HashMap<String, HashSet<String>>>,
    pub passive_properties: BTreeSet<KeyValuePair>,
}

impl EventInstance {

    pub fn new(dtstart_timestamp: &i64, event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> Self {
        EventInstance {
            uuid:               event.uuid.to_owned(),
            dtstart_timestamp:  dtstart_timestamp.to_owned(),
            dtend_timestamp:    Self::get_dtend_timestamp(dtstart_timestamp, event, event_occurrence_override),
            duration:           Self::get_duration(dtstart_timestamp, event, event_occurrence_override),
            geo:                Self::get_geo(event, event_occurrence_override),
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
                String::from("RECURRENCE-ID"),
                format!(";VALUE=DATE-TIME:{}", dtstart_datetime.to_rfc3339()),
            )
        );

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

    fn get_geo(event: &Event, event_occurrence_override: Option<&EventOccurrenceOverride>) -> Option<GeoPoint> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.geo.is_some() {
                return event_occurrence_override.geo.clone();
            }
        }

        event.indexed_properties.geo.clone()
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
    event:         &'a Event,
    internal_iter: EventOccurrenceIterator<'a>,
}

impl<'a> EventInstanceIterator<'a> {
    pub fn new(
        event:                        &'a Event,
        limit:                        Option<u16>,
        filter_from:                  Option<LowerBoundFilterCondition>,
        filter_until:                 Option<UpperBoundFilterCondition>,
        filtering_indexed_conclusion: Option<IndexedConclusion>,
    ) -> Result<EventInstanceIterator<'a>, String> {
        let internal_iter =
            EventOccurrenceIterator::new(
                &event.schedule_properties,
                &event.overrides,
                limit,
                filter_from,
                filter_until,
                filtering_indexed_conclusion.clone(),
            )?;

        Ok(
            EventInstanceIterator {
                event,
                internal_iter,
            }
        )
    }

}

impl<'a> Iterator for EventInstanceIterator<'a> {
    type Item = EventInstance;

    fn next(&mut self) -> Option<Self::Item> {
        // Filter occurrence index iterator timestamps according to IndexedConclusion if
        // present, else include all.
        self.internal_iter
            .next()
            .and_then(
                |(dtstart_timestamp, dtend_timestamp, event_occurrence_override)| {
                    Some(
                        EventInstance::new(
                            &dtstart_timestamp,
                            self.event,
                            event_occurrence_override.as_ref(),
                        )
                    )
                }
            )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::BTreeMap;

    use crate::data_types::{PassiveProperties, IndexedProperties, ScheduleProperties, EventOccurrenceOverrides, OccurrenceCacheValue};

    use crate::parsers::datetime::datestring_to_date;

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
            indexed_geo:         None,
        };

        let event_instance = EventInstance::new(&100, &event, None);

        assert_eq!(
            event_instance,
            EventInstance {
                uuid:               String::from("event_UUID"),
                dtstart_timestamp:  100,
                dtend_timestamp:    160,
                duration:           60,
                geo:                None,
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
                String::from("RECURRENCE-ID;VALUE=DATE-TIME:1970-01-01T00:01:40+00:00"),
                String::from("RELATED_TO;RELTYPE=CHILD:ChildUUID"),
                String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_One"),
                String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_Two"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three"),
                String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"),
                String::from("UUID:event_UUID"),
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
            indexed_geo:         None,
        };

        let event_occurrence_override = EventOccurrenceOverride {
            geo:              None,
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
                uuid:               String::from("event_UUID"),
                dtstart_timestamp:  100,
                dtend_timestamp:    160,
                duration:           60,
                geo:                None,
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
                 String::from("RECURRENCE-ID;VALUE=DATE-TIME:1970-01-01T00:01:40+00:00"),
                 String::from("RELATED_TO;RELTYPE=CHILD:ChildUUID"),
                 String::from("RELATED_TO;RELTYPE=PARENT:ParentUUID_Three"),
                 String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four"),
                 String::from("RELATED_TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                 String::from("UUID:event_UUID"),
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
        assert!(event.schedule_properties.build_parsed_rrule_set().is_ok());

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
                    String::from("DESCRIPTION:OVERRIDDEN description text."),
                    String::from("DTEND:2021-01-05T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-05T18:30:00+00:00"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:2021-01-05T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID"),
                    String::from("UUID:event_UUID"),
                ]
            ),

            (
                1610476200,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-12T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-12T18:30:00+00:00"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:2021-01-12T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID"),
                    String::from("UUID:event_UUID"),
                ]
            ),

            (
                1611081000,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-01-19T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-19T18:30:00+00:00"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:2021-01-19T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:BASE_ParentdUUID"),
                    String::from("UUID:event_UUID"),
                ]
            ),

            (
                1611685800,
                vec![
                    String::from("CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO"),
                    String::from("DESCRIPTION:OVERRIDDEN description text."),
                    String::from("DTEND:2021-01-26T19:00:00+00:00"),
                    String::from("DTSTART:2021-01-26T18:30:00+00:00"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:2021-01-26T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:OVERRIDDEN_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUUID"),
                    String::from("UUID:event_UUID"),
                ]
            ),

            (
                1612290600,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:2021-02-02T19:00:00+00:00"),
                    String::from("DTSTART:2021-02-02T18:30:00+00:00"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:2021-02-02T18:30:00+00:00"),
                    String::from("RELATED_TO;RELTYPE=CHILD:BASE_ChildUUID"),
                    String::from("RELATED_TO;RELTYPE=PARENT:BASE_ParentdUUID"),
                    String::from("UUID:event_UUID"),
                ]
            ),
        ]);

        // Testing without any filtered index conclusion
        let mut event_instance_iterator =
            EventInstanceIterator::new(
                &event,
                None,
                None,
                None,
                None,
            ).unwrap();

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

        // Testing with filtered IndexedConclusion::Include without exceptions
        let mut event_instance_iterator =
            EventInstanceIterator::new(
                &event,
                None,
                None,
                None,
                Some(IndexedConclusion::Include(None)),
            ).unwrap();

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


        // Testing with filtered IndexedConclusion::Include with exceptions
        let mut event_instance_iterator =
            EventInstanceIterator::new(
                &event,
                None,
                None,
                None,
                Some(
                    IndexedConclusion::Include(
                        Some(
                            HashSet::from([
                                1609871400,
                                1611081000,
                                1612290600
                            ])
                        )
                    )
                ),
            ).unwrap();

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
            EventInstanceIterator::new(
                &event,
                None,
                None,
                None,
                Some(IndexedConclusion::Exclude(None)),
            ).unwrap().next(),
            None
        );

        // Testing with filtered IndexedConclusion::Exclude with exceptions
        let mut event_instance_iterator =
            EventInstanceIterator::new(
                &event,
                None,
                None,
                None,
                Some(
                    IndexedConclusion::Exclude(
                        Some(
                            HashSet::from([
                                1609871400,
                                1611081000,
                                1612290600,
                            ])
                        )
                    )
                ),
            ).unwrap();

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
