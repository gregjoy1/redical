use std::collections::BTreeSet;

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::core::event::{IndexedProperties, PassiveProperties};

use crate::core::ical::serializer::{SerializableICalComponent, SerializableICalProperty, SerializationPreferences};

use crate::core::ical::parser::datetime::{datestring_to_date, ParseError};

use crate::core::ical::properties::{
    DTEndProperty, DTStartProperty, DurationProperty, Properties, Property,
};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
    pub fn get_dtstart_timestamp(&self) -> Option<i64> {
        self.dtstart
            .as_ref()
            .and_then(|dtstart| Some(dtstart.utc_timestamp.to_owned()))
    }

    pub fn get_dtend_timestamp(&self) -> Option<i64> {
        self.dtend
            .as_ref()
            .and_then(|dtend| Some(dtend.utc_timestamp.to_owned()))
    }

    pub fn get_duration_in_seconds(&self) -> Option<i64> {
        if let Some(parsed_duration) = self.duration.as_ref() {
            return Some(parsed_duration.get_duration_in_seconds());
        }

        match (self.get_dtstart_timestamp(), self.get_dtend_timestamp()) {
            (Some(dtstart_timestamp), Some(dtend_timestamp)) => {
                Some(dtend_timestamp - dtstart_timestamp)
            }

            _ => None,
        }
    }

    pub fn parse_ical(input: &str) -> Result<EventOccurrenceOverride, String> {
        Properties::from_str(input).and_then(|Properties(parsed_properties)| {
            let mut new_override = EventOccurrenceOverride::default();

            for parsed_property in parsed_properties {
                match parsed_property {
                    Property::Class(_)
                    | Property::Geo(_)
                    | Property::Categories(_)
                    | Property::RelatedTo(_) => {
                        new_override.indexed_properties.insert(parsed_property)?;
                    }

                    Property::RRule(_) => {
                        return Err(String::from(
                            "Event occurrence override does not expect an RRULE property",
                        ));
                    }

                    Property::ExRule(_) => {
                        return Err(String::from(
                            "Event occurrence override does not expect an EXRULE property",
                        ));
                    }

                    Property::RDate(_) => {
                        return Err(String::from(
                            "Event occurrence override does not expect an RDATE property",
                        ));
                    }

                    Property::ExDate(_) => {
                        return Err(String::from(
                            "Event occurrence override does not expect an EXDATE property",
                        ));
                    }

                    Property::DTStart(dtstart_property) => {
                        new_override.dtstart = Some(dtstart_property);
                    }

                    Property::DTEnd(dtend_property) => {
                        new_override.dtend = Some(dtend_property);
                    }

                    Property::Duration(duration_property) => {
                        new_override.duration = Some(duration_property);
                    }

                    _ => {
                        new_override.passive_properties.insert(parsed_property)?;
                    }
                }
            }

            if new_override.dtstart.is_none() {
                return Err(String::from(
                    "Event occurrence override requires a DTSTART property",
                ));
            }

            Ok(new_override)
        })
    }
}

impl SerializableICalComponent for EventOccurrenceOverride {
    fn serialize_to_ical_set(
        &self,
        preferences: Option<&SerializationPreferences>,
    ) -> BTreeSet<String> {
        let mut serializable_properties: BTreeSet<String> = BTreeSet::new();

        if let Some(dtstart_property) = &self.dtstart {
            serializable_properties.insert(dtstart_property.serialize_to_ical(preferences));
        }

        if let Some(dtend_property) = &self.dtend {
            serializable_properties.insert(dtend_property.serialize_to_ical(preferences));
        }

        if let Some(duration_property) = &self.duration {
            serializable_properties.insert(duration_property.serialize_to_ical(preferences));
        }

        if let Some(geo_property) = &self.indexed_properties.geo {
            serializable_properties.insert(geo_property.serialize_to_ical(preferences));
        }

        if let Some(class_property) = &self.indexed_properties.class {
            serializable_properties.insert(class_property.serialize_to_ical(preferences));
        }

        if let Some(related_to_properties) = &self.indexed_properties.related_to {
            for related_to_property in related_to_properties {
                serializable_properties.insert(related_to_property.serialize_to_ical(preferences));
            }
        }

        if let Some(categories_properties) = &self.indexed_properties.categories {
            for categories_property in categories_properties {
                serializable_properties.insert(categories_property.serialize_to_ical(preferences));
            }
        }

        for passive_property in &self.passive_properties.properties {
            serializable_properties.insert(passive_property.serialize_to_ical(preferences));
        }

        serializable_properties
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::collections::{BTreeSet, HashSet};

    use crate::core::ical::properties::{
        CategoriesProperty, ClassProperty, DescriptionProperty, Property,
    };

    use crate::testing::macros::build_property_from_ical;

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical() {
        let ical_with_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA DTSTART:19700101T000500Z RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_with_rrule),
            Err(String::from(
                "Event occurrence override does not expect an RRULE property"
            ))
        );

        let ical_without_dtstart: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_without_dtstart),
            Err(String::from(
                "Event occurrence override requires a DTSTART property"
            ))
        );

        let ical_without_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA DTSTART:19700101T000500Z CLASS:PRIVATE CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_without_rrule).unwrap(),
            EventOccurrenceOverride {
                indexed_properties: IndexedProperties {
                    geo: None,
                    class: Some(build_property_from_ical!(ClassProperty, "CLASS:PRIVATE")),
                    categories: Some(HashSet::from([build_property_from_ical!(CategoriesProperty, "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,CATEGORY THREE")])),
                    related_to: None,
                },

                passive_properties: PassiveProperties {
                    properties: BTreeSet::from([Property::Description(build_property_from_ical!(DescriptionProperty, "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas\\, NV\\, USA"))]),
                },

                duration: None,
                dtstart: Some(build_property_from_ical!(DTStartProperty, "DTSTART:19700101T000500Z")),
                dtend: None,
            }
        );
    }
}
