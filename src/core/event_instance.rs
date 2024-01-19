use chrono::TimeZone;
use chrono_tz::Tz;

use std::cmp::Ordering;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::core::{Event, EventOccurrenceOverride, GeoPoint, IndexedConclusion, KeyValuePair};

use crate::core::ical::serializer::{SerializableICalProperty, SerializableICalComponent};

use crate::core::event_occurrence_iterator::{
    EventOccurrenceIterator, LowerBoundFilterCondition, UpperBoundFilterCondition,
};

use crate::core::serializers::ical_serializer;

use crate::core::event::{IndexedProperties, PassiveProperties};

use crate::core::ical::properties::{
    RecurrenceIDProperty, UIDProperty, DTEndProperty, DTStartProperty, DurationProperty, GeoProperty, CategoriesProperty, RelatedToProperty, ClassProperty, Property
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EventInstance {
    pub uid: UIDProperty,
    pub dtstart: DTStartProperty,
    pub dtend: DTEndProperty,
    pub duration: DurationProperty,

    pub indexed_properties: IndexedProperties,
    pub passive_properties: PassiveProperties,
}

impl EventInstance {
    pub fn new(
        dtstart_timestamp: &i64,
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Self {
        EventInstance {
            uid: event.uid.clone(),
            dtstart: dtstart_timestamp.clone().into(),
            dtend: Self::get_dtend_timestamp(dtstart_timestamp, event, event_occurrence_override).into(),
            duration: Self::get_duration_in_seconds(event, event_occurrence_override).into(),
            indexed_properties: IndexedProperties {
                geo: Self::get_geo(event, event_occurrence_override),
                categories: Self::get_categories(event, event_occurrence_override),
                related_to: Self::get_related_to(event, event_occurrence_override),
                class: Self::get_class(event, event_occurrence_override),
            },
            passive_properties: PassiveProperties {
                properties: Self::get_passive_properties(event, event_occurrence_override),
            },
        }
    }

    fn get_dtend_timestamp(
        dtstart_timestamp: &i64,
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> i64 {
        dtstart_timestamp + Self::get_duration_in_seconds(event, event_occurrence_override)
    }

    fn get_duration_in_seconds(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> i64 {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Ok(Some(overridden_duration)) = event_occurrence_override.get_duration_in_seconds() {
                return overridden_duration;
            }
        }

        if let Ok(Some(event_duration)) = event.schedule_properties.get_duration_in_seconds() {
            return event_duration;
        }

        0
    }

    fn get_geo(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<GeoProperty> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.indexed_properties.geo.is_some() {
                return event_occurrence_override.indexed_properties.geo.to_owned();
            }
        }

        event.indexed_properties.geo.to_owned()
    }

    fn get_categories(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashSet<CategoriesProperty>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.indexed_properties.categories.is_some() {
                return event_occurrence_override.indexed_properties.categories.to_owned();
            }
        }

        event.indexed_properties.categories.to_owned()
    }

    fn get_related_to(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashSet<RelatedToProperty>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.indexed_properties.related_to.is_some() {
                return event_occurrence_override.indexed_properties.related_to.to_owned();
            }
        }

        event.indexed_properties.related_to.to_owned()
    }

    // This gets all the product of all the passive properties overridden by property name.
    // As these are stored in an ordered set of KeyValuePairs we get the overridden passive
    // properties and then iterate over the base event passive properties, checking for the
    // presence of the base event passive property name key, and inserting it if it is not found.
    fn get_passive_properties(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> BTreeSet<Property> {
        let mut passive_properties = BTreeSet::new();

        if let Some(event_occurrence_override) = event_occurrence_override {
            for passive_property in &event_occurrence_override.passive_properties.properties {
                passive_properties.insert(passive_property.to_owned());
            }
        }

        // This searches for the presence of the base event passsive property name key in all the overrides:
        // If found:
        //  Skip
        //
        // If not found:
        //  Add the base event property
        for base_passive_property in &event.passive_properties.properties {
            if passive_properties.iter().find(|passive_property| passive_property.property_name_eq(base_passive_property)).is_none() {
                passive_properties.insert(base_passive_property.to_owned());
            }
        }

        passive_properties
    }

    fn get_class(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<ClassProperty> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.indexed_properties.class.is_some() {
                return event_occurrence_override.indexed_properties.class.to_owned();
            }
        }

        event.indexed_properties.class.to_owned()
    }

    // Copy the contents of the DTStartProperty into RecurrenceIDProperty as it serves
    // essentially the same purpose.
    //
    // TODO: Verify that reckless assertion above:
    //       - https://icalendar.org/iCalendar-RFC-5545/3-8-4-4-recurrence-id.html
    fn build_recurrence_id_from_dtstart(&self) -> RecurrenceIDProperty {
        RecurrenceIDProperty::from(&self.dtstart)
    }

}

impl SerializableICalComponent for EventInstance {
    // TODO: Cater to timezone
    fn serialize_to_ical_set(&self, timezone: &Tz) -> BTreeSet<String> {
        let mut serializable_properties: BTreeSet<String> = BTreeSet::new();

        serializable_properties.insert(self.uid.serialize_to_ical());
        serializable_properties.insert(self.dtstart.serialize_to_ical());
        serializable_properties.insert(self.dtend.serialize_to_ical());
        serializable_properties.insert(self.duration.serialize_to_ical());

        if let Some(geo_property) = &self.indexed_properties.geo {
            serializable_properties.insert(geo_property.serialize_to_ical());
        }

        if let Some(class_property) = &self.indexed_properties.class {
            serializable_properties.insert(class_property.serialize_to_ical());
        }

        if let Some(related_to_properties) = &self.indexed_properties.related_to {
            for related_to_property in related_to_properties {
                serializable_properties.insert(related_to_property.serialize_to_ical());
            }
        }

        if let Some(categories_properties) = &self.indexed_properties.categories {
            for categories_property in categories_properties {
                serializable_properties.insert(categories_property.serialize_to_ical());
            }
        }

        for passive_property in &self.passive_properties.properties {
            serializable_properties.insert(passive_property.serialize_to_ical());
        }

        serializable_properties.insert(self.build_recurrence_id_from_dtstart().serialize_to_ical());

        serializable_properties
    }
}

impl PartialOrd for EventInstance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let dtstart_timestamp_comparison =
            self.dtstart.utc_timestamp.partial_cmp(&other.dtstart.utc_timestamp);

        if dtstart_timestamp_comparison.is_some_and(|comparison| comparison.is_eq()) {
            self.dtend.utc_timestamp.partial_cmp(&other.dtend.utc_timestamp)
        } else {
            dtstart_timestamp_comparison
        }
    }
}

impl Ord for EventInstance {
    fn cmp(&self, other: &Self) -> Ordering {
        let dtstart_timestamp_comparison = self.dtstart.utc_timestamp.cmp(&other.dtstart.utc_timestamp);

        if dtstart_timestamp_comparison.is_eq() {
            self.dtend.utc_timestamp.cmp(&other.dtend.utc_timestamp)
        } else {
            dtstart_timestamp_comparison
        }
    }
}

#[derive(Debug)]
pub struct EventInstanceIterator<'a> {
    event: &'a Event,
    internal_iter: EventOccurrenceIterator<'a>,
}

impl<'a> EventInstanceIterator<'a> {
    pub fn new(
        event: &'a Event,
        limit: Option<usize>,
        filter_from: Option<LowerBoundFilterCondition>,
        filter_until: Option<UpperBoundFilterCondition>,
        filtering_indexed_conclusion: Option<IndexedConclusion>,
    ) -> Result<EventInstanceIterator<'a>, String> {
        let internal_iter = EventOccurrenceIterator::new(
            &event.schedule_properties,
            &event.overrides,
            limit,
            filter_from,
            filter_until,
            filtering_indexed_conclusion.clone(),
        )?;

        Ok(EventInstanceIterator {
            event,
            internal_iter,
        })
    }
}

impl<'a> Iterator for EventInstanceIterator<'a> {
    type Item = EventInstance;

    fn next(&mut self) -> Option<Self::Item> {
        // Filter occurrence index iterator timestamps according to IndexedConclusion if
        // present, else include all.
        self.internal_iter.next().and_then(
            |(dtstart_timestamp, _dtend_timestamp, event_occurrence_override)| {
                Some(EventInstance::new(
                    &dtstart_timestamp,
                    self.event,
                    event_occurrence_override.as_ref(),
                ))
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::testing::utils::{build_event_and_overrides_from_ical, build_event_from_ical};
    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

    use crate::testing::macros::build_property_from_ical;

    use crate::core::ical::properties::{DescriptionProperty, LocationProperty};

    #[test]
    fn test_event_instance_without_override() {
        let event = build_event_from_ical(
            "event_UID",
            vec![
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T183100Z",
                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE",
                "RELATED-TO;RELTYPE=CHILD:ChildUID",
                "RELATED-TO;RELTYPE=PARENT:ParentUID_One",
                "RELATED-TO;RELTYPE=PARENT:ParentUID_Two",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two",
                "CLASS:PRIVATE",
                "GEO:48.85299;2.36885",
                "DESCRIPTION:Event description text.",
                "LOCATION:Event address text.",
            ],
        );

        let event_instance = EventInstance::new(&100, &event, None);

        assert_eq_sorted!(
            event_instance,
            EventInstance {
                uid: build_property_from_ical!(UIDProperty, "UID:event_UID"),
                dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000140Z"),
                dtend: build_property_from_ical!(DTEndProperty, "DTEND:19700101T000240Z"),
                duration: build_property_from_ical!(DurationProperty, "DURATION:PT1M"),
                indexed_properties: IndexedProperties {
                    class: Some(build_property_from_ical!(ClassProperty, "CLASS:PRIVATE")),
                    geo: Some(build_property_from_ical!(GeoProperty, "GEO:48.85299;2.36885")),
                    categories: Some(HashSet::from([
                            build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE"),
                    ])),
                    related_to: Some(HashSet::from([
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=PARENT:ParentUID_One"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=PARENT:ParentUID_Two"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=CHILD:ChildUID"),
                    ])),
                },
                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([
                                    Property::Description(build_property_from_ical!(DescriptionProperty, "DESCRIPTION:Event description text.")),
                                    Property::Location(build_property_from_ical!(LocationProperty, "LOCATION:Event address text.")),
                    ])
                },
            }
        );

        assert_eq!(
            event_instance.serialize_to_ical(&Tz::Europe__London),
            vec![
                String::from("CATEGORIES:CATEGORY THREE,CATEGORY_ONE,CATEGORY_TWO"),
                String::from("DESCRIPTION:Event description text."),
                String::from("DTEND;TZID=Europe/London:19700101T010240"),
                String::from("DTSTART;TZID=Europe/London:19700101T010140"),
                String::from("GEO:48.85299;2.36885"),
                String::from("LOCATION:Event address text."),
                String::from("RECURRENCE-ID;VALUE=DATE-TIME:19700101T000140Z"),
                String::from("RELATED-TO;RELTYPE=CHILD:ChildUID"),
                String::from("RELATED-TO;RELTYPE=PARENT:ParentUID_One"),
                String::from("RELATED-TO;RELTYPE=PARENT:ParentUID_Two"),
                String::from("RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                String::from("RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three"),
                String::from("RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"),
                String::from("UID:event_UID"),
            ]
        );
    }

    #[test]
    fn test_event_instance_with_override() {
        let event = build_event_and_overrides_from_ical(
            "event_UID",
            vec![
                "DTSTART:20201231T183000Z",
                "DTEND:20201231T183100Z", // Ends 60 seconds after it starts.
                "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE",
                "RELATED-TO;RELTYPE=CHILD:ChildUID",
                "RELATED-TO;RELTYPE=PARENT:ParentUID_One",
                "RELATED-TO;RELTYPE=PARENT:ParentUID_Two",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three",
                "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two",
                "DESCRIPTION:Event description text.",
                "LOCATION:Event address text.",
            ],
            vec![
                vec![
                    "LOCATION:Overridden Event address text.",
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                    "DTSTART:20201231T183000Z",
                    "RELATED-TO;RELTYPE=CHILD:ChildUID",
                    "RELATED-TO;RELTYPE=PARENT:ParentUID_Three",
                    "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                    "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four",
                ],
            ],
        );

        let Some(event_occurrence_override) = event.overrides.get(&1609439400) else {
            panic!("Expected event to have an occurrence...");
        };

        let event_instance =
            EventInstance::new(&1609439400, &event, Some(&event_occurrence_override));

        assert_eq!(
            event_instance,
            EventInstance {
                uid: build_property_from_ical!(UIDProperty, "UID:event_UID"),
                dtstart: build_property_from_ical!(DTStartProperty, "DTSTART:20201231T183000Z"),
                dtend: build_property_from_ical!(DTEndProperty, "DTEND:20201231T183100Z"),
                duration: build_property_from_ical!(DurationProperty, "DURATION:PT1M"),
                indexed_properties: IndexedProperties {
                    class: None,
                    geo: None,
                    categories: Some(HashSet::from([
                            build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR"),
                    ])),
                    related_to: Some(HashSet::from([
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=PARENT:ParentUID_Three"),
                            build_property_from_ical!(RelatedToProperty, "RELATED-TO;RELTYPE=CHILD:ChildUID"),
                    ])),
                },
                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([
                                    Property::Description(build_property_from_ical!(DescriptionProperty, "DESCRIPTION:Event description text.")),
                                    Property::Location(build_property_from_ical!(LocationProperty, "LOCATION:Overridden Event address text.")),
                    ])
                },
            }
        );

        assert_eq!(
            event_instance.serialize_to_ical(&Tz::UTC),
            vec![
                String::from("CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE"),
                String::from("DESCRIPTION:Event description text."),
                String::from("DTEND:20201231T183100Z"),
                String::from("DTSTART:20201231T183000Z"),
                String::from("DURATION:PT1M"),
                String::from("LOCATION:Overridden Event address text."),
                String::from("RECURRENCE-ID;VALUE=DATE-TIME:20201231T183000Z"),
                String::from("RELATED-TO;RELTYPE=CHILD:ChildUID"),
                String::from("RELATED-TO;RELTYPE=PARENT:ParentUID_Three"),
                String::from("RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four"),
                String::from("RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"),
                String::from("UID:event_UID"),
            ]
        );
    }

    #[test]
    fn test_event_instance_iterator() {
        let event = build_event_and_overrides_from_ical(
            "event_UID",
            vec![
                "DESCRIPTION:BASE description text.",
                "DTSTART:20210105T183000Z",
                "DTEND:20210105T190000Z",
                "RRULE:FREQ=WEEKLY;UNTIL=20210202T183000Z;INTERVAL=1",
                "CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO",
                "RELATED-TO;RELTYPE=PARENT:BASE_ParentdUID",
                "RELATED-TO;RELTYPE=CHILD:BASE_ChildUID",
            ],
            vec![
                vec![
                    "DESCRIPTION:OVERRIDDEN description text.",
                    "DTSTART:20210105T183000Z",
                    "CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID",
                ],
                vec![
                    "RELATED-TO;RELTYPE=CHILD:BASE_ChildUID",
                    "DTSTART:20210112T183000Z",
                    "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID",
                ],
                vec![
                    "DESCRIPTION:OVERRIDDEN description text.",
                    "DTSTART:20210126T183000Z",
                    "CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                    "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID",
                    "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID",
                ],
            ],
        );

        let expected_event_instances_ical = HashMap::from([
            (
                1609871400,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE"),
                    String::from("DESCRIPTION:OVERRIDDEN description text."),
                    String::from("DTEND:20210105T190000Z"),
                    String::from("DTSTART:20210105T183000Z"),
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z"),
                    String::from("RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID"),
                    String::from("UID:event_UID"),
                ],
            ),
            (
                1610476200,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:20210112T190000Z"),
                    String::from("DTSTART:20210112T183000Z"),
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210112T183000Z"),
                    String::from("RELATED-TO;RELTYPE=CHILD:BASE_ChildUID"),
                    String::from("RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID"),
                    String::from("UID:event_UID"),
                ],
            ),
            (
                1611081000,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:20210119T190000Z"),
                    String::from("DTSTART:20210119T183000Z"),
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210119T183000Z"),
                    String::from("RELATED-TO;RELTYPE=CHILD:BASE_ChildUID"),
                    String::from("RELATED-TO;RELTYPE=PARENT:BASE_ParentdUID"),
                    String::from("UID:event_UID"),
                ],
            ),
            (
                1611685800,
                vec![
                    String::from("CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO"),
                    String::from("DESCRIPTION:OVERRIDDEN description text."),
                    String::from("DTEND:20210126T190000Z"),
                    String::from("DTSTART:20210126T183000Z"),
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210126T183000Z"),
                    String::from("RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID"),
                    String::from("RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID"),
                    String::from("UID:event_UID"),
                ],
            ),
            (
                1612290600,
                vec![
                    String::from("CATEGORIES:BASE_CATEGORY_ONE,BASE_CATEGORY_TWO"),
                    String::from("DESCRIPTION:BASE description text."),
                    String::from("DTEND:20210202T190000Z"),
                    String::from("DTSTART:20210202T183000Z"),
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210202T183000Z"),
                    String::from("RELATED-TO;RELTYPE=CHILD:BASE_ChildUID"),
                    String::from("RELATED-TO;RELTYPE=PARENT:BASE_ParentdUID"),
                    String::from("UID:event_UID"),
                ],
            ),
        ]);

        // Testing without any filtered index conclusion
        let mut event_instance_iterator =
            EventInstanceIterator::new(&event, None, None, None, None).unwrap();

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1611685800].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1612290600].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);

        // Testing with filtered IndexedConclusion::Include without exceptions
        let mut event_instance_iterator = EventInstanceIterator::new(
            &event,
            None,
            None,
            None,
            Some(IndexedConclusion::Include(None)),
        )
        .unwrap();

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1611685800].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1612290600].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);

        // Testing with filtered IndexedConclusion::Include with exceptions
        let mut event_instance_iterator = EventInstanceIterator::new(
            &event,
            None,
            None,
            None,
            Some(IndexedConclusion::Include(Some(HashSet::from([
                1609871400, 1611081000, 1612290600,
            ])))),
        )
        .unwrap();

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
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
            )
            .unwrap()
            .next(),
            None
        );

        // Testing with filtered IndexedConclusion::Exclude with exceptions
        let mut event_instance_iterator = EventInstanceIterator::new(
            &event,
            None,
            None,
            None,
            Some(IndexedConclusion::Exclude(Some(HashSet::from([
                1609871400, 1611081000, 1612290600,
            ])))),
        )
        .unwrap();

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator
                .next()
                .and_then(|event_instance| Some(event_instance.serialize_to_ical(&Tz::UTC))),
            Some(expected_event_instances_ical[&1612290600].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);
    }
}
