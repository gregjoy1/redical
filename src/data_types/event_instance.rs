use std::collections::{HashMap, HashSet, BTreeSet};

use crate::data_types::{Event, EventOccurrenceOverride, KeyValuePair};

#[derive(Debug, PartialEq)]
pub struct EventInstance<'a> {
    dtstart_timestamp:         i64,
    event:                     &'a Event,
    event_occurrence_override: Option<&'a EventOccurrenceOverride>,
}

impl<'a> EventInstance<'a> {

    pub fn new(dtstart_timestamp: i64, event: &'a Event, event_occurrence_override: Option<&'a EventOccurrenceOverride>) -> Self {
        EventInstance {
            dtstart_timestamp,
            event,
            event_occurrence_override,
        }
    }

    pub fn get_uuid(&self) -> String {
        format!("{}-{}", self.event.uuid, self.dtstart_timestamp)
    }

    pub fn get_dtend_timestamp(&self) -> i64 {
        self.dtstart_timestamp + self.get_duration()
    }

    pub fn get_duration(&self) -> i64 {
        if let Some(event_occurrence_override) = self.event_occurrence_override {
            if let Ok(Some(overridden_duration)) = event_occurrence_override.get_duration(&self.dtstart_timestamp) {
                return overridden_duration;
            }
        }

       if let Ok(Some(event_duration)) = self.event.schedule_properties.get_duration() {
           return event_duration;
       }

       0
    }

    pub fn get_categories(&self) -> Option<HashSet<String>> {
        if let Some(event_occurrence_override) = self.event_occurrence_override {
            if let Some(overridden_categories) = &event_occurrence_override.categories {
                return Some(overridden_categories.clone());
            }
        }

        self.event.indexed_properties.categories.clone()
    }

    pub fn get_related_to(&self) -> Option<HashMap<String, HashSet<String>>> {
        if let Some(event_occurrence_override) = self.event_occurrence_override {
            if let Some(overridden_related_to) = &event_occurrence_override.related_to {
                return Some(overridden_related_to.clone());
            }
        }

        self.event.indexed_properties.related_to.clone()
    }

    // This gets all the product of all the passive properties overridden by property name.
    // As these are stored in an ordered set of KeyValuePairs we get the overridden passive
    // properties and then iterate over the base event passive properties, checking for the
    // presence of the base event passive property name key, and inserting it if it is not found.
    pub fn get_passive_properties(&self) -> BTreeSet<KeyValuePair> {
        let mut passive_properties = self.event_occurrence_override
                                                                 .and_then(|event_occurrence_override| event_occurrence_override.properties.clone())
                                                                 .unwrap_or(BTreeSet::new());

        // This searches for the presence of the base event passsive property name key in all the overrides:
        // If found:
        //  Skip
        //
        // If not found:
        //  Add the base event property
        for base_property in &self.event.passive_properties.properties {
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

#[cfg(test)]
mod test {
    use super::*;

    use crate::data_types::{PassiveProperties, IndexedProperties, ScheduleProperties, EventOccurrenceOverrides};

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

        let event_instance = EventInstance::new(100, &event, None);

        assert_eq!(
            event_instance,
            EventInstance {
                dtstart_timestamp:         100,
                event:                     &event,
                event_occurrence_override: None,
            }
        );

        assert_eq!(
            event_instance.get_uuid(),
            String::from("event_UUID-100")
        );


        assert_eq!(
            event_instance.get_dtend_timestamp(),
            160
        );

        assert_eq!(
            event_instance.get_duration(),
            60
        );

        assert_eq!(
            event_instance.get_categories(),
            Some(
                HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                    String::from("CATEGORY THREE")
                ])
            )
        );

        assert_eq!(
            event_instance.get_related_to(),
            Some(
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
            )
        );

        assert_eq!(
            event_instance.get_passive_properties(),
            BTreeSet::from([
                KeyValuePair::new(
                    String::from("DESCRIPTION"),
                    String::from(":Event description text."),
                ),

                KeyValuePair::new(
                    String::from("LOCATION"),
                    String::from(":Event address text."),
                ),
            ])
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

        let event_instance = EventInstance::new(100, &event, Some(&event_occurrence_override));

        assert_eq!(
            event_instance,
            EventInstance {
                dtstart_timestamp:         100,
                event:                     &event,
                event_occurrence_override: Some(&event_occurrence_override),
            }
        );

        assert_eq!(
            event_instance.get_uuid(),
            String::from("event_UUID-100")
        );


        assert_eq!(
            event_instance.get_dtend_timestamp(),
            160
        );

        assert_eq!(
            event_instance.get_duration(),
            60
        );

        assert_eq!(
            event_instance.get_categories(),
            Some(
                HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_FOUR"),
                ])
            )
        );

        assert_eq!(
            event_instance.get_related_to(),
            Some(
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
            )
        );

        assert_eq!(
            event_instance.get_passive_properties(),
            BTreeSet::from([
                KeyValuePair::new(
                    String::from("DESCRIPTION"),
                    String::from(":Event description text."),
                ),

                KeyValuePair::new(
                    String::from("LOCATION"),
                    String::from(":Overridden Event address text."),
                ),
            ])
        );
    }
}
