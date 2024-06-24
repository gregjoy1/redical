use std::str::FromStr;

use nom::error::context;
use nom::branch::alt;
use nom::combinator::{map, all_consuming, recognize};
use nom::multi::separated_list0;
use nom::sequence::preceded;

mod dtstart;
mod dtend;
mod exdate;
mod rdate;
mod duration;
mod rrule;
mod exrule;

mod categories;
mod location_type;
mod class;
mod geo;
mod related_to;

mod passive;

use crate::grammar::{wsp, wsp_1_1};

pub use dtstart::{DTStartProperty, DTStartPropertyParams};
pub use dtend::{DTEndProperty, DTEndPropertyParams};
pub use exdate::{ExDateProperty, ExDatePropertyParams};
pub use rdate::{RDateProperty, RDatePropertyParams};
pub use duration::{DurationProperty, DurationPropertyParams};
pub use rrule::{RRuleProperty, RRulePropertyParams};
pub use exrule::{ExRuleProperty, ExRulePropertyParams};

pub use categories::{CategoriesProperty, CategoriesPropertyParams};
pub use location_type::{LocationTypeProperty, LocationTypePropertyParams};
pub use class::{ClassProperty, ClassPropertyParams};
pub use geo::{GeoProperty, GeoPropertyParams};
pub use related_to::{RelatedToProperty, RelatedToPropertyParams};

use crate::content_line::ContentLine;

pub use passive::PassiveProperty;

use crate::properties::uid::UIDProperty;
use crate::properties::last_modified::LastModifiedProperty;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserContext, ParserResult, convert_error};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum EventProperty {
    UID(UIDProperty),
    LastModified(LastModifiedProperty),
    DTStart(DTStartProperty),
    DTEnd(DTEndProperty),
    ExDate(ExDateProperty),
    RDate(RDateProperty),
    Duration(DurationProperty),
    RRule(RRuleProperty),
    ExRule(ExRuleProperty),
    Categories(CategoriesProperty),
    LocationType(LocationTypeProperty),
    Class(ClassProperty),
    Geo(GeoProperty),
    RelatedTo(RelatedToProperty),
    Passive(PassiveProperty),
}

impl EventProperty {
    pub fn parser_context_property_lookahead(input: ParserInput) -> ParserResult<ParserInput> {
        // dbg!(&input.len(), &input.extra);
        context(
            "EVENT PARSER CONTEXT",
            preceded(
                wsp_1_1,
                alt((
                    recognize(ContentLine::parse_ical_for_property("UID")),
                    recognize(ContentLine::parse_ical_for_property("LAST-MODIFIED")),
                    recognize(ContentLine::parse_ical_for_property("DTSTART")),
                    recognize(ContentLine::parse_ical_for_property("DTEND")),
                    recognize(ContentLine::parse_ical_for_property("EXDATE")),
                    recognize(ContentLine::parse_ical_for_property("RDATE")),
                    recognize(ContentLine::parse_ical_for_property("DURATION")),
                    recognize(ContentLine::parse_ical_for_property("RRULE")),
                    recognize(ContentLine::parse_ical_for_property("EXRULE")),
                    recognize(ContentLine::parse_ical_for_property("CATEGORIES")),
                    recognize(ContentLine::parse_ical_for_property("LOCATION-TYPE")),
                    recognize(ContentLine::parse_ical_for_property("CLASS")),
                    recognize(ContentLine::parse_ical_for_property("GEO")),
                    recognize(ContentLine::parse_ical_for_property("RELATED-TO")),
                    recognize(PassiveProperty::parse_ical),
                )),
            ),
        )(input)
    }
}

impl ICalendarEntity for EventProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(UIDProperty::parse_ical, Self::UID),
            map(LastModifiedProperty::parse_ical, Self::LastModified),
            map(DTStartProperty::parse_ical, Self::DTStart),
            map(DTEndProperty::parse_ical, Self::DTEnd),
            map(ExDateProperty::parse_ical, Self::ExDate),
            map(RDateProperty::parse_ical, Self::RDate),
            map(DurationProperty::parse_ical, Self::Duration),
            map(RRuleProperty::parse_ical, Self::RRule),
            map(ExRuleProperty::parse_ical, Self::ExRule),
            map(CategoriesProperty::parse_ical, Self::Categories),
            map(LocationTypeProperty::parse_ical, Self::LocationType),
            map(ClassProperty::parse_ical, Self::Class),
            map(GeoProperty::parse_ical, Self::Geo),
            map(RelatedToProperty::parse_ical, Self::RelatedTo),
            map(PassiveProperty::parse_ical, Self::Passive),
        ))(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::UID(property) => property.render_ical(),
            Self::LastModified(property) => property.render_ical(),
            Self::DTStart(property) => property.render_ical(),
            Self::DTEnd(property) => property.render_ical(),
            Self::ExDate(property) => property.render_ical(),
            Self::RDate(property) => property.render_ical(),
            Self::Duration(property) => property.render_ical(),
            Self::RRule(property) => property.render_ical(),
            Self::ExRule(property) => property.render_ical(),
            Self::Categories(property) => property.render_ical(),
            Self::LocationType(property) => property.render_ical(),
            Self::Class(property) => property.render_ical(),
            Self::Geo(property) => property.render_ical(),
            Self::RelatedTo(property) => property.render_ical(),
            Self::Passive(property) => property.render_ical(),
        }
    }
}

impl std::hash::Hash for EventProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl std::str::FromStr for EventProperty {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parser_result = all_consuming(Self::parse_ical)(ParserInput::new_extra(input, ParserContext::Event));

        match parser_result {
            Ok((_remaining, value)) => Ok(value),

            Err(error) => {
                if let nom::Err::Error(error) = error {
                    Err(crate::convert_error(input, error))
                } else {
                    Err(error.to_string())
                }
            }
        }
    }
}

impl ToString for EventProperty {
    fn to_string(&self) -> String {
        self.render_ical()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct EventProperties(pub Vec<EventProperty>);

impl FromStr for EventProperties {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_properties =
            all_consuming(separated_list0(wsp, EventProperty::parse_ical))(ParserInput::new_extra(input, ParserContext::Event));

        match parsed_properties {
            Ok((_remaining, event_properties)) => {
                Ok(EventProperties(event_properties))
            },

            Err(nom::Err::Error(error)) | Err(nom::Err::Failure(error)) => {
                Err(convert_error(input, error))
            },

            Err(error) => {
                Err(error.to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    #[test]
    fn parse_ical_fuzzing_hang_test() {
        /*
        let message: String = std::fs::read_to_string("./tests/fuzz_finds/hangs/id:000005,src:003038,time:3327034,execs:26454896,op:havoc,rep:2").unwrap();
        dbg!(EventProperties::from_str(message.as_str()));
        let message: String = std::fs::read_to_string("./tests/fuzz_finds/hangs/id:000065,src:004524,time:12952877,execs:78555536,op:havoc,rep:2").unwrap();
        dbg!(EventProperties::from_str(message.as_str()));
        */

        /*
        let paths = std::fs::read_dir("./tests/fuzz_finds/hangs/").unwrap();

        for path in paths {
            dbg!(&path);
            let path = path.unwrap().path();

            let message: String = std::fs::read_to_string(&path).unwrap();

            let (done_tx, done_rx) = std::sync::mpsc::channel();

            let handle = std::thread::spawn(move || {
                let _ = EventProperties::from_str(message.as_str());

                done_tx.send(()).expect("Unable to send completion signal");
            });

            match done_rx.recv_timeout(std::time::Duration::from_millis(1000)) {
                Ok(_) => handle.join().expect("Thread panicked"),
                Err(_) => panic!("Thread took too long -- hang file: {}", &path.display()),
            }
        }
        */
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            EventProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::DTStart(
                    DTStartProperty::from_str("DTSTART:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("DTEND:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::DTEnd(
                    DTEndProperty::from_str("DTEND:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("RDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::RDate(
                    RDateProperty::from_str("RDATE:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("EXDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::ExDate(
                    ExDateProperty::from_str("EXDATE:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("DURATION:PT1H0M0S DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Duration(
                    DurationProperty::from_str("DURATION:PT1H0M0S").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::RRule(
                    RRuleProperty::from_str("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::ExRule(
                    ExRuleProperty::from_str("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Categories(
                    CategoriesProperty::from_str("CATEGORIES:APPOINTMENT,EDUCATION").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("LOCATION-TYPE:HOTEL,RESTRAUNT DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::LocationType(
                    LocationTypeProperty::from_str("LOCATION-TYPE:HOTEL,RESTRAUNT").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("CLASS:PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Class(
                    ClassProperty::from_str("CLASS:PRIVATE").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("GEO:37.386013;-122.082932 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Geo(
                    GeoProperty::from_str("GEO:37.386013;-122.082932").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::RelatedTo(
                    RelatedToProperty::from_str("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical(ParserInput::new_extra("DESCRIPTION:Description text TEST:ING DTSTART:19960401T150000Z", ParserContext::Event)),
            (
                " DTSTART:19960401T150000Z",
                EventProperty::Passive(
                    PassiveProperty::from_str("DESCRIPTION:Description text TEST:ING").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("LAST-MODIFIED:19960401T080045Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::LastModified(
                    LastModifiedProperty::from_str("LAST-MODIFIED:19960401T080045Z".into()).unwrap(),
                ),
            ),
        );
    }

    #[test]
    fn parse_ical_error() {
        let mut mapped_err_parser =
            crate::map_err(
                nom::bytes::complete::tag("-"),
                |mut error: crate::ParserError| {
                    error.message = Some(String::from("TESTING SOMETHING"));

                    error
                },
            );

        if let Err(nom::Err::Error(error)) = mapped_err_parser(":".into()) {
            assert_eq!(error.message, Some(String::from("TESTING SOMETHING")));
        } else {
            panic!("Expected map_err to return transformed nom::Err::Error(ParserError).");
        }

        let mut non_mapped_err_parser =
            crate::map_err(
                nom::combinator::cut(nom::bytes::complete::tag("-")),
                |mut error: crate::ParserError| {
                    error.message = Some(String::from("TESTING SOMETHING"));

                    error
                },
            );

        if let Err(nom::Err::Failure(error)) = non_mapped_err_parser(":".into()) {
            assert_eq!(error.message, Some(String::from("parse error Tag")));
        } else {
            panic!("Expected map_err to return non-transformed nom::Err::Error(ParserError).");
        }
    }
}
