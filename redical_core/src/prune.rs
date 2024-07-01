use std::ops::Bound;

use crate::{Event, EventOccurrenceOverride};

pub fn prune_event_overrides(event: &mut Event, from_timestamp_bound: Bound<i64>, until_timestamp_bound: Bound<i64>, mut callback: impl FnMut(i64, EventOccurrenceOverride)) -> Result<bool, String> {
    let from_timestamp =
        match from_timestamp_bound {
            Bound::Included(from_timestamp) => from_timestamp,
            Bound::Excluded(from_timestamp) => from_timestamp,

            Bound::Unbounded => {
                return Err(format!("Lower bound cannot be unbounded and have no value"));
            }
        };

    let until_timestamp =
        match until_timestamp_bound {
            Bound::Included(until_timestamp) => until_timestamp,
            Bound::Excluded(until_timestamp) => until_timestamp,

            Bound::Unbounded => {
                return Err(format!("Upper bound cannot be unbounded and have no value"));
            }
        };

    // Further bounds validation - ensuring `from_timestamp` is less than `until_timestamp`.
    // Also prevent `event.overrides.range` from panicing (if provided with equal values both
    // wrapped in `Bound::Excluded`.
    match (from_timestamp_bound, until_timestamp_bound) {
        (Bound::Excluded(from_timestamp), Bound::Excluded(until_timestamp)) => {
            return Err(format!("Lower bound (excluded) value: {from_timestamp} cannot be equal to upper bound (excluded) value: {until_timestamp}"));
        },

        _ => {
            if from_timestamp > until_timestamp {
                return Err(format!("Lower bound value: {from_timestamp} cannot be greater than upper bound value: {until_timestamp}"));
            }
        },
    }

    let timestamps_to_remove: Vec<i64> =
        event.overrides
             .range((from_timestamp_bound, until_timestamp_bound))
             .map(|(timestamp, _)| timestamp.to_owned())
             .collect();

    for timestamp_to_remove in timestamps_to_remove {
        if let Ok(Some(removed_event_occurrence_override)) = event.remove_occurrence_override(timestamp_to_remove, true) {
            callback(timestamp_to_remove, removed_event_occurrence_override);
        }
    }

    Ok(true)
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{HashSet, HashMap, BTreeMap, BTreeSet};
    use std::str::FromStr;

    use redical_ical::{
        properties::{
            LastModifiedProperty,
            CategoriesProperty,
        },
    };

    use crate::IndexedConclusion;
    use crate::inverted_index::InvertedEventIndex;
    use crate::{IndexedProperties, PassiveProperties, ScheduleProperties};
    use crate::testing::macros::build_property_from_ical;

    use pretty_assertions_sorted::assert_eq;

    // Override 100 has all event categories plus CATEGORY_FOUR
    fn build_event_occurrence_override_100() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE,CATEGORY_FOUR"
                )])),
                class: None,
            },
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }

    // Override 200 has only some event categories (missing CATEGORY_THREE)
    fn build_event_occurrence_override_200() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO"
                )])),
                class: None,
            },
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }

    // Override 300 has no overridden categories
    fn build_event_occurrence_override_300() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties::new(),
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }

    // Override 400 has removed all categories
    fn build_event_occurrence_override_400() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::new()),
                class: None,
            },
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }

    // Override 500 has no base event categories, but does have CATEGORY_FOUR
    fn build_event_occurrence_override_500() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),
            indexed_properties: IndexedProperties {
                geo: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_FOUR"
                )])),
                class: None,
            },
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }

    fn build_event() -> Event {
        Event {
            uid: String::from("event_UID").into(),
            last_modified: build_property_from_ical!(LastModifiedProperty, "LAST-MODIFIED:20201230T173000Z"),

            schedule_properties: ScheduleProperties {
                rrule: None,
                exrule: None,
                rdates: None,
                exdates: None,
                duration: None,
                dtstart: None,
                dtend: None,
                parsed_rrule_set: None,
            },

            indexed_properties: IndexedProperties {
                geo: None,
                class: None,
                related_to: None,
                location_type: None,
                categories: Some(HashSet::from([build_property_from_ical!(
                    CategoriesProperty,
                    "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE"
                )])),
            },

            passive_properties: PassiveProperties {
                properties: BTreeSet::new(),
            },

            overrides: BTreeMap::from([
                (100, build_event_occurrence_override_100()), // Override 100 has all event categories plus CATEGORY_FOUR
                (200, build_event_occurrence_override_200()), // Override 200 has only some event categories (missing CATEGORY_THREE)
                (300, build_event_occurrence_override_300()), // Override 300 has no overridden categories
                (400, build_event_occurrence_override_400()), // Override 400 has removed all categories
                (500, build_event_occurrence_override_500()), // Override 500 has no base event categories, but does have CATEGORY_FOUR
            ]),
            indexed_categories: None,
            indexed_related_to: None,
            indexed_geo: None,
            indexed_class: None,
        }
    }

    #[test]
    fn test_prune_event_overrides() {
        let mut event = build_event();

        let mut callback_values: Vec<(i64, EventOccurrenceOverride)> = Vec::new();

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Excluded(125),
                std::ops::Bound::Included(400),
                |timestamp, event_occurrence_override| {
                    callback_values.push((timestamp, event_occurrence_override));
                },
            ),
            Ok(true),
        );

        assert_eq!(
            callback_values,
            vec![
                (200, build_event_occurrence_override_200()), // Override 200 has only some event categories (missing CATEGORY_THREE)
                (300, build_event_occurrence_override_300()), // Override 300 has no overridden categories
                (400, build_event_occurrence_override_400()), // Override 400 has removed all categories
            ],
        );

        assert_eq!(
            event.overrides,
            BTreeMap::from([
                (100, build_event_occurrence_override_100()), // Override 100 has all event categories plus CATEGORY_FOUR
                (500, build_event_occurrence_override_500()), // Override 500 has no base event categories, but does have CATEGORY_FOUR
            ]),
        );
    }

    #[test]
    fn test_prune_event_overrides_bounds_validation() {
        let mut event = build_event();

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Excluded(125),
                std::ops::Bound::Excluded(125),
                |_timestamp, _event_occurrence_override| {},
            ),
            Err(String::from("Lower bound (excluded) value: 125 cannot be equal to upper bound (excluded) value: 125")),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Included(125),
                std::ops::Bound::Excluded(125),
                |_timestamp, _event_occurrence_override| {},
            ),
            Ok(true),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Excluded(125),
                std::ops::Bound::Included(125),
                |_timestamp, _event_occurrence_override| {},
            ),
            Ok(true),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Included(125),
                std::ops::Bound::Included(125),
                |_timestamp, _event_occurrence_override| {},
            ),
            Ok(true),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Excluded(125),
                std::ops::Bound::Included(120),
                |_timestamp, _event_occurrence_override| {},
            ),
            Err(String::from("Lower bound value: 125 cannot be greater than upper bound value: 120")),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Excluded(125),
                std::ops::Bound::Included(140),
                |_timestamp, _event_occurrence_override| {},
            ),
            Ok(true),
        );

        assert_eq!(
            prune_event_overrides(
                &mut event,
                std::ops::Bound::Included(125),
                std::ops::Bound::Excluded(140),
                |_timestamp, _event_occurrence_override| {},
            ),
            Ok(true),
        );
    }
}
