use std::collections::{BTreeSet, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::core::parsers::ical_common::ParsedValue;
use crate::core::parsers::ical_properties::{parse_properties, ParsedProperty};

use crate::core::parsers::datetime::{
    datestring_to_date, extract_and_parse_timezone_from_str, extract_datetime_from_str, ParseError,
};
use crate::core::parsers::duration::ParsedDuration;

use crate::core::utils::KeyValuePair;

use crate::core::geo_index::GeoPoint;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverride {
    pub categories: Option<HashSet<String>>,
    pub duration: Option<ParsedDuration>,
    pub geo: Option<GeoPoint>,
    pub class: Option<String>,
    pub dtstart: Option<KeyValuePair>,
    pub dtend: Option<KeyValuePair>,
    pub related_to: Option<HashMap<String, HashSet<String>>>,
    pub properties: Option<BTreeSet<KeyValuePair>>,
}

impl EventOccurrenceOverride {
    pub fn new() -> EventOccurrenceOverride {
        EventOccurrenceOverride {
            properties: None,
            categories: None,
            class: None,
            duration: None,
            geo: None,
            dtstart: None,
            dtend: None,
            related_to: None,
        }
    }

    pub fn get_dtend_timestamp(&self) -> Result<Option<i64>, ParseError> {
        if let Some(datetime) = self.dtend.as_ref() {
            let datetime_str = extract_datetime_from_str(&datetime.to_string())?;
            let timezone = extract_and_parse_timezone_from_str(&datetime.to_string())?;

            return match datestring_to_date(&datetime_str, timezone, "DTEND") {
                Ok(datetime) => Ok(Some(datetime.timestamp())),
                Err(error) => Err(error),
            };
        }

        Ok(None)
    }

    pub fn get_duration(&self, dtstart_timestamp: &i64) -> Result<Option<i64>, ParseError> {
        if let Some(duration) = self.duration.as_ref() {
            return Ok(Some(duration.get_duration_in_seconds()));
        }

        if let Some(dtend_timestamp) = self.get_dtend_timestamp()? {
            return Ok(Some(dtend_timestamp - dtstart_timestamp));
        }

        Ok(None)
    }

    pub fn parse_ical(input: &str) -> Result<EventOccurrenceOverride, String> {
        match parse_properties(input) {
            Ok((_, parsed_properties)) => {
                let new_override: &mut EventOccurrenceOverride =
                    &mut EventOccurrenceOverride::new();

                parsed_properties.into_iter()
                    .try_for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Class(content) => {
                                if let ParsedValue::Single(parsed_classification) = content.value {
                                    new_override.class = Some(String::from(parsed_classification));
                                } else {
                                    return Err(String::from("Expected classification to be single value"));
                                }
                            },

                            ParsedProperty::Categories(content) => {
                                let mut categories: HashSet<String> = HashSet::new();

                                if let ParsedValue::List(list) = content.value {
                                    list.iter().for_each(|category| {
                                        categories.insert(String::from(*category));
                                    });
                                }

                                new_override.categories = Some(categories);
                            },

                            ParsedProperty::RelatedTo(content)   => {
                                // TODO: improve
                                let default_reltype = String::from("PARENT");

                                let reltype: String = match content.params {
                                    Some(params) => {
                                        match params.get(&"RELTYPE") {
                                            Some(value) => {
                                                // TODO: Clean this up...
                                                match value {
                                                    ParsedValue::List(list_values) => {
                                                        if list_values.len() == 1 {
                                                            String::from(list_values[0])
                                                        } else {
                                                            return Err(String::from("Expected related_to RELTYPE to be a single value."))
                                                        }
                                                    },

                                                    ParsedValue::Single(value) => {
                                                        String::from(*value)
                                                    },

                                                    _ => {
                                                        return Err(String::from("Expected related_to RELTYPE to be a single value."))
                                                    }
                                                }
                                            },

                                            None => default_reltype
                                        }
                                    },

                                    None => default_reltype
                                };

                                match content.value {
                                    ParsedValue::List(list) => {
                                        list.iter().for_each(|related_to_uuid| {
                                            match &mut new_override.related_to {
                                                Some(related_to_map) => {
                                                    related_to_map.entry(reltype.clone())
                                                                  .and_modify(|reltype_uuids| { reltype_uuids.insert(String::from(*related_to_uuid)); })
                                                                  .or_insert(HashSet::from([String::from(*related_to_uuid)]));
                                                },

                                                None => {
                                                    new_override.related_to = Some(
                                                        HashMap::from(
                                                            [
                                                                (
                                                                    reltype.clone(),
                                                                    HashSet::from([
                                                                        String::from(*related_to_uuid)
                                                                    ])
                                                                )
                                                            ]
                                                        )
                                                    );
                                                }
                                            }
                                        });
                                    },

                                    _ => {
                                        return Err(String::from("Expected related_to to have list value."));
                                    }
                                };
                            },

                            ParsedProperty::RRule(_)  => { return Err(String::from("Event occurrence override does not expect an rrule property")); },
                            ParsedProperty::ExRule(_) => { return Err(String::from("Event occurrence override does not expect an exrule property")); },
                            ParsedProperty::RDate(_)  => { return Err(String::from("Event occurrence override does not expect an rdate property")); },
                            ParsedProperty::ExDate(_) => { return Err(String::from("Event occurrence override does not expect an exdate property")); },

                            ParsedProperty::DtStart(content)  => { new_override.dtstart     = Some(content.content_line); },
                            ParsedProperty::DtEnd(content)    => { new_override.dtend       = Some(content.content_line); },

                            ParsedProperty::Duration(content) => {
                                if let ParsedValue::Duration(parsed_duration) = content.value {
                                    new_override.duration = Some(parsed_duration);
                                } else {
                                    return Err(String::from("Event occurrence override expected DURATION property to be valid."))
                                }
                            },

                            ParsedProperty::Geo(content) => {
                                if let ParsedValue::LatLong(parsed_latitude, parsed_longitude) = content.value {
                                    let geo_point = GeoPoint::from(
                                        (
                                            parsed_longitude,
                                            parsed_latitude,
                                        )
                                    );

                                    geo_point.validate()?;

                                    new_override.geo = Some(geo_point);
                                } else {
                                    return Err(String::from("Expected latitude, longitude"));
                                }
                            },

                            ParsedProperty::Description(content) | ParsedProperty::Other(content) => {
                                if let Some(properties) = &mut new_override.properties {
                                    properties.insert(content.content_line);
                                } else {
                                    new_override.properties = Some(
                                        BTreeSet::from([
                                            content.content_line
                                        ])
                                    );
                                }
                            }
                        }

                        Ok(())
                    })?;

                Ok(new_override.clone())
            }
            Err(err) => Err(err.to_string()),
        }
    }

    // TODO: pull into DRY util to turn hash into set
    pub fn build_override_related_to_set(&self) -> Option<HashSet<KeyValuePair>> {
        if self.related_to.is_none() {
            return None;
        }

        let mut override_related_to_set = HashSet::<KeyValuePair>::new();

        if let Some(override_related_to_map) = &self.related_to {
            for (reltype, reltype_uuids) in override_related_to_map.iter() {
                for reltype_uuid in reltype_uuids.iter() {
                    override_related_to_set
                        .insert(KeyValuePair::new(reltype.clone(), reltype_uuid.clone()));
                }
            }
        }

        Some(override_related_to_set)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_ical() {
        let ical_with_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_with_rrule),
            Err(String::from(
                "Event occurrence override does not expect an rrule property"
            ))
        );

        let ical_without_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA CLASS:PRIVATE CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_without_rrule).unwrap(),
            EventOccurrenceOverride {
                geo:              None,
                class:            Some(String::from("PRIVATE")),
                properties:       Some(
                    BTreeSet::from([
                        KeyValuePair::new(
                            String::from("DESCRIPTION"),
                            String::from(";ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                        )
                    ])
                ),
                categories:       Some(
                    HashSet::from([
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ])
                ),
                duration:         None,
                dtstart:          None,
                dtend:            None,
                related_to:       None
            }
        );
    }
}
