use std::cmp::Ordering;

use std::collections::{BTreeSet, HashSet, HashMap};

use crate::{Event, EventOccurrenceOverride, IndexedConclusion};

use crate::event_occurrence_iterator::{
    EventOccurrenceIterator, LowerBoundFilterCondition, UpperBoundFilterCondition,
};

use crate::event::{IndexedProperties, PassiveProperties};

use redical_ical::{
    ICalendarComponent,
    RenderingContext,
    content_line::ContentLine,
    properties::{
        ICalendarProperty,
        ICalendarDateTimeProperty,
        ICalendarGeoProperty,
        CategoriesProperty,
        LocationTypeProperty,
        ClassProperty,
        DTEndProperty,
        DTStartProperty,
        DurationProperty,
        GeoProperty,
        RelatedToProperty,
        UIDProperty,
        PassiveProperty,
        RecurrenceIDProperty,
    },
};

use crate::queries::results::QueryableEntity;
use crate::queries::results_ordering::{QueryResultOrdering, OrderingCondition};

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
        let uid = event.uid.clone();
        let dtstart = DTStartProperty::new_from_utc_timestamp(dtstart_timestamp);
        let dtend = DTEndProperty::new_from_utc_timestamp(&Self::get_dtend_timestamp(dtstart_timestamp, event, event_occurrence_override));
        let duration = DurationProperty::new_from_seconds(&Self::get_duration_in_seconds(event, event_occurrence_override));

        let indexed_properties = IndexedProperties {
            geo: Self::get_geo(event, event_occurrence_override),
            categories: Self::get_categories(event, event_occurrence_override),
            location_type: Self::get_location_type(event, event_occurrence_override),
            related_to: Self::get_related_to(event, event_occurrence_override),
            class: Self::get_class(event, event_occurrence_override),
        };

        let passive_properties = PassiveProperties {
            properties: Self::get_passive_properties(event, event_occurrence_override),
        };

        EventInstance {
            uid,
            dtstart,
            dtend,
            duration,
            indexed_properties,
            passive_properties,
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
            if let Some(overridden_duration) = event_occurrence_override.get_duration_in_seconds() {
                return overridden_duration;
            }
        }

        if let Some(event_duration) = event.schedule_properties.get_duration_in_seconds() {
            return event_duration;
        }

        0
    }

    fn get_geo(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<GeoProperty> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if let Some(overridden_geo_property) = event_occurrence_override.indexed_properties.geo.as_ref() {
                if overridden_geo_property.is_present() {
                    return Some(overridden_geo_property.to_owned());
                } else {
                    return None;
                }
            }
        }

        event.indexed_properties.geo.to_owned()
    }

    fn get_location_type(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<LocationTypeProperty> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override
                .indexed_properties
                .location_type
                .is_some()
            {
                return event_occurrence_override
                    .indexed_properties
                    .location_type
                    .to_owned();
            }
        }

        event.indexed_properties.location_type.to_owned()
    }

    fn get_categories(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashSet<CategoriesProperty>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override
                .indexed_properties
                .categories
                .is_some()
            {
                return event_occurrence_override
                    .indexed_properties
                    .categories
                    .to_owned();
            }
        }

        event.indexed_properties.categories.to_owned()
    }

    fn get_related_to(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<HashSet<RelatedToProperty>> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override
                .indexed_properties
                .related_to
                .is_some()
            {
                return event_occurrence_override
                    .indexed_properties
                    .related_to
                    .to_owned();
            }
        }

        event.indexed_properties.related_to.to_owned()
    }

    // This gets all resulting passive properties for the event instance where any overrides are
    // merged ontop of the passive properties defined within the base event.
    //
    // We do this grouped via the property name so that overrides are applied on a property name
    // level only which allows the patching of groups of specific overridden property names.
    fn get_passive_properties(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> BTreeSet<PassiveProperty> {
        let mut grouped_passive_properties: HashMap<String, Vec<PassiveProperty>> = event.passive_properties.extract_properties_grouped_by_name();

        if let Some(event_occurrence_override) = event_occurrence_override {
            for (property_name, grouped_properties) in event_occurrence_override.passive_properties.extract_properties_grouped_by_name() {
                grouped_passive_properties.insert(property_name, grouped_properties);
            }
        }

        BTreeSet::from_iter(grouped_passive_properties.values().flatten().cloned())
    }

    fn get_class(
        event: &Event,
        event_occurrence_override: Option<&EventOccurrenceOverride>,
    ) -> Option<ClassProperty> {
        if let Some(event_occurrence_override) = event_occurrence_override {
            if event_occurrence_override.indexed_properties.class.is_some() {
                return event_occurrence_override
                    .indexed_properties
                    .class
                    .to_owned();
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
        RecurrenceIDProperty::new_from_utc_timestamp(&self.dtstart.get_utc_timestamp())
    }
}

impl QueryableEntity for EventInstance {
    fn get_uid(&self) -> String {
        self.uid.uid.to_string()
    }

    fn build_result_ordering(&self, ordering_condition: &OrderingCondition) -> QueryResultOrdering {
        ordering_condition.build_result_ordering_for_event_instance(self)
    }
}

impl ICalendarComponent for EventInstance {
    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine> {
        let mut serializable_properties: BTreeSet<ContentLine> = BTreeSet::new();

        serializable_properties.insert(self.uid.to_content_line_with_context(context));
        serializable_properties.insert(self.dtstart.to_content_line_with_context(context));
        serializable_properties.insert(self.dtend.to_content_line_with_context(context));
        serializable_properties.insert(self.duration.to_content_line_with_context(context));

        if let Some(geo_property) = &self.indexed_properties.geo {
            serializable_properties.insert(geo_property.to_content_line_with_context(context));
        }

        if let Some(location_type_property) = &self.indexed_properties.location_type {
            serializable_properties.insert(location_type_property.to_content_line_with_context(context));
        }

        if let Some(class_property) = &self.indexed_properties.class {
            serializable_properties.insert(class_property.to_content_line_with_context(context));
        }

        if let Some(related_to_properties) = &self.indexed_properties.related_to {
            for related_to_property in related_to_properties {
                serializable_properties.insert(related_to_property.to_content_line_with_context(context));
            }
        }

        if let Some(categories_properties) = &self.indexed_properties.categories {
            for categories_property in categories_properties {
                serializable_properties.insert(categories_property.to_content_line_with_context(context));
            }
        }

        for passive_property in &self.passive_properties.properties {
            serializable_properties.insert(passive_property.to_content_line_with_context(context));
        }

        serializable_properties.insert(
            self.build_recurrence_id_from_dtstart()
                .to_content_line_with_context(context),
        );

        serializable_properties
    }
}

impl PartialOrd for EventInstance {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EventInstance {
    fn cmp(&self, other: &Self) -> Ordering {
        let dtstart_timestamp_comparison =
            self.dtstart.get_utc_timestamp().cmp(&other.dtstart.get_utc_timestamp());

        if dtstart_timestamp_comparison.is_eq() {
            self.dtend.get_utc_timestamp().cmp(&other.dtend.get_utc_timestamp())
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

impl Iterator for EventInstanceIterator<'_> {
    type Item = EventInstance;

    fn next(&mut self) -> Option<Self::Item> {
        // Filter occurrence index iterator timestamps according to IndexedConclusion if
        // present, else include all.
        self.internal_iter.next().map(
            |(dtstart_timestamp, _dtend_timestamp, event_occurrence_override)| {
                EventInstance::new(
                    &dtstart_timestamp,
                    self.event,
                    event_occurrence_override.as_ref(),
                )
            },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::str::FromStr;

    use chrono_tz::Tz;

    use std::collections::{BTreeSet, HashMap, HashSet};

    use crate::IndexedConclusion;

    use crate::testing::utils::{build_event_and_overrides_from_ical, build_event_from_ical};
    use crate::testing::macros::build_property_from_ical;

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
                "CLASS:PRIVATE",
                "LOCATION-TYPE:LOCATION_TYPE_ONE,LOCATION_TYPE_TWO",
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
                    geo: Some(build_property_from_ical!(
                        GeoProperty,
                        "GEO:48.85299;2.36885"
                    )),
                    location_type: Some(build_property_from_ical!(
                        LocationTypeProperty,
                        "LOCATION-TYPE:LOCATION_TYPE_ONE,LOCATION_TYPE_TWO"
                    )),
                    categories: Some(HashSet::from([build_property_from_ical!(
                        CategoriesProperty,
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE"
                    )])),
                    related_to: Some(HashSet::from([
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Two"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Three"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=PARENT:ParentUID_One"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=PARENT:ParentUID_Two"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=CHILD:ChildUID"
                        ),
                    ])),
                },
                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([
                        build_property_from_ical!(PassiveProperty, "DESCRIPTION:Event description text."),
                        build_property_from_ical!(PassiveProperty, "LOCATION:Event address text."),
                    ])
                },
            }
        );

        let rendering_context = RenderingContext {
            tz: Some(Tz::Europe__London),
            distance_unit: None,
        };

        assert_eq!(
            event_instance.to_rendered_content_lines_with_context(Some(&rendering_context)),
            vec![
                String::from("CATEGORIES:CATEGORY THREE,CATEGORY_ONE,CATEGORY_TWO"),
                String::from("CLASS:PRIVATE"),
                String::from("DESCRIPTION:Event description text."),
                String::from("DTEND;TZID=Europe/London:19700101T010240"),
                String::from("DTSTART;TZID=Europe/London:19700101T010140"),
                String::from("DURATION:PT1M"),
                String::from("GEO:48.85299;2.36885"),
                String::from("LOCATION:Event address text."),
                String::from("LOCATION-TYPE:LOCATION_TYPE_ONE,LOCATION_TYPE_TWO"),
                String::from("RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/London:19700101T010140"),
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
                (
                    "20201231T183000Z",
                    vec![
                        "LOCATION:Overridden Event address text.",
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR",
                        "RELATED-TO;RELTYPE=CHILD:ChildUID",
                        "RELATED-TO;RELTYPE=PARENT:ParentUID_Three",
                        "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One",
                        "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four",
                    ]
                ),
            ],
        );

        let Some(event_occurrence_override) = event.overrides.get(&1609439400) else {
            panic!("Expected event to have an occurrence...");
        };

        let event_instance =
            EventInstance::new(&1609439400, &event, Some(event_occurrence_override));

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
                    location_type: None,
                    categories: Some(HashSet::from([build_property_from_ical!(
                        CategoriesProperty,
                        "CATEGORIES:CATEGORY_ONE,CATEGORY_FOUR"
                    ),])),
                    related_to: Some(HashSet::from([
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_One"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=X-IDX-CAL:redical//IndexedCalendar_Four"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=PARENT:ParentUID_Three"
                        ),
                        build_property_from_ical!(
                            RelatedToProperty,
                            "RELATED-TO;RELTYPE=CHILD:ChildUID"
                        ),
                    ])),
                },
                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([
                        build_property_from_ical!(PassiveProperty, "DESCRIPTION:Event description text."),
                        build_property_from_ical!(PassiveProperty, "LOCATION:Overridden Event address text."),
                    ])
                },
            }
        );

        assert_eq!(
            event_instance.to_rendered_content_lines(),
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
                "X-ENTITY:ENTITY_UID",
                "X-TICKET;X-COST=100:Ticket Name One",
                "X-TICKET;X-COST=200:Ticket Name Two",
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
                        "X-ENTITY:OVERRIDDEN_ENTITY_UID",
                    ],
                ),
                (
                    "20210126T183000Z",
                    vec![
                        "DESCRIPTION:OVERRIDDEN description text.",
                        "CATEGORIES:OVERRIDDEN_CATEGORY_ONE,OVERRIDDEN_CATEGORY_TWO",
                        "RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID",
                        "RELATED-TO;RELTYPE=CHILD:OVERRIDDEN_ChildUID",
                        "X-TICKET;X-COST=300:Ticket Name Three",
                    ],
                ),
                (
                    "20210202T183000Z",
                    vec![
                        "X-TICKET;X-COST=300:Ticket Name Three",
                        "X-TICKET;X-COST=400:Ticket Name Four",
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
                    String::from("DURATION:PT30M"),
                    String::from("RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z"),
                    String::from("RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_ParentdUID"),
                    String::from("UID:event_UID"),
                    String::from("X-ENTITY:ENTITY_UID"),
                    String::from("X-TICKET;X-COST=100:Ticket Name One"),
                    String::from("X-TICKET;X-COST=200:Ticket Name Two"),
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
                    String::from("X-ENTITY:OVERRIDDEN_ENTITY_UID"),
                    String::from("X-TICKET;X-COST=100:Ticket Name One"),
                    String::from("X-TICKET;X-COST=200:Ticket Name Two"),
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
                    String::from("X-ENTITY:ENTITY_UID"),
                    String::from("X-TICKET;X-COST=100:Ticket Name One"),
                    String::from("X-TICKET;X-COST=200:Ticket Name Two"),
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
                    String::from("X-ENTITY:ENTITY_UID"),
                    String::from("X-TICKET;X-COST=300:Ticket Name Three"),
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
                    String::from("X-ENTITY:ENTITY_UID"),
                    String::from("X-TICKET;X-COST=300:Ticket Name Three"),
                    String::from("X-TICKET;X-COST=400:Ticket Name Four"),
                ],
            ),
        ]);

        // Testing without any filtered index conclusion
        let mut event_instance_iterator =
            EventInstanceIterator::new(&event, None, None, None, None).unwrap();

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1611685800].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
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
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1611685800].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
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
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1610476200].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
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
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1609871400].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1611081000].clone())
        );

        assert_eq!(
            event_instance_iterator.next().map(|event_instance| event_instance.to_rendered_content_lines()),
            Some(expected_event_instances_ical[&1612290600].clone())
        );

        assert_eq!(event_instance_iterator.next(), None);
    }
}
