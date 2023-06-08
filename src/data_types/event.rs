use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::data_types::ical_property_parser::{parse_properties, ParsedProperty, ParsedPropertyContent, ParsedValue};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Event<'a> {
    #[serde(borrow)]
    pub properties:  HashMap<&'a str, Vec<ParsedProperty<'a>>>,
    pub categories:  Option<Vec<ParsedProperty<'a>>>,
    pub rrule:       Option<Vec<ParsedProperty<'a>>>,
    pub exrule:      Option<Vec<ParsedProperty<'a>>>,
    pub rdate:       Option<Vec<ParsedProperty<'a>>>,
    pub exdate:      Option<Vec<ParsedProperty<'a>>>,
    pub duration:    Option<Vec<ParsedProperty<'a>>>,
    pub dtstart:     Option<Vec<ParsedProperty<'a>>>,
    pub dtend:       Option<Vec<ParsedProperty<'a>>>,
    pub description: Option<Vec<ParsedProperty<'a>>>,
    pub related_to:  Option<Vec<ParsedProperty<'a>>>,
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
            Ok((remaining, parsed_properties)) => {
                let new_event: &mut Event = &mut Event::new();

                parsed_properties.into_iter()
                    .for_each(|parsed_property: ParsedProperty| {
                        match parsed_property {
                            ParsedProperty::Categories(_)  => { Event::append_to(&mut new_event.categories, parsed_property) },
                            ParsedProperty::RRule(_)       => { Event::append_to(&mut new_event.rrule, parsed_property) },
                            ParsedProperty::ExRule(_)      => { Event::append_to(&mut new_event.exrule, parsed_property) },
                            ParsedProperty::RDate(_)       => { Event::append_to(&mut new_event.rdate, parsed_property) },
                            ParsedProperty::ExDate(_)      => { Event::append_to(&mut new_event.exdate, parsed_property) },
                            ParsedProperty::Duration(_)    => { Event::append_to(&mut new_event.duration, parsed_property) },
                            ParsedProperty::DtStart(_)     => { Event::append_to(&mut new_event.dtstart, parsed_property) },
                            ParsedProperty::DtEnd(_)       => { Event::append_to(&mut new_event.dtend, parsed_property) },
                            ParsedProperty::Description(_) => { Event::append_to(&mut new_event.description, parsed_property) },
                            ParsedProperty::RelatedTo(_)   => { Event::append_to(&mut new_event.related_to, parsed_property) },
                            ParsedProperty::Other(_)       => { Event::append_to(&mut new_event.categories, parsed_property) }
                        }
                    });

                Ok(new_event.clone())
            },
            Err(err) => Err(err.to_string())
        }
    }

    fn append_to(attribute: &mut Option<Vec<ParsedProperty<'a>>>, parsed_property: ParsedProperty<'a>) {
        match attribute {
            Some(properties) => { properties.push(parsed_property) },
            None => { *attribute = Some(vec![parsed_property]) }
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
                        ParsedProperty::Categories(
                            ParsedPropertyContent {
                                name: Some("CATEGORIES"),
                                params: None,
                                value: ParsedValue::List(
                                    vec![
                                    "CATEGORY_ONE",
                                    "CATEGORY_TWO",
                                    "CATEGORY THREE",
                                    ]
                                ),
                                content_line: "CATEGORIES:CATEGORY_ONE,CATEGORY_TWO,\"CATEGORY THREE\""
                            }
                        )
                    ]
                ),
                rrule:       Some(
                    vec![
                        ParsedProperty::RRule(
                            ParsedPropertyContent {
                                name: Some("RRULE"),
                                params: None,
                                value: ParsedValue::Params(
                                    HashMap::from(
                                        [
                                        ("FREQ", vec!["WEEKLY"]),
                                        ("UNTIL", vec!["20211231T183000Z"]),
                                        ("INTERVAL", vec!["1"]),
                                        ("BYDAY", vec!["TU","TH"])
                                        ]
                                    )
                                ),
                                content_line: "RRULE:FREQ=WEEKLY;UNTIL=20211231T183000Z;INTERVAL=1;BYDAY=TU,TH"
                            }
                        )
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
                        ParsedProperty::Description(
                            ParsedPropertyContent {
                                name: Some("DESCRIPTION"),
                                params: Some(
                                    HashMap::from(
                                        [
                                        ("ALTREP", vec!["cid:part1.0001@example.org"]),
                                        ]
                                    )
                                ),
                                value: ParsedValue::Single(
                                    "The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                                ),
                                content_line: "DESCRIPTION;ALTREP=\"cid:part1.0001@example.org\":The Fall'98 Wild Wizards Conference - - Las Vegas, NV, USA"
                            }
                        )
                    ]
                ),
                related_to:  None,
            }
        );
    }
}
