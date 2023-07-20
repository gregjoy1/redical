use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use chrono::prelude::*;
use chrono::{DateTime, Utc, Months, Days};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct EventOccurrenceOverride<'a> {
    pub categories:  Option<Vec<String>>,
    pub duration:    Option<String>,
    pub dtstart:     Option<String>,
    pub dtend:       Option<String>,
    pub description: Option<String>,
    pub related_to:  Option<Vec<String>>,

    #[serde(borrow)]
    pub properties:  HashMap<&'a str, Vec<String>>
}

impl<'a> EventOccurrenceOverride<'a> {
    pub fn new() -> EventOccurrenceOverride<'a> {
        EventOccurrenceOverride {
            properties:  HashMap::new(),
            categories:  None,
            duration:    None,
            dtstart:     None,
            dtend:       None,
            description: None,
            related_to:  None,
        }
    }

    pub fn parse_ical<'de: 'a>(input: &str) -> Result<EventOccurrenceOverride<'a>, String> {
        match parse_properties(input) {
            Ok((_, parsed_properties)) => {
                let new_override: &mut EventOccurrenceOverride = &mut EventOccurrenceOverride::new();

                parsed_properties.into_iter()
                    .try_for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Categories(content)  => {
                                let mut categories: Vec<String> = vec![];

                                match content.value {
                                    ParsedValue::List(list) => {
                                        list.iter().for_each(|category| {
                                            categories.push(String::from(*category));
                                        });
                                    },
                                    _ => {}
                                };

                                new_override.categories = Some(categories);
                            },
                            ParsedProperty::RRule(_)  => { return Err(String::from("Event occurrence override does not expect an rrule property")); },
                            ParsedProperty::ExRule(_) => { return Err(String::from("Event occurrence override does not expect an exrule property")); },
                            ParsedProperty::RDate(_)  => { return Err(String::from("Event occurrence override does not expect an rdate property")); },
                            ParsedProperty::ExDate(_) => { return Err(String::from("Event occurrence override does not expect an exdate property")); },
                            ParsedProperty::Duration(content)    => { new_override.duration    = Some(String::from(content.content_line)); },
                            ParsedProperty::DtStart(content)     => { new_override.dtstart     = Some(String::from(content.content_line)); },
                            ParsedProperty::DtEnd(content)       => { new_override.dtend       = Some(String::from(content.content_line)); },
                            ParsedProperty::Description(content) => { new_override.description = Some(String::from(content.content_line)); },
                            ParsedProperty::RelatedTo(content)   => { new_override.related_to  = Some(vec![String::from(content.content_line)]); },
                            ParsedProperty::Other(_content)      => { } // TODO
                        }

                        Ok(())
                    })?;

                Ok(new_override.clone())
            },
            Err(err) => Err(err.to_string())
        }
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
            Err(String::from("Event occurrence override does not expect an rrule property"))
        );

        let ical_without_rrule: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            EventOccurrenceOverride::parse_ical(ical_without_rrule).unwrap(),
            EventOccurrenceOverride {
                properties:       HashMap::from([]),
                categories:       Some(
                    vec![
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ]
                ),
                duration:         None,
                dtstart:          None,
                dtend:            None,
                description:      Some(String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")),
                related_to:       None
            }
        );
    }
}
