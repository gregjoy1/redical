use rrule::Tz;
use std::cmp::Ordering;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::core::{Event, EventOccurrenceOverride, GeoPoint, IndexedConclusion, KeyValuePair};

use crate::core::event_occurrence_iterator::{
    EventOccurrenceIterator, LowerBoundFilterCondition, UpperBoundFilterCondition,
};

use crate::core::serializers::ical_serializer;
use crate::core::serializers::ical_serializer::ICalSerializer;

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EventInstance {
    pub uid: String,
    pub dtstart_timestamp: i64,
    pub dtend_timestamp: i64,
    pub duration: i64,
    pub geo: Option<GeoPoint>,
    pub categories: Option<HashSet<String>>,
    pub related_to: Option<HashMap<String, HashSet<String>>>,
    pub passive_properties: BTreeSet<KeyValuePair>,
}

impl EventInstance {
    pub fn new(
        dtstart_timestamp: &i64,
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Self {
        EventInstance {
            uid: event.uid.to_owned(),
            dtstart_timestamp: dtstart_timestamp.to_owned(),
            dtend_timestamp: Self::get_dtend_timestamp(
                dtstart_timestamp,
                event,
                event_occurrence_override,
            ),
            duration: Self::get_duration(dtstart_timestamp, event, event_occurrence_override),
            geo: Self::get_geo(event, event_occurrence_override),
            categories: Self::get_categories(event, event_occurrence_override),
            related_to: Self::get_related_to(event, event_occurrence_override),
            passive_properties: Self::get_passive_properties(event, event_occurrence_override),
        }
    }

    fn get_dtend_timestamp(
        dtstart_timestamp: &i64,
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> i64 {
        dtstart_timestamp + Self::get_duration(dtstart_timestamp, event, event_occurrence_override)
    }

    fn get_duration(
        dtstart_timestamp: &i64,
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> i64 {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Ok(Some(overridden_duration)) =
                event_occurrence_override.get_duration(&dtstart_timestamp)
            {
                return overridden_duration;
            }
        }

        if let Ok(Some(event_duration)) = event.schedule_properties.get_duration() {
            return event_duration;
        }

        0
    }

    fn get_geo(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<GeoPoint> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.geo.is_some() {
                return event_occurrence_override.geo.clone();
            }
        }

        event.indexed_properties.geo.clone()
    }

    fn get_categories(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashSet<String>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Some(overridden_categories) = &event_occurrence_override.categories {
                return Some(overridden_categories.clone());
            }
        }

        event.indexed_properties.categories.clone()
    }

    fn get_related_to(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashMap<String, HashSet<String>>> {
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
    fn get_passive_properties(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> BTreeSet<KeyValuePair> {
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
            match passive_properties
                .iter()
                .find(|passive_property| passive_property.key == base_property.key)
            {
                Some(_) => {
                    continue;
                }

                None => {
                    passive_properties.insert(base_property.clone());
                }
            }
        }

        passive_properties
    }
}

impl ICalSerializer for EventInstance {
    fn serialize_to_ical_set(&self, timezone: &Tz) -> BTreeSet<KeyValuePair> {
        let mut serialized_ical_set = self.passive_properties.clone();

        serialized_ical_set.insert(ical_serializer::serialize_uid_to_ical(&self.uid));

        serialized_ical_set.insert(ical_serializer::serialize_dtstart_timestamp_to_ical(
            &self.dtstart_timestamp,
            &timezone,
        ));

        serialized_ical_set.insert(ical_serializer::serialize_dtend_timestamp_to_ical(
            &self.dtend_timestamp,
            &timezone,
        ));

        serialized_ical_set.append(
            &mut ical_serializer::serialize_indexed_categories_to_ical_set(&self.categories),
        );

        serialized_ical_set.append(&mut ical_serializer::serialize_indexed_related_to_ical_set(
            &self.related_to,
        ));

        if let Some(geo) = &self.geo {
            serialized_ical_set.insert(ical_serializer::serialize_indexed_geo_to_ical(geo));
        }

        serialized_ical_set.insert(KeyValuePair::new(
            String::from("RECURRENCE-ID"),
            format!(
                ";VALUE=DATE-TIME:{}",
                ical_serializer::serialize_timestamp_to_ical_utc_datetime(&self.dtstart_timestamp)
            ),
        ));

        serialized_ical_set
    }
}

impl PartialOrd for EventInstance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let dtstart_timestamp_comparison =
            self.dtstart_timestamp.partial_cmp(&other.dtstart_timestamp);

        if dtstart_timestamp_comparison.is_some_and(|comparison| comparison.is_eq()) {
            self.dtend_timestamp.partial_cmp(&other.dtend_timestamp)
        } else {
            dtstart_timestamp_comparison
        }
    }
}

impl Ord for EventInstance {
    fn cmp(&self, other: &Self) -> Ordering {
        let dtstart_timestamp_comparison = self.dtstart_timestamp.cmp(&other.dtstart_timestamp);

        if dtstart_timestamp_comparison.is_eq() {
            self.dtend_timestamp.cmp(&other.dtend_timestamp)
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

    use crate::core::{IndexedProperties, PassiveProperties, ScheduleProperties};

    use crate::testing::utils::{build_event_and_overrides_from_ical, build_event_from_ical};
    use pretty_assertions_sorted::{assert_eq, assert_eq_sorted};

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
                "GEO:48.85299;2.36885",
                "DESCRIPTION:Event description text.",
                "LOCATION:Event address text.",
            ],
        );

        let event_instance = EventInstance::new(&100, &event, None);

        assert_eq_sorted!(
            event_instance,
            EventInstance {
                uid: String::from("event_UID"),
                dtstart_timestamp: 100,
                dtend_timestamp: 160,
                duration: 60,
                geo: Some(GeoPoint::new(2.36885, 48.85299,)),
                categories: Some(HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                    String::from("CATEGORY THREE")
                ])),
                related_to: Some(HashMap::from([
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
                            String::from("ParentUID_One"),
                            String::from("ParentUID_Two"),
                        ])
                    ),
                    (
                        String::from("CHILD"),
                        HashSet::from([String::from("ChildUID"),])
                    )
                ])),
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
            vec![(
                "20201231T183000Z",
                vec![
                    "LOCATION:Overridden Event address text.",
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                    "RELATED-TO;RELTYPE=CHILD:ChildUID",
                    "RELATED-TO;RELTYPE=PARENT:ParentUID_Three",
                    "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                    "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four",
                ],
            )],
        );

        let Some(event_occurrence_override) = event.overrides.get(&1609439400) else {
            panic!("Expected event to have an occurrence...");
        };

        let event_instance =
            EventInstance::new(&1609439400, &event, Some(&event_occurrence_override));

        assert_eq!(
            event_instance,
            EventInstance {
                uid: String::from("event_UID"),
                dtstart_timestamp: 1609439400,
                dtend_timestamp: 1609439460,
                duration: 60,
                geo: None,
                categories: Some(HashSet::from([
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_FOUR"),
                ])),
                related_to: Some(HashMap::from([
                    (
                        String::from("X-IDX-CAL"),
                        HashSet::from([
                            String::from("redical//IndexedCalendar_One"),
                            String::from("redical//IndexedCalendar_Four"),
                        ])
                    ),
                    (
                        String::from("PARENT"),
                        HashSet::from([String::from("ParentUID_Three"),])
                    ),
                    (
                        String::from("CHILD"),
                        HashSet::from([String::from("ChildUID"),])
                    )
                ])),
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
            event_instance.serialize_to_ical(&Tz::UTC),
            vec![
                String::from("CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE"),
                String::from("DESCRIPTION:Event description text."),
                String::from("DTEND:20201231T183100Z"),
                String::from("DTSTART:20201231T183000Z"),
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
                (
                    "20210105T183000Z",
                    vec![
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:BASE_CATEGORY_ONE,OVERRIDDEN_CATEGORY_ONE",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID",
                    ],
                ),
                (
                    "20210112T183000Z",
                    vec![
                        "RELATED-TO;RELTYPE=CHILD:BASE_ChildUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID",
                    ],
                ),
                (
                    "20210126T183000Z",
                    vec![
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID",
                    ],
                ),
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
