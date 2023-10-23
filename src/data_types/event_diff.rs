use crate::data_types::{UpdatedSetMembers, Event, hashmap_to_hashset, btree_hashset_to_hashset, KeyValuePair, UpdatedAttribute, GeoPoint};

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
    rrule:    Option<UpdatedSetMembers<KeyValuePair>>,
    exrule:   Option<UpdatedSetMembers<KeyValuePair>>,
    rdate:    Option<UpdatedSetMembers<KeyValuePair>>,
    exdate:   Option<UpdatedSetMembers<KeyValuePair>>,
    duration: Option<UpdatedSetMembers<KeyValuePair>>,
    dtstart:  Option<UpdatedSetMembers<KeyValuePair>>,
    dtend:    Option<UpdatedSetMembers<KeyValuePair>>,
}

impl SchedulePropertiesDiff {
    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        let original_event_schedule_properties = &original_event.schedule_properties;
        let updated_event_schedule_properties  = &updated_event.schedule_properties;

        SchedulePropertiesDiff {
            rrule:    Self::build_updated_set_members(original_event_schedule_properties.rrule.as_ref(),    updated_event_schedule_properties.rrule.as_ref()),
            exrule:   Self::build_updated_set_members(original_event_schedule_properties.exrule.as_ref(),   updated_event_schedule_properties.exrule.as_ref()),
            rdate:    Self::build_updated_set_members(original_event_schedule_properties.rdate.as_ref(),    updated_event_schedule_properties.rdate.as_ref()),
            exdate:   Self::build_updated_set_members(original_event_schedule_properties.exdate.as_ref(),   updated_event_schedule_properties.exdate.as_ref()),
            duration: Self::build_updated_set_members(original_event_schedule_properties.duration.as_ref(), updated_event_schedule_properties.duration.as_ref()),
            dtstart:  Self::build_updated_set_members(original_event_schedule_properties.dtstart.as_ref(),  updated_event_schedule_properties.dtstart.as_ref()),
            dtend:    Self::build_updated_set_members(original_event_schedule_properties.dtend.as_ref(),    updated_event_schedule_properties.dtend.as_ref()),
        }
    }

    pub fn get_schedule_rebuild_consensus(&self) -> ScheduleRebuildConsensus {
        fn property_has_changed(property: Option<&UpdatedSetMembers<KeyValuePair>>) -> bool {
            property.is_some_and(|property| property.is_changed())
        }

        #[allow(unused_parens)]
        if (
            property_has_changed(self.rrule.as_ref())  ||
            property_has_changed(self.exrule.as_ref()) ||
            property_has_changed(self.rdate.as_ref())  ||
            property_has_changed(self.exdate.as_ref()) ||
            property_has_changed(self.dtstart.as_ref())
        ) {
            // TODO: handle more granular changes yielding ScheduleRebuildConsensus::Partial for partial
            // updated occurrence extrapolation.
            ScheduleRebuildConsensus::Full
        } else {
            ScheduleRebuildConsensus::None
        }
    }

    fn build_updated_set_members<T>(original_set: Option<&HashSet<T>>, updated_set: Option<&HashSet<T>>) -> Option<UpdatedSetMembers<T>>
    where
        T: Eq + PartialEq + Hash + Clone
    {
        match (original_set, updated_set) {
            (None, None) => None,

            _ => Some(UpdatedSetMembers::new(original_set, updated_set))
        }
    }
}

pub enum ScheduleRebuildConsensus {
    None,
    Full,
    Partial,
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{HashMap, BTreeSet, BTreeMap};

    use crate::data_types::{ScheduleProperties, IndexedProperties, PassiveProperties, EventOccurrenceOverrides, InvertedEventIndex, KeyValuePair};

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

            overrides: EventOccurrenceOverrides {
                detached: BTreeMap::new(),
                current:  BTreeMap::new(),
            },
            occurrence_cache:   None,
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
                                                          UpdatedSetMembers {
                                                              removed:    HashSet::new(),
                                                              maintained: HashSet::new(),
                                                              added:      HashSet::from([
                                                                  KeyValuePair::new(
                                                                      String::from("RRULE"),
                                                                      String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                                                                  )
                                                              ])
                                                          }
                                                      ),
                                            exrule:   None,
                                            rdate:    None,
                                            exdate:   None,
                                            duration: None,
                                            dtstart:  Some(
                                                        UpdatedSetMembers {
                                                            removed:    HashSet::new(),
                                                            maintained: HashSet::new(),
                                                            added:      HashSet::from([
                                                                KeyValuePair::new(
                                                                    String::from("DTSTART"),
                                                                    String::from(":20201231T183000Z"),
                                                                )
                                                            ])
                                                        }
                                                    ),
                                            dtend: None
                                        }
                                     )
            }
        );

        // Test changes between populated original and updated Events (with removals).
        let original_event = Event {
            uuid: String::from("event_UUID"),

            schedule_properties: ScheduleProperties {
                rrule:            Some(
                    HashSet::from([
                        KeyValuePair::new(
                            String::from("RRULE"),
                            String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
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
                            String::from(":20201131T183000Z"),
                        )
                    ])
                ),
                dtend:            None,
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

            overrides: EventOccurrenceOverrides {
                detached: BTreeMap::new(),
                current:  BTreeMap::new(),
            },
            occurrence_cache:   None,
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
                                        KeyValuePair::new(String::from("PARENT"), String::from("another_event_UUID")),
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
                        rrule:    Some(
                                      UpdatedSetMembers {
                                          removed:    HashSet::from([
                                                          KeyValuePair::new(
                                                              String::from("RRULE"),
                                                              String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
                                                          )
                                          ]),
                                          maintained: HashSet::new(),
                                          added:      HashSet::from([
                                              KeyValuePair::new(
                                                  String::from("RRULE"),
                                                  String::from(":FREQ=DAILY;UNTIL=20230331T183000Z;INTERVAL=1"),
                                              )
                                          ])
                                      }
                                  ),
                                  exrule:   None,
                                  rdate:    None,
                                  exdate:   None,
                                  duration: None,
                                  dtstart:  Some(
                                      UpdatedSetMembers {
                                          removed:    HashSet::from([
                                                          KeyValuePair::new(
                                                              String::from("DTSTART"),
                                                              String::from(":20201131T183000Z"),
                                                          )
                                          ]),
                                          maintained: HashSet::new(),
                                          added:      HashSet::from([
                                              KeyValuePair::new(
                                                  String::from("DTSTART"),
                                                  String::from(":20201231T183000Z"),
                                              )
                                          ])
                                      }
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
                                                          UpdatedSetMembers {
                                                              removed:    HashSet::from([
                                                                  KeyValuePair::new(
                                                                      String::from("RRULE"),
                                                                      String::from(":FREQ=DAILY;UNTIL=20230231T183000Z;INTERVAL=1"),
                                                                  )
                                                              ]),
                                                              maintained: HashSet::new(),
                                                              added:      HashSet::new(),
                                                          }
                                                      ),
                                            exrule:   None,
                                            rdate:    None,
                                            exdate:   None,
                                            duration: None,
                                            dtstart:  Some(
                                                        UpdatedSetMembers {
                                                            removed:    HashSet::from([
                                                                KeyValuePair::new(
                                                                    String::from("DTSTART"),
                                                                    String::from(":20201131T183000Z"),
                                                                )
                                                            ]),
                                                            maintained: HashSet::new(),
                                                            added:      HashSet::new(),
                                                        }
                                                    ),
                                            dtend: None
                                        }
                                     )
            }
        );
    }
}
