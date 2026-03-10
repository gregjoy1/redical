use crate::core::{Calendar, Event, EventOccurrenceOverride};

pub fn build_test_calendar() -> Calendar {
    let mut calendar = Calendar::new(String::from("LOAD_TEST_UID"));

    let mut event = Event::parse_ical(
        "EVENT_UID",
        "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 \
         CLASS:PUBLIC CATEGORIES:CATEGORY_ONE \
         DTSTART:19700101T000500Z \
         LAST-MODIFIED:19700101T010500Z",
    ).unwrap();

    let event_override = EventOccurrenceOverride::parse_ical(
        "19700101T000500Z",
        "CLASS:PRIVATE \
         CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO \
         LAST-MODIFIED:19700101T020500Z",
    ).unwrap();

    event.override_occurrence(&event_override, true).unwrap();
    event.validate().unwrap();

    calendar.insert_event(event);
    calendar.validate_and_rebuild_indexes().unwrap();

    calendar
}

pub fn fixture_path(filename: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(filename)
}
