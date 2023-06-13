use std::collections::HashMap;

use rrule::RRuleSet;

use serde::{Serialize, Deserialize};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event<'a> {
    #[serde(borrow)]
    pub properties:  HashMap<&'a str, Vec<String>>,
    pub categories:  Option<Vec<String>>,
    pub rrule:       Option<Vec<String>>,
    pub exrule:      Option<Vec<String>>,
    pub rdate:       Option<Vec<String>>,
    pub exdate:      Option<Vec<String>>,
    pub duration:    Option<Vec<String>>,
    pub dtstart:     Option<Vec<String>>,
    pub dtend:       Option<Vec<String>>,
    pub description: Option<Vec<String>>,
    pub related_to:  Option<Vec<String>>,
}

impl<'a> Event<'a> {
    pub fn new() -> Event<'a> {
        Event {
            properties:  HashMap::new(),
            categories:  None,
            rrule:       None,
            exrule:      None,
            rdate:       None,
            exdate:      None,
            duration:    None,
            dtstart:     None,
            dtend:       None,
            description: None,
            related_to:  None,
        }
    }

    pub fn parse_ical<'de: 'a>(input: &str) -> Result<Event<'a>, String> {
        match parse_properties(input) {
            Ok((_, parsed_properties)) => {
                let new_event: &mut Event = &mut Event::new();

                parsed_properties.into_iter()
                    .for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Categories(content)  => {
                                match content.value {
                                    ParsedValue::List(list) => {
                                        list.iter().for_each(|category| {
                                            Event::append_to(&mut new_event.categories, *category)
                                        });
                                    },
                                    _ => {}
                                }
                            },
                            ParsedProperty::RRule(content)       => { Event::append_to(&mut new_event.rrule, content.content_line) },
                            ParsedProperty::ExRule(content)      => { Event::append_to(&mut new_event.exrule, content.content_line) },
                            ParsedProperty::RDate(content)       => { Event::append_to(&mut new_event.rdate, content.content_line) },
                            ParsedProperty::ExDate(content)      => { Event::append_to(&mut new_event.exdate, content.content_line) },
                            ParsedProperty::Duration(content)    => { Event::append_to(&mut new_event.duration, content.content_line) },
                            ParsedProperty::DtStart(content)     => { Event::append_to(&mut new_event.dtstart, content.content_line) },
                            ParsedProperty::DtEnd(content)       => { Event::append_to(&mut new_event.dtend, content.content_line) },
                            ParsedProperty::Description(content) => { Event::append_to(&mut new_event.description, content.content_line) },
                            ParsedProperty::RelatedTo(content)   => { Event::append_to(&mut new_event.related_to, content.content_line) },
                            ParsedProperty::Other(content)       => { } // TODO
                        }
                    });

                Ok(new_event.clone())
            },
            Err(err) => Err(err.to_string())
        }
    }

    fn append_to(attribute: &mut Option<Vec<String>>, content: &'a str) {
        let content = String::from(content);

        match attribute {
            Some(properties) => { properties.push(content) },
            None => { *attribute = Some(vec![content]) }
        }
    }

    fn build_ical(&self) -> String {
        let mut ical_parts = vec![];

        if self.dtstart.is_some() {
            self.dtstart.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }
        if self.rrule.is_some() {
            self.rrule.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.exrule.is_some() {
            self.exrule.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.rdate.is_some() {
            self.rdate.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        if self.exdate.is_some() {
            self.exdate.clone().unwrap().into_iter().for_each(|content_line| {
                ical_parts.push(content_line);
            });
        }

        ical_parts.join("\n")
    }

    fn validate_rrule(&self) -> bool {
        match self.build_ical().parse::<RRuleSet>() {
            Ok(_) => { true },
            Err(_) => { false }
        }
    }
}

mod test {
    use super::*;

    #[test]
    fn test_parse_ical() {
        let ical: &str = "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\"";

        assert_eq!(
            Event::parse_ical(ical).unwrap(),
            Event {
                properties:  HashMap::from([]),
                categories:  Some(
                    vec![
                        String::from("CATEGORY_ONE"),
                        String::from("CATEGORY_TWO"),
                        String::from("CATEGORY THREE")
                    ]
                ),
                rrule:       Some(
                    vec![
                        String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                    ]
                ),
                exrule:      None,
                rdate:       None,
                exdate:      None,
                duration:    None,
                dtstart:     None,
                dtend:       None,
                description: Some(
                    vec![
                        String::from("DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA")
                    ]
                ),
                related_to:  None,
            }
        );
    }

    #[test]
    fn test_validate_rrule() {
        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH DTSTART:16010101T020000";

        let parsed_event = Event::parse_ical(ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                properties:  HashMap::from([]),
                categories:  None,
                rrule:       Some(
                    vec![
                        String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                    ]
                ),
                exrule:      None,
                rdate:       None,
                exdate:      None,
                duration:    None,
                dtstart:     Some(
                    vec![
                        String::from("DTSTART:16010101T020000")
                    ]
                ),
                dtend:       None,
                description: None,
                related_to:  None,
            }
        );

        assert!(parsed_event.validate_rrule());

        let ical: &str = "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH";

        let parsed_event = Event::parse_ical(ical).unwrap();

        assert_eq!(
            parsed_event,
            Event {
                properties:  HashMap::from([]),
                categories:  None,
                rrule:       Some(
                    vec![
                        String::from("RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH")
                    ]
                ),
                exrule:      None,
                rdate:       None,
                exdate:      None,
                duration:    None,
                dtstart:     None,
                dtend:       None,
                description: None,
                related_to:  None,
            }
        );

        assert_eq!(parsed_event.validate_rrule(), false);
    }
}
