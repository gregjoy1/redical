use std::collections::HashSet;
use std::hash::Hash;

use crate::core::ical::properties::DurationProperty;
use crate::core::{
    btree_hashset_to_hashset, hashmap_to_hashset, Event, GeoPoint, KeyValuePair, UpdatedAttribute,
    UpdatedSetMembers,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EventDiff {
    pub indexed_categories: Option<UpdatedSetMembers<String>>,
    pub indexed_related_to: Option<UpdatedSetMembers<KeyValuePair>>,
    pub indexed_geo: Option<UpdatedAttribute<GeoPoint>>,
    pub indexed_class: Option<UpdatedAttribute<String>>,

    pub passive_properties: Option<UpdatedSetMembers<KeyValuePair>>,
    pub schedule_properties: Option<SchedulePropertiesDiff>,
}

impl EventDiff {
    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        EventDiff {
            indexed_categories: Self::diff_indexed_categories(original_event, updated_event),
            indexed_related_to: Self::diff_indexed_related_to(original_event, updated_event),
            indexed_geo: Self::diff_indexed_geo(original_event, updated_event),
            indexed_class: Self::diff_indexed_class(original_event, updated_event),

            passive_properties: Self::diff_passive_properties(original_event, updated_event),
            schedule_properties: Self::diff_schedule_properties(original_event, updated_event),
        }
    }

    fn diff_indexed_categories(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<UpdatedSetMembers<String>> {
        Some(UpdatedSetMembers::new(
            original_event
                .indexed_properties
                .extract_all_category_strings()
                .as_ref(),
            updated_event
                .indexed_properties
                .extract_all_category_strings()
                .as_ref(),
        ))
    }

    fn diff_indexed_related_to(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<UpdatedSetMembers<KeyValuePair>> {
        let original_related_to = original_event
            .indexed_properties
            .extract_all_related_to_key_value_pairs();
        let updated_related_to = updated_event
            .indexed_properties
            .extract_all_related_to_key_value_pairs();

        Some(UpdatedSetMembers::new(
            original_related_to.as_ref(),
            updated_related_to.as_ref(),
        ))
    }

    fn diff_indexed_geo(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<UpdatedAttribute<GeoPoint>> {
        let original_geo = &original_event.indexed_properties.extract_geo_point();
        let updated_geo = &updated_event.indexed_properties.extract_geo_point();

        if original_geo.is_none() && updated_geo.is_none() {
            None
        } else {
            Some(UpdatedAttribute::new(original_geo, updated_geo))
        }
    }

    fn diff_indexed_class(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<UpdatedAttribute<String>> {
        let original_class = &original_event.indexed_properties.extract_class();
        let updated_class = &updated_event.indexed_properties.extract_class();

        if original_class.is_none() && updated_class.is_none() {
            None
        } else {
            Some(UpdatedAttribute::new(original_class, updated_class))
        }
    }

    fn diff_passive_properties(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<UpdatedSetMembers<KeyValuePair>> {
        // TODO: Improve this to be 0 copy
        let original_passive_properties = Some(
            original_event
                .passive_properties
                .extract_properties_serialized_ical_key_value_pairs(),
        );

        let updated_passive_properties = Some(
            updated_event
                .passive_properties
                .extract_properties_serialized_ical_key_value_pairs(),
        );

        Some(UpdatedSetMembers::new(
            original_passive_properties.as_ref(),
            updated_passive_properties.as_ref(),
        ))
    }

    fn diff_schedule_properties(
        original_event: &Event,
        updated_event: &Event,
    ) -> Option<SchedulePropertiesDiff> {
        Some(SchedulePropertiesDiff::new(original_event, updated_event))
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct SchedulePropertiesDiff {
    rrule: Option<UpdatedAttribute<KeyValuePair>>,
    exrule: Option<UpdatedAttribute<KeyValuePair>>,
    rdate: Option<UpdatedSetMembers<KeyValuePair>>,
    exdate: Option<UpdatedSetMembers<KeyValuePair>>,
    duration: Option<UpdatedAttribute<DurationProperty>>,
    dtstart: Option<UpdatedAttribute<KeyValuePair>>,
    dtend: Option<UpdatedAttribute<KeyValuePair>>,
}

impl SchedulePropertiesDiff {
    pub fn new(original_event: &Event, updated_event: &Event) -> Self {
        let original_event_schedule_properties = &original_event.schedule_properties;
        let updated_event_schedule_properties = &updated_event.schedule_properties;

        SchedulePropertiesDiff {
            rrule: Self::build_updated_attribute(
                &original_event_schedule_properties.extract_serialized_rrule_ical_key_value_pair(),
                &updated_event_schedule_properties.extract_serialized_rrule_ical_key_value_pair(),
            ),
            exrule: Self::build_updated_attribute(
                &original_event_schedule_properties.extract_serialized_exrule_ical_key_value_pair(),
                &updated_event_schedule_properties.extract_serialized_exrule_ical_key_value_pair(),
            ),
            rdate: Self::build_updated_set_members(
                original_event_schedule_properties
                    .extract_serialized_rdates_ical_key_value_pairs()
                    .as_ref(),
                updated_event_schedule_properties
                    .extract_serialized_rdates_ical_key_value_pairs()
                    .as_ref(),
            ),
            exdate: Self::build_updated_set_members(
                original_event_schedule_properties
                    .extract_serialized_exdates_ical_key_value_pairs()
                    .as_ref(),
                updated_event_schedule_properties
                    .extract_serialized_exdates_ical_key_value_pairs()
                    .as_ref(),
            ),
            duration: Self::build_updated_attribute(
                &original_event_schedule_properties.duration,
                &updated_event_schedule_properties.duration,
            ),
            dtstart: Self::build_updated_attribute(
                &original_event_schedule_properties.extract_serialized_dtstart_ical_key_value_pair(),
                &updated_event_schedule_properties.extract_serialized_dtstart_ical_key_value_pair(),
            ),
            dtend: Self::build_updated_attribute(
                &original_event_schedule_properties.extract_serialized_dtend_ical_key_value_pair(),
                &updated_event_schedule_properties.extract_serialized_dtend_ical_key_value_pair(),
            ),
        }
    }

    fn build_updated_set_members<T>(
        original_set: Option<&HashSet<T>>,
        updated_set: Option<&HashSet<T>>,
    ) -> Option<UpdatedSetMembers<T>>
    where
        T: Eq + PartialEq + Hash + Clone,
    {
        match (original_set, updated_set) {
            (None, None) => None,

            _ => Some(UpdatedSetMembers::new(original_set, updated_set)),
        }
    }

    fn build_updated_attribute<T>(
        original: &Option<T>,
        updated: &Option<T>,
    ) -> Option<UpdatedAttribute<T>>
    where
        T: Eq + PartialEq + Clone,
    {
        match (original, updated) {
            (None, None) => None,

            _ => Some(UpdatedAttribute::new(original, updated)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::testing::macros::build_property_from_ical;

    use std::collections::{BTreeMap, BTreeSet, HashSet};

    use crate::core::ical::properties::{
        CategoriesProperty, ClassProperty, DTStartProperty, DescriptionProperty,
        DurationProperty, ExDateProperty, ExRuleProperty, GeoProperty, Property, RDateProperty,
        RRuleProperty, RelatedToProperty,
    };

    use crate::core::{IndexedProperties, KeyValuePair, PassiveProperties, ScheduleProperties};

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_event_diff() {
        // Test when no changes between both original and updated Events
        let original_event = Event::new(String::from("event_UID"));
        let updated_event = Event::new(String::from("event_UID"));

        let expected_indexed_categories = Some(UpdatedSetMembers {
            removed: HashSet::new(),
            maintained: HashSet::new(),
            added: HashSet::new(),
        });

        let expected_indexed_related_to = Some(UpdatedSetMembers {
            removed: HashSet::new(),
            maintained: HashSet::new(),
            added: HashSet::new(),
        });

        let expected_indexed_geo = None;

        let expected_indexed_class = None;

        let expected_passive_properties = Some(UpdatedSetMembers {
            removed: HashSet::new(),
            maintained: HashSet::new(),
            added: HashSet::new(),
        });

        let expected_schedule_properties = Some(SchedulePropertiesDiff {
            rrule: None,
            exrule: None,
            rdate: None,
            exdate: None,
            duration: None,
            dtstart: None,
            dtend: None,
        });

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories: expected_indexed_categories,
                indexed_related_to: expected_indexed_related_to,
                indexed_geo: expected_indexed_geo,
                indexed_class: expected_indexed_class,
                passive_properties: expected_passive_properties,
                schedule_properties: expected_schedule_properties,
            }
        );

        // Test changes between blank original Event and populated updated Event
        let updated_event = Event {
            uid: String::from("event_UID").into(),

            schedule_properties: ScheduleProperties {
                rrule: Some(build_property_from_ical!(
                    RRuleProperty,
                    "RRULE:FREQ=DAILY;UNTIL=20230330T183000Z;INTERVAL=1"
                )),
                exrule: None,
                rdates: None,
                exdates: None,
                duration: None,
                dtstart: Some(build_property_from_ical!(
                    DTStartProperty,
                    "DTSTART:20201230T183000Z"
                )),
                dtend: None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo: Some(build_property_from_ical!(
                    GeoProperty,
                    "GEO:51.5074;-0.1278"
                )),
                class: Some(build_property_from_ical!(ClassProperty, "CLASS:PRIVATE")),
                related_to: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE"
                )])),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::from([Property::Description(build_property_from_ical!(
                    DescriptionProperty,
                    "DESCRIPTION:Testing updated description text"
                ))]),
            },

            overrides: BTreeMap::new(),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo: None,
            indexed_class: None,
        };

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories: Some(UpdatedSetMembers {
                    removed: HashSet::new(),
                    maintained: HashSet::new(),
                    added: HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY_THREE"),
                    ])
                }),
                indexed_related_to: Some(UpdatedSetMembers {
                    removed: HashSet::new(),
                    maintained: HashSet::new(),
                    added: HashSet::new(),
                }),
                indexed_geo: Some(UpdatedAttribute::Added(GeoPoint::from((
                    -0.1278f64, 51.5074f64
                )))),
                indexed_class: Some(UpdatedAttribute::Added(String::from("PRIVATE"))),
                passive_properties: Some(UpdatedSetMembers {
                    removed: HashSet::new(),
                    maintained: HashSet::new(),
                    added: HashSet::from([KeyValuePair {
                        key: String::from("DESCRIPTION"),
                        value: String::from(":Testing updated description text")
                    }])
                }),
                schedule_properties: Some(SchedulePropertiesDiff {
                    rrule: Some(UpdatedAttribute::Added(KeyValuePair::new(
                        String::from("RRULE"),
                        String::from(":FREQ=DAILY;INTERVAL=1;UNTIL=20230330T183000Z"),
                    ))),
                    exrule: None,
                    rdate: None,
                    exdate: None,
                    duration: None,
                    dtstart: Some(UpdatedAttribute::Added(KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(":20201230T183000Z"),
                    ))),
                    dtend: None
                })
            }
        );

        // Test changes between populated original and updated Events (with removals).
        let original_event = Event {
            uid: String::from("event_UID").into(),

            schedule_properties: ScheduleProperties {
                rrule: Some(build_property_from_ical!(
                    RRuleProperty,
                    "RRULE:FREQ=DAILY;UNTIL=20230130T183000Z;INTERVAL=1"
                )),
                exrule: None,
                rdates: None,
                exdates: None,
                duration: None,
                dtstart: Some(build_property_from_ical!(
                    DTStartProperty,
                    "DTSTART:20201130T183000Z"
                )),
                dtend: None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                related_to: Some(HashSet::from([
                    build_property_from_ical!(
                        RelatedToProperty,
                        "RELATED-TO;RELTYPE=X-IDX-CAL:indexed_calendar_UID"
                    ),
                    build_property_from_ical!(
                        RelatedToProperty,
                        "RELATED-TO;RELTYPE=PARENT:another_event_UID"
                    ),
                ])),
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_THREE,CATEGORY_FOUR"
                )])),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::from([Property::Description(build_property_from_ical!(
                    DescriptionProperty,
                    "DESCRIPTION:Testing original description text"
                ))]),
            },

            overrides: BTreeMap::new(),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo: None,
            indexed_class: None,
        };

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories: Some(UpdatedSetMembers {
                    removed: HashSet::from([String::from("CATEGORY_FOUR")]),
                    maintained: HashSet::from([String::from("CATEGORY_THREE")]),
                    added: HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO")
                    ])
                }),
                indexed_related_to: Some(UpdatedSetMembers {
                    removed: HashSet::from([
                        KeyValuePair::new(
                            String::from("X-IDX-CAL"),
                            String::from("indexed_calendar_UID")
                        ),
                        KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("another_event_UID")
                        ),
                    ]),
                    maintained: HashSet::new(),
                    added: HashSet::new()
                }),
                indexed_geo: Some(UpdatedAttribute::Added(GeoPoint::from((
                    -0.1278f64, 51.5074f64
                )))),
                indexed_class: Some(UpdatedAttribute::Added(String::from("PRIVATE"))),
                passive_properties: Some(UpdatedSetMembers {
                    removed: HashSet::from([KeyValuePair {
                        key: String::from("DESCRIPTION"),
                        value: String::from(":Testing original description text"),
                    }]),
                    maintained: HashSet::new(),
                    added: HashSet::from([KeyValuePair {
                        key: String::from("DESCRIPTION"),
                        value: String::from(":Testing updated description text"),
                    }])
                }),
                schedule_properties: Some(SchedulePropertiesDiff {
                    rrule: Some(UpdatedAttribute::Updated(
                        KeyValuePair::new(
                            String::from("RRULE"),
                            String::from(":FREQ=DAILY;INTERVAL=1;UNTIL=20230130T183000Z"),
                        ),
                        KeyValuePair::new(
                            String::from("RRULE"),
                            String::from(":FREQ=DAILY;INTERVAL=1;UNTIL=20230330T183000Z"),
                        )
                    )),
                    exrule: None,
                    rdate: None,
                    exdate: None,
                    duration: None,
                    dtstart: Some(UpdatedAttribute::Updated(
                        KeyValuePair::new(
                            String::from("DTSTART"),
                            String::from(":20201130T183000Z"),
                        ),
                        KeyValuePair::new(
                            String::from("DTSTART"),
                            String::from(":20201230T183000Z"),
                        )
                    )),
                    dtend: None
                })
            }
        );

        // Test changes between populated original Event and blank updated Event (pure removals).
        let updated_event = Event::new(String::from("event_UID"));

        assert_eq!(
            EventDiff::new(&original_event, &updated_event),
            EventDiff {
                indexed_categories: Some(UpdatedSetMembers {
                    removed: HashSet::from([
                        String::from("CATEGORY_THREE"),
                        String::from("CATEGORY_FOUR")
                    ]),
                    maintained: HashSet::new(),
                    added: HashSet::new(),
                }),
                indexed_related_to: Some(UpdatedSetMembers {
                    removed: HashSet::from([
                        KeyValuePair::new(
                            String::from("X-IDX-CAL"),
                            String::from("indexed_calendar_UID")
                        ),
                        KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("another_event_UID")
                        ),
                    ]),
                    maintained: HashSet::new(),
                    added: HashSet::new()
                }),
                indexed_geo: None,
                indexed_class: None,
                passive_properties: Some(UpdatedSetMembers {
                    removed: HashSet::from([KeyValuePair {
                        key: String::from("DESCRIPTION"),
                        value: String::from(":Testing original description text")
                    }]),
                    maintained: HashSet::new(),
                    added: HashSet::new()
                }),
                schedule_properties: Some(SchedulePropertiesDiff {
                    rrule: Some(UpdatedAttribute::Removed(KeyValuePair::new(
                        String::from("RRULE"),
                        String::from(":FREQ=DAILY;INTERVAL=1;UNTIL=20230130T183000Z"),
                    ))),
                    exrule: None,
                    rdate: None,
                    exdate: None,
                    duration: None,
                    dtstart: Some(UpdatedAttribute::Removed(KeyValuePair::new(
                        String::from("DTSTART"),
                        String::from(":20201130T183000Z"),
                    ))),
                    dtend: None
                })
            }
        );
    }
}
