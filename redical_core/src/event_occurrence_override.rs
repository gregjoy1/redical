use std::collections::BTreeSet;

use std::str::FromStr;

use crate::event::{IndexedProperties, PassiveProperties};

use redical_ical::{
    ICalendarComponent,
    RenderingContext,
    content_line::ContentLine,
    properties::{
        ICalendarProperty,
        ICalendarDateTimeProperty,
        EventProperty,
        EventProperties,
        DTStartProperty,
        DTEndProperty,
        DurationProperty,
    },
};

use redical_ical::values::date_time::DateTime as ICalDateTime;

#[derive(Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverride {
    pub indexed_properties: IndexedProperties,
    pub passive_properties: PassiveProperties,

    pub dtstart: Option<DTStartProperty>,
    pub dtend: Option<DTEndProperty>,
    pub duration: Option<DurationProperty>,
}

impl Default for EventOccurrenceOverride {
    fn default() -> Self {
        EventOccurrenceOverride {
            indexed_properties: IndexedProperties::new(),
            passive_properties: PassiveProperties::new(),
            dtstart: None,
            dtend: None,
            duration: None,
        }
    }
}

impl EventOccurrenceOverride {
    pub fn set_dtstart_timestamp(&mut self, dtstart_timestamp: i64) {
        self.dtstart = Some(DTStartProperty::new_from_utc_timestamp(&dtstart_timestamp));
    }

    pub fn get_dtstart_timestamp(&self) -> Option<i64> {
        self.dtstart
            .as_ref()
            .and_then(|dtstart| Some(dtstart.get_utc_timestamp()))
    }

    pub fn get_dtend_timestamp(&self) -> Option<i64> {
        self.dtend
            .as_ref()
            .and_then(|dtend| Some(dtend.get_utc_timestamp()))
    }

    pub fn get_duration_in_seconds(&self) -> Option<i64> {
        if let Some(parsed_duration) = self.duration.as_ref() {
            return Some(parsed_duration.duration.get_duration_in_seconds());
        }

        match (self.get_dtstart_timestamp(), self.get_dtend_timestamp()) {
            (Some(dtstart_timestamp), Some(dtend_timestamp)) => {
                Some(dtend_timestamp - dtstart_timestamp)
            }

            _ => None,
        }
    }

    pub fn parse_ical(dtstart_date_string: &str, input: &str) -> Result<EventOccurrenceOverride, String> {
        EventProperties::from_str(input).and_then(|EventProperties(parsed_properties)| {
            let mut new_override = EventOccurrenceOverride::default();

            for parsed_property in parsed_properties {
                new_override.insert(parsed_property)?;
            }

            let Ok(dtstart_datetime) = ICalDateTime::from_str(dtstart_date_string) else {
                return Err(
                    format!("Event occurrence override datetime: {dtstart_date_string} is not a valid datetime format.")
                );
            };

            new_override.set_dtstart_timestamp(dtstart_datetime.get_utc_timestamp(None));

            new_override.validate()?;

            Ok(new_override)
        })
    }

    pub fn validate(&self) -> Result<bool, String> {
        if self.dtstart.is_none() {
            return Err(
                String::from("Event occurrence override innvalid, expected DTSTART to be defined.")
            );
        }

        Ok(true)
    }

    pub fn insert(&mut self, property: EventProperty) -> Result<&Self, String> {
        match property {
            EventProperty::Class(_)
            | EventProperty::Geo(_)
            | EventProperty::Categories(_)
            | EventProperty::RelatedTo(_) => {
                self.indexed_properties.insert(property)?;
            }

            EventProperty::RRule(_) => {
                return Err(String::from(
                    "Event occurrence override does not expect an RRULE property",
                ));
            }

            EventProperty::ExRule(_) => {
                return Err(String::from(
                    "Event occurrence override does not expect an EXRULE property",
                ));
            }

            EventProperty::RDate(_) => {
                return Err(String::from(
                    "Event occurrence override does not expect an RDATE property",
                ));
            }

            EventProperty::ExDate(_) => {
                return Err(String::from(
                    "Event occurrence override does not expect an EXDATE property",
                ));
            }

            EventProperty::DTStart(dtstart_property) => {
                self.dtstart = Some(dtstart_property);
            }

            EventProperty::DTEnd(dtend_property) => {
                self.dtend = Some(dtend_property);
            }

            EventProperty::Duration(duration_property) => {
                self.duration = Some(duration_property);
            }

            _ => {
                self.passive_properties.insert(property)?;
            }
        }

        Ok(self)
    }
}

impl ICalendarComponent for EventOccurrenceOverride {
    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine> {
        let mut serializable_properties: BTreeSet<ContentLine> = BTreeSet::new();

        if let Some(dtstart_property) = &self.dtstart {
            serializable_properties.insert(dtstart_property.to_content_line_with_context(context));
        }

        if let Some(dtend_property) = &self.dtend {
            serializable_properties.insert(dtend_property.to_content_line_with_context(context));
        }

        if let Some(duration_property) = &self.duration {
            serializable_properties.insert(duration_property.to_content_line_with_context(context));
        }

        if let Some(geo_property) = &self.indexed_properties.geo {
            serializable_properties.insert(geo_property.to_content_line_with_context(context));
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

        serializable_properties
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{BTreeSet, HashSet};

    use redical_ical::properties::{
        PassiveProperty,
        ClassProperty,
        CategoriesProperty,
    };

    use crate::testing::macros::build_property_from_ical;

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical() {
        let ical_with_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical("19700101T000500Z", ical_with_rrule),
            Err(String::from(
                "Event occurrence override does not expect an RRULE property"
            ))
        );

        let ical_with_different_dtstart: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA DTSTART:19700202T000500Z";

        // Expect the DTSTART in the ical to be overridden by the date string provided to parse_ical.
        assert_eq!(
            EventOccurrenceOverride::parse_ical("19700101T000500Z", ical_with_different_dtstart),
            Ok(
                EventOccurrenceOverride {
                    indexed_properties: IndexedProperties {
                        geo: None,
                        class: None,
                        categories: None,
                        related_to: None,
                    },

                    passive_properties: PassiveProperties {
                        properties: BTreeSet::from([build_property_from_ical!(PassiveProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA")]),
                    },

                    duration: None,
                    dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000500Z")),
                    dtend: None,
                }
            )
        );

        let ical_without_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA CLASS:PRIVATE CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY (THREE)\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical("19700101T000500Z", ical_without_rrule).unwrap(),
            EventOccurrenceOverride {
                indexed_properties: IndexedProperties {
                    geo: None,
                    class: Some(build_property_from_ical!(ClassProperty, "CLASS:PRIVATE")),
                    categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY (THREE)\"")])),
                    related_to: None,
                },

                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([build_property_from_ical!(PassiveProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA")]),
                },

                duration: None,
                dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000500Z")),
                dtend: None,
            }
        );
    }
}
