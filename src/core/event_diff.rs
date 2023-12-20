use crate::core::{UpdatedSetMembers, Event, hashmap_to_hashset, btree_hashset_to_hashset, KeyValuePair, UpdatedAttribute, GeoPoint};
use crate::core::parsers::duration::ParsedDuration;

use std::collections::HashSet;

use std::hash::Hash;

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct EventDiff {
    pub indexed_categories:  Option<UpdatedSetMembers<String>>,
    pub indexed_related_to:  Option<UpdatedSetMembers<KeyValuePair>>,
    pub indexed_geo:         Option<UpdatedAttribute<GeoPoint>>,

    pub passive_properties:  Option<UpdatedSetMembers<KeyValuePair>>,
    pub schedule_properties: Option<SchedulePropertiesDiff>,
}

impl EventDiff {

    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        EventDiff {
            indexed_categories:  Self::diff_indexed_categories(original_event, updated_event),
            indexed_related_to:  Self::diff_indexed_related_to(original_event, updated_event),
            indexed_geo:         Self::diff_indexed_geo(original_event, updated_event),

            passive_properties:  Self::diff_passive_properties(original_event, updated_event),
            schedule_properties: Self::diff_schedule_properties(original_event, updated_event),
        }
    }

    fn diff_indexed_categories(original_event: &Event, updated_event: &Event) -> Option<UpdatedSetMembers<String>> {
        Some(
            UpdatedSetMembers::new(
                original_event.indexed_properties.categories.as_ref(),
                updated_event.indexed_properties.categories.as_ref()
            )
        )
    }

    fn diff_indexed_related_to(original_event: &Event, updated_event: &Event) -> Option<UpdatedSetMembers<KeyValuePair>> {
        let original_related_to = hashmap_to_hashset(original_event.indexed_properties.related_to.as_ref());
        let updated_related_to  = hashmap_to_hashset(updated_event.indexed_properties.related_to.as_ref());

        Some(
            UpdatedSetMembers::new(
                original_related_to.as_ref(),
                updated_related_to.as_ref()
            )
        )
    }

    fn diff_indexed_geo(original_event: &Event, updated_event: &Event) -> Option<UpdatedAttribute<GeoPoint>> {
        let original_geo = &original_event.indexed_properties.geo;
        let updated_geo  = &updated_event.indexed_properties.geo;

        if original_geo.is_none() && updated_geo.is_none() {
            None
        } else {
            Some(
                UpdatedAttribute::new(
                    original_geo,
                    updated_geo,
                )
            )
        }
    }

    fn diff_passive_properties(original_event: &Event, updated_event: &Event) -> Option<UpdatedSetMembers<KeyValuePair>> {
        // TODO: Improve this to be 0 copy
        let original_passive_properties = btree_hashset_to_hashset(Some(&original_event.passive_properties.properties));
        let updated_passive_properties  = btree_hashset_to_hashset(Some(&updated_event.passive_properties.properties));

        Some(
            UpdatedSetMembers::new(
                original_passive_properties.as_ref(),
                updated_passive_properties.as_ref()
            )
        )
    }

    fn diff_schedule_properties(original_event: &Event, updated_event: &Event) -> Option<SchedulePropertiesDiff> {
        Some(
            SchedulePropertiesDiff::new(original_event, updated_event)
        )
    }

}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct SchedulePropertiesDiff {
    rrule:    Option<UpdatedAttribute<KeyValuePair>>,
    exrule:   Option<UpdatedAttribute<KeyValuePair>>,
    rdate:    Option<UpdatedAttribute<KeyValuePair>>,
    exdate:   Option<UpdatedAttribute<KeyValuePair>>,
    duration: Option<UpdatedAttribute<ParsedDuration>>,
    dtstart:  Option<UpdatedAttribute<KeyValuePair>>,
    dtend:    Option<UpdatedAttribute<KeyValuePair>>,
}

impl SchedulePropertiesDiff {
    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        let original_event_schedule_properties = &original_event.schedule_properties;
        let updated_event_schedule_properties  = &updated_event.schedule_properties;

        SchedulePropertiesDiff {
            rrule:    Self::build_updated_attribute(&original_event_schedule_properties.rrule,    &updated_event_schedule_properties.rrule),
            exrule:   Self::build_updated_attribute(&original_event_schedule_properties.exrule,   &updated_event_schedule_properties.exrule),
            rdate:    Self::build_updated_attribute(&original_event_schedule_properties.rdate,    &updated_event_schedule_properties.rdate),
            exdate:   Self::build_updated_attribute(&original_event_schedule_properties.exdate,   &updated_event_schedule_properties.exdate),
            duration: Self::build_updated_attribute(&original_event_schedule_properties.duration, &updated_event_schedule_properties.duration),
            dtstart:  Self::build_updated_attribute(&original_event_schedule_properties.dtstart,  &updated_event_schedule_properties.dtstart),
            dtend:    Self::build_updated_attribute(&original_event_schedule_properties.dtend,    &updated_event_schedule_properties.dtend),
        }
    }

    fn build_updated_attribute<T>(original: &Option<T>, updated: &Option<T>) -> Option<UpdatedAttribute<T>>
    where
        T: Eq + PartialEq + Clone
    {
        match (original, updated) {
            (None, None) => None,

            _ => Some(UpdatedAttribute::new(original, updated))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{HashMap, BTreeSet, BTreeMap};

    use crate::core::{ScheduleProperties, IndexedProperties, PassiveProperties, EventOccurrenceOverride, KeyValuePair};

    #[test]
    fn test_event_diff() {
        // Test when no changes between both original and updated Events
        let original_event = Event::new(String::from("event_UUID"));
        let updated_event  = Event::new(String::from("event_UUID"));

        let expected_indexed_categories = Some(
            UpdatedSetMembers {
                removed:    HashSet::new(),
                maintained: HashSet::new(),
                added:      HashSet::new()
            }
        );

        let expected_indexed_related_to = Some(
            UpdatedSetMembers {
                removed:    HashSet::new(),
                maintained: HashSet::new(),
                added:      HashSet::new()
            }
        );

        let expected_indexed_geo = None;

        let expected_passive_properties = Some(
            UpdatedSetMembers {
                removed:    HashSet::new(),
                maintained: HashSet::new(),
                added:      HashSet::new()
            }
        );

        let expected_schedule_properties = Some(
            SchedulePropertiesDiff {
                rrule:    None,
                exrule:   None,
                rdate:    None,
                exdate:   None,
                duration: None,
                dtstart:  None,
                dtend:    None
            }
        );

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories:  expected_indexed_categories,
                indexed_related_to:  expected_indexed_related_to,
                indexed_geo:         expected_indexed_geo,
                passive_properties:  expected_passive_properties,
                schedule_properties: expected_schedule_properties,
            }
        );

        // Test changes between blank original Event and populated updated Event
        let updated_event = Event {
            uuid: String::from("event_UUID"),

            schedule_properties: ScheduleProperties {
                rrule:            Some(
                    KeyValuePair::new(
                        String::from("RRULE"),
                        String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                    )
                ),
                exrule:           None,
                rdate:            None,
                exdate:           None,
                duration:         None,
                dtstart:          Some(
                    KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(":20201231T183000Z"),
                    )
                ),
                dtend:            None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo:        Some(GeoPoint::from((-0.1278f64, 51.5074f64))),
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
                properties: BTreeSet::from([
                                KeyValuePair::new(
                                    String::from("DESCRIPTION"),
                                    String::from("Testing description text."),
                                )
                ])
            },

            overrides:          BTreeMap::new(),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo:        None,
        };

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories: Some(
                                         UpdatedSetMembers {
                                             removed:    HashSet::new(),
                                             maintained: HashSet::new(),
                                             added:      HashSet::from([
                                                 String::from("CATEGORY_ONE"),
                                                 String::from("CATEGORY_TWO"),
                                                 String::from("CATEGORY_THREE"),
                                             ])
                                         }
                                     ),
                indexed_related_to:  Some(
                                         UpdatedSetMembers {
                                             removed:    HashSet::new(),
                                             maintained: HashSet::new(),
                                             added:      HashSet::new(),
                                         }
                                     ),
                indexed_geo:         Some(UpdatedAttribute::Added(GeoPoint::from((-0.1278f64, 51.5074f64)))),
                passive_properties:  Some(
                                        UpdatedSetMembers {
                                            removed:    HashSet::new(),
                                            maintained: HashSet::new(),
                                            added:      HashSet::from([
                                                KeyValuePair {
                                                    key:   String::from("DESCRIPTION"),
                                                    value: String::from("Testing description text.")
                                                }
                                            ])
                                        }
                                     ),
                schedule_properties: Some(
                                        SchedulePropertiesDiff {
                                            rrule:    Some(
                                                          UpdatedAttribute::Added(
                                                              KeyValuePair::new(
                                                                  String::from("RRULE"),
                                                                  String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                                                              )
                                                          )
                                                      ),
                                            exrule:   None,
                                            rdate:    None,
                                            exdate:   None,
                                            duration: None,
                                            dtstart:  Some(
                                                          UpdatedAttribute::Added(
                                                              KeyValuePair::new(
                                                                  String::from("DTSTART"),
                                                                  String::from(":20201231T183000Z"),
                                                              )
                                                          )
                                                      ),
                                            dtend:    None
                                        }
                                     )
            }
        );

        // Test changes between populated original and updated Events (with removals).
        let original_event = Event {
            uuid: String::from("event_UUID"),

            schedule_properties: ScheduleProperties {
                rrule:            Some(
                                      KeyValuePair::new(
                                          String::from("RRULE"),
                                          String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
                                      )
                                  ),
                exrule:           None,
                rdate:            None,
                exdate:           None,
                duration:         None,
                dtstart:          Some(
                                     KeyValuePair::new(
                                         String::from("DTSTART"),
                                         String::from(":20201131T183000Z"),
                                     )
                                  ),
                dtend:            None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo:        None,
                related_to: Some(
                    HashMap::from([
                        (
                            String::from("X-IDX-CAL"),
                            HashSet::from([
                                String::from("indexed_calendar_UUID"),
                            ])
                        ),
                        (
                            String::from("PARENT"),
                            HashSet::from([
                                String::from("another_event_UUID"),
                            ])
                        ),
                    ])
                ),
                categories: Some(
                    HashSet::from([
                        String::from("CATEGORY_THREE"),
                        String::from("CATEGORY_FOUR"),
                    ])
                ),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::from([
                                KeyValuePair::new(
                                    String::from("DESCRIPTION"),
                                    String::from("Testing original description text."),
                                )
                ])
            },

            overrides:          BTreeMap::new(),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo:        None,
        };

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories:  Some(
                                         UpdatedSetMembers {
                                             removed:    HashSet::from([String::from("CATEGORY_FOUR")]),
                                             maintained: HashSet::from([String::from("CATEGORY_THREE")]),
                                             added:      HashSet::from([String::from("CATEGORY_ONE"), String::from("CATEGORY_TWO")])
                                         }
                                     ),
                indexed_related_to:  Some(
                    UpdatedSetMembers {
                        removed:    HashSet::from([
                                        KeyValuePair::new(String::from("X-IDX-CAL"), String::from("indexed_calendar_UUID")),
                                        KeyValuePair::new(String::from("PARENT"),    String::from("another_event_UUID")),
                        ]),
                        maintained: HashSet::new(),
                        added:      HashSet::new()
                    }
                ),
                indexed_geo:         Some(UpdatedAttribute::Added(GeoPoint::from((-0.1278f64, 51.5074f64)))),
                passive_properties:  Some(
                    UpdatedSetMembers {
                        removed:    HashSet::from([
                                        KeyValuePair {
                                            key:   String::from("DESCRIPTION"),
                                            value: String::from("Testing original description text."),
                                        }
                        ]),
                        maintained: HashSet::new(),
                        added:      HashSet::from([
                            KeyValuePair {
                                key:   String::from("DESCRIPTION"),
                                value: String::from("Testing description text."),
                            }
                        ])
                    }
                ),
                schedule_properties: Some(
                    SchedulePropertiesDiff {
                        rrule: Some(
                            UpdatedAttribute::Updated(
                               KeyValuePair::new(
                                   String::from("RRULE"),
                                   String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
                               ),
                               KeyValuePair::new(
                                   String::from("RRULE"),
                                   String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                               )
                            )
                        ),
                        exrule:   None,
                        rdate:    None,
                        exdate:   None,
                        duration: None,
                        dtstart:  Some(
                            UpdatedAttribute::Updated(
                                KeyValuePair::new(
                                    String::from("DTSTART"),
                                    String::from(":20201131T183000Z"),
                                ),
                                KeyValuePair::new(
                                    String::from("DTSTART"),
                                    String::from(":20201231T183000Z"),
                                )
                            )
                        ),
                        dtend: None
                    }
                )
            }
        );

        // Test changes between populated original Event and blank updated Event (pure removals).
        let updated_event  = Event::new(String::from("event_UUID"));

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories:  Some(
                                         UpdatedSetMembers {
                                             removed:    HashSet::from([String::from("CATEGORY_THREE"), String::from("CATEGORY_FOUR")]),
                                             maintained: HashSet::new(),
                                             added:      HashSet::new(),
                                         }
                                     ),
                indexed_related_to:  Some(
                                        UpdatedSetMembers {
                                            removed:    HashSet::from([
                                                            KeyValuePair::new(String::from("X-IDX-CAL"), String::from("indexed_calendar_UUID")),
                                                            KeyValuePair::new(String::from("PARENT"),    String::from("another_event_UUID")),
                                            ]),
                                            maintained: HashSet::new(),
                                            added:      HashSet::new()
                                        }
                                     ),
                indexed_geo:         None,
                passive_properties:  Some(
                                        UpdatedSetMembers {
                                            removed:    HashSet::from([
                                                KeyValuePair {
                                                    key:   String::from("DESCRIPTION"),
                                                    value: String::from("Testing original description text.")
                                                }
                                            ]),
                                            maintained: HashSet::new(),
                                            added:      HashSet::new()
                                        }
                                     ),
                schedule_properties: Some(
                                        SchedulePropertiesDiff {
                                            rrule:    Some(
                                                          UpdatedAttribute::Removed(
                                                              KeyValuePair::new(
                                                                  String::from("RRULE"),
                                                                  String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
                                                              )
                                                          ),
                                                      ),
                                            exrule:   None,
                                            rdate:    None,
                                            exdate:   None,
                                            duration: None,
                                            dtstart:  Some(
                                                          UpdatedAttribute::Removed(
                                                              KeyValuePair::new(
                                                                  String::from("DTSTART"),
                                                                  String::from(":20201131T183000Z"),
                                                              )
                                                          )
                                                      ),
                                            dtend:    None
                                        }
                                     )
            }
        );
    }
}
