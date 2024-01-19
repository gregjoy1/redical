use crate::core::{Event, EventOccurrenceOverride};

pub fn build_event_from_ical(event_uid: &str, event_ical_parts: Vec<&str>) -> crate::core::Event {
    build_event_and_overrides_from_ical(event_uid, event_ical_parts, vec![])
}

pub fn build_event_and_overrides_from_ical(
    event_uid: &str,
    event_ical_parts: Vec<&str>,
    event_overrides: Vec<Vec<&str>>,
) -> crate::core::Event {
    let mut event = Event::parse_ical(event_uid, event_ical_parts.join(" ").as_str()).unwrap();

    if let Err(error) = event.schedule_properties.build_parsed_rrule_set() {
        panic!("Build Event '{event_uid}' from ical failed -- build_parsed_rrule_set returned error: {:#?}", error);
    }

    for override_ical_parts in event_overrides {
        let parsed_event_occurrence_override = build_event_override_from_ical(override_ical_parts);

        assert!(event
            .override_occurrence(
                parsed_event_occurrence_override.get_dtstart_timestamp().unwrap().unwrap(),
                &parsed_event_occurrence_override,
            )
            .is_ok());
    }

    assert!(event.rebuild_indexed_categories().is_ok());
    assert!(event.rebuild_indexed_related_to().is_ok());

    event
}

pub fn build_event_override_from_ical(event_override_ical_parts: Vec<&str>) -> EventOccurrenceOverride {
    EventOccurrenceOverride::parse_ical(event_override_ical_parts.join(" ").as_str()).unwrap()
}
