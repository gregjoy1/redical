use crate::core::{Calendar, Event, EventOccurrenceOverride};

use rayon::prelude::*;

use serde::{Deserialize, Serialize};

use std::str::FromStr;

use redical_ical::ICalendarComponent;
use redical_ical::properties::{EventProperty, CalendarProperty};

#[derive(Debug, PartialEq)]
pub enum ParseRDBEntityError {
    OnSelf(String, String),
    OnChild(String, Box<ParseRDBEntityError>),
}

impl From<ParseRDBEntityError> for String {
    fn from(parse_error: ParseRDBEntityError) -> String {
        parse_error.to_string()
    }
}

impl std::fmt::Display for ParseRDBEntityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut error_string = String::from("Error at ");

        let mut pointer = Some(self);

        while let Some(current_error) = pointer {
            match current_error {
                Self::OnSelf(current_name, error_message) => {
                    error_string.push_str(
                        format!("{}:{}", current_name, error_message).as_str()
                    );

                    pointer = None;
                },

                Self::OnChild(current_name, child_error) => {
                    error_string.push_str(
                        format!("{} -> ", current_name).as_str()
                    );

                    pointer = Some(child_error.as_ref());
                },
            }
        }

        write!(f, "{}", error_string)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RDBCalendar(String, Vec<String>, Vec<RDBEvent>);

impl TryFrom<&Calendar> for RDBCalendar {
    type Error = String;

    fn try_from(calendar: &Calendar) -> Result<Self, Self::Error> {
        let uid = calendar.uid.uid.to_string();

        let properties: Vec<String> = calendar.to_rendered_content_lines().into_iter().collect();

        let mut rdb_events: Vec<RDBEvent> = Vec::new();

        for event in calendar.events.values() {
            rdb_events.push(
                RDBEvent::try_from(event.as_ref())?
            );
        }

        Ok(
            RDBCalendar(uid, properties, rdb_events)
        )
    }
}

impl TryFrom<&RDBCalendar> for Calendar {
    type Error = ParseRDBEntityError;

    fn try_from(rdb_calendar: &RDBCalendar) -> Result<Self, Self::Error> {
        let rdb_calendar_uid = rdb_calendar.0.to_owned();

        let mut calendar = Calendar::new(rdb_calendar_uid.clone());

        for rdb_property in &rdb_calendar.1 {
            let property = CalendarProperty::from_str(rdb_property.as_str()).map_err(|error| ParseRDBEntityError::OnSelf(rdb_calendar_uid.to_string(), error))?;

            calendar.insert(property).map_err(|error| ParseRDBEntityError::OnSelf(rdb_calendar_uid.to_string(), error))?;
        }

        let parsed_calendar_uid = calendar.uid.uid.to_string();

        if rdb_calendar_uid != parsed_calendar_uid {
            return Err(
                ParseRDBEntityError::OnSelf(
                    rdb_calendar_uid.to_string(),
                    format!("Calendar UID property: {} does not match stored UID key: {}", parsed_calendar_uid, rdb_calendar_uid),
                )
            );
        }

        // Parallelise the parsed rehydration of the Calendar Events.
        let parse_event_results: Vec<Result<Event, ParseRDBEntityError>> =
            rdb_calendar.2.par_iter().map(|rdb_event: &RDBEvent| {
                Event::try_from(rdb_event).map_err(|error| ParseRDBEntityError::OnChild(rdb_calendar_uid.to_string(), Box::new(error)))
            }).collect();

        for parse_event_result in parse_event_results {
            calendar.insert_event(parse_event_result?);
        }

        calendar.rebuild_indexes().map_err(|error| ParseRDBEntityError::OnSelf(rdb_calendar_uid.to_string(), error))?;

        Ok(
            calendar
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RDBEvent(String, Vec<String>, Vec<RDBEventOccurrenceOverride>);

impl TryFrom<&Event> for RDBEvent {
    type Error = String;

    fn try_from(event: &Event) -> Result<Self, Self::Error> {
        let uid = event.uid.uid.to_string();

        let properties: Vec<String> = event.to_rendered_content_lines().into_iter().collect();

        let mut rdb_event_occurrence_overrides: Vec<RDBEventOccurrenceOverride> = Vec::new();

        for event_occurrence_override in event.overrides.values() {
            rdb_event_occurrence_overrides.push(
                RDBEventOccurrenceOverride::try_from(event_occurrence_override)?
            );
        }

        Ok(
            RDBEvent(uid, properties, rdb_event_occurrence_overrides)
        )
    }
}

impl TryFrom<&RDBEvent> for Event {
    type Error = ParseRDBEntityError;

    fn try_from(rdb_event: &RDBEvent) -> Result<Self, Self::Error> {
        let rdb_event_uid = rdb_event.0.to_owned();

        let mut event = Event::new(rdb_event_uid.clone());

        for rdb_property in &rdb_event.1 {
            let property =
                EventProperty::from_str(rdb_property.as_str())
                    .map_err(|error| ParseRDBEntityError::OnSelf(rdb_event_uid.to_string(), error))?;

            event.insert(property)
                 .map_err(|error| ParseRDBEntityError::OnSelf(rdb_event_uid.to_string(), error))?;
        }

        let parsed_event_uid = event.uid.uid.to_string();

        if rdb_event_uid != parsed_event_uid {
            return Err(
                ParseRDBEntityError::OnSelf(
                    rdb_event_uid.to_string(),
                    format!("Event UID property: {} does not match stored UID key: {}", parsed_event_uid, rdb_event_uid),
                )
            );
        }

        event.validate().map_err(|error| {
            ParseRDBEntityError::OnSelf(rdb_event_uid.to_string(), error)
        })?;

        // Parallelise the parsed rehydration of the Calendar Event EventOccurrenceOverrides.
        let parse_event_occurrence_override_results: Vec<Result<EventOccurrenceOverride, ParseRDBEntityError>> =
            rdb_event.2.par_iter().map(|rdb_event_occurrence_override: &RDBEventOccurrenceOverride| {
                EventOccurrenceOverride::try_from(rdb_event_occurrence_override)
                    .map_err(|error| ParseRDBEntityError::OnChild(rdb_event_uid.to_string(), Box::new(error)))
            }).collect();

        for parse_event_occurrence_override_result in parse_event_occurrence_override_results {
            event.override_occurrence(&parse_event_occurrence_override_result?, false).map_err(|error| ParseRDBEntityError::OnSelf(rdb_event_uid.to_string(), error))?;
        }

        event.rebuild_indexes().map_err(|error| ParseRDBEntityError::OnSelf(rdb_event_uid.to_string(), error))?;

        Ok(
            event
        )
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct RDBEventOccurrenceOverride(String, Vec<String>);

impl TryFrom<&EventOccurrenceOverride> for RDBEventOccurrenceOverride {
    type Error = String;

    fn try_from(event_occurrence_override: &EventOccurrenceOverride) -> Result<Self, Self::Error> {
        let Some(dtstart_property) = event_occurrence_override.dtstart.as_ref() else {
            return Err(String::from("EventOccurrenceOverride is invalid, requires defined DTSTART property"));
        };

        let occurrence_date_string = dtstart_property.date_time.render_formatted_date_time(None);

        let properties: Vec<String> = event_occurrence_override.to_rendered_content_lines().into_iter().collect();

        Ok(
            RDBEventOccurrenceOverride(occurrence_date_string, properties)
        )
    }
}

impl TryFrom<&RDBEventOccurrenceOverride> for EventOccurrenceOverride {
    type Error = ParseRDBEntityError;

    fn try_from(rdb_event_occurrence_override: &RDBEventOccurrenceOverride) -> Result<Self, Self::Error> {
        let rdb_date_time_string = rdb_event_occurrence_override.0.to_owned();

        let mut event_occurrence_override = EventOccurrenceOverride::default();

        for rdb_property in &rdb_event_occurrence_override.1 {
            let property = EventProperty::from_str(rdb_property.as_str()).map_err(|error| ParseRDBEntityError::OnSelf(rdb_date_time_string.to_owned(), error))?;

            event_occurrence_override.insert(property).map_err(|error| ParseRDBEntityError::OnSelf(rdb_date_time_string.to_owned(), error))?;
        }

        event_occurrence_override.validate().map_err(|error| ParseRDBEntityError::OnSelf(rdb_date_time_string.to_owned(), error))?;

        if let Some(dtstart) = event_occurrence_override.dtstart.as_ref() {
            let parsed_date_time_string = dtstart.date_time.render_formatted_date_time(None);

            if rdb_date_time_string != parsed_date_time_string {
                return Err(
                    ParseRDBEntityError::OnSelf(
                        rdb_date_time_string.to_owned(),
                        format!("EventOccurrenceOverride DTSTART property: {parsed_date_time_string} does not match stored DTSTART key: {rdb_date_time_string}"),
                    )
                );
            }
        }

        Ok(
            event_occurrence_override
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_calendar_rdb_entity() {
        let event_occurrence_override =
            EventOccurrenceOverride::parse_ical(
                "19700101T000500Z",
                "CLASS:PRIVATE CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO LAST-MODIFIED:19700101T020500Z",
            ).unwrap();

        let mut event =
            Event::parse_ical(
                "EVENT_UID",
                "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 CLASS:PUBLIC CATEGORIES:CATEGORY_ONE DTSTART:19700101T000500Z LAST-MODIFIED:19700101T010500Z",
            ).unwrap();

        event.override_occurrence(&event_occurrence_override, true).unwrap();

        event.validate().unwrap();
        event.rebuild_indexes().unwrap();

        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        calendar.insert_event(event.clone());

        calendar.rebuild_indexes().unwrap();

        let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();

        assert_eq!(
            rdb_calendar,
            RDBCalendar(
                String::from("CALENDAR_UID"),
                vec![
                    String::from("UID:CALENDAR_UID"),
                ],
                vec![
                    RDBEvent(
                        String::from("EVENT_UID"),
                        vec![
                            String::from("CATEGORIES:CATEGORY_ONE"),
                            String::from("CLASS:PUBLIC"),
                            String::from("DTSTART:19700101T000500Z"),
                            String::from("LAST-MODIFIED:19700101T010500Z"),
                            String::from("RRULE:FREQ=WEEKLY;INTERVAL=1;UNTIL=19700101T000500Z"),
                            String::from("UID:EVENT_UID"),
                        ],
                        vec![
                            RDBEventOccurrenceOverride(
                                String::from("19700101T000500Z"),
                                vec![
                                    String::from("CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO"),
                                    String::from("CLASS:PRIVATE"),
                                    String::from("DTSTART:19700101T000500Z"),
                                    String::from("LAST-MODIFIED:19700101T020500Z"),
                                ],
                            ),
                        ],
                    ),
                ],
            ),
        );

        assert_eq!(
            Calendar::try_from(&rdb_calendar),
            Ok(calendar),
        );
    }

    #[test]
    fn test_parse_invalid_calendar_event_rdb_entity() {
        let event_occurrence_override =
            EventOccurrenceOverride::parse_ical(
                "19700101T000500Z",
                "CLASS:PRIVATE CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"",
            ).unwrap();

        let mut event =
            Event::parse_ical(
                "EVENT_UID",
                "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 CLASS:PUBLIC CATEGORIES:CATEGORY_ONE DTSTART:19700101T000500Z",
            ).unwrap();

        event.override_occurrence(&event_occurrence_override, true).unwrap();

        event.validate().unwrap();
        event.rebuild_indexes().unwrap();

        let mut calendar = Calendar::new(String::from("CALENDAR_UID"));

        calendar.insert_event(event.clone());

        calendar.rebuild_indexes().unwrap();

        let invalid_rdb_calendar =
            RDBCalendar(
                String::from("CALENDAR_UID"),
                vec![
                    String::from("UID:CALENDAR_UID"),
                ],
                vec![
                    RDBEvent(
                        String::from("EVENT_UID"),
                        vec![
                            String::from("CATEGORIES:CATEGORY_ONE"),
                            String::from("CLASS:PUBLIC    "),
                            String::from("DTSTART:19700101T000500Z"),
                            String::from("RRULE:FREQ=WEEKLY;INTERVAL=1;UNTIL=19700101T000500Z"),
                            String::from("UID:EVENT_UID"),
                        ],
                        vec![
                        ],
                    ),
                ],
            );

        assert_eq!(
            Calendar::try_from(&invalid_rdb_calendar).map_err(String::from),
            Err(String::from("Error at CALENDAR_UID -> EVENT_UID:Error - parse error Eof at \"\"")),
        );
    }

    #[test]
    fn test_parse_invalid_event_occurrence_override_rdb_entity() {
        let invalid_rdb_event_occurrence_override =
            RDBEventOccurrenceOverride(
                String::from("19700101T000500Z"),
                vec![
                    String::from("CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO"),
                    String::from("CLASS:PRIVATE   "),
                    String::from("DTSTART:19700101T000500Z"),
                ],
            );

        assert_eq!(
            EventOccurrenceOverride::try_from(&invalid_rdb_event_occurrence_override).map_err(String::from),
            Err(String::from("Error at 19700101T000500Z:Error - parse error Eof at \"\"")),
        );
    }

    #[test]
    fn test_calendar_level_parse_rdb_entity_error_to_string() {
        assert_eq!(
            ParseRDBEntityError::OnSelf(
                String::from("CALENDAR_UID"),
                String::from("Calendar error message."),
            ).to_string(),
            String::from("Error at CALENDAR_UID:Calendar error message."),
        );
    }

    #[test]
    fn test_event_level_parse_rdb_entity_error_to_string() {
        assert_eq!(
            ParseRDBEntityError::OnChild(
                String::from("CALENDAR_UID"),
                Box::new(
                    ParseRDBEntityError::OnSelf(
                        String::from("EVENT_UID"),
                        String::from("Event error message."),
                    )
                ),
            ).to_string(),
            String::from("Error at CALENDAR_UID -> EVENT_UID:Event error message."),
        );
    }

    #[test]
    fn test_event_occurrence_override_level_parse_rdb_entity_error_to_string() {
        assert_eq!(
            ParseRDBEntityError::OnChild(
                String::from("CALENDAR_UID"),
                Box::new(
                    ParseRDBEntityError::OnChild(
                        String::from("EVENT_UID"),
                        Box::new(
                            ParseRDBEntityError::OnSelf(
                                String::from("EVENT_OCCURRENCE_OVERRIDE_UID"),
                                String::from("Event occurrence override error message."),
                            )
                        ),
                    )
                ),
            ).to_string(),
            String::from("Error at CALENDAR_UID -> EVENT_UID -> EVENT_OCCURRENCE_OVERRIDE_UID:Event occurrence override error message."),
        );
    }
}
