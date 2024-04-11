use nom::branch::alt;
use nom::combinator::map;

mod dtstart;
mod dtend;
mod exdate;
mod rdate;
mod duration;
mod rrule;
mod exrule;

mod categories;
mod class;
mod geo;
mod related_to;

mod passive;

pub use dtstart::{DTStartProperty, DTStartPropertyParams};
pub use dtend::{DTEndProperty, DTEndPropertyParams};
pub use exdate::{ExDateProperty, ExDatePropertyParams};
pub use rdate::{RDateProperty, RDatePropertyParams};
pub use duration::{DurationProperty, DurationPropertyParams};
pub use rrule::{RRuleProperty, RRulePropertyParams};
pub use exrule::{ExRuleProperty, ExRulePropertyParams};

pub use categories::{CategoriesProperty, CategoriesPropertyParams};
pub use class::{ClassProperty, ClassPropertyParams};
pub use geo::{GeoProperty, GeoPropertyParams};
pub use related_to::{RelatedToProperty, RelatedToPropertyParams};

pub use passive::PassiveProperty;

use crate::properties::uid::UIDProperty;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum EventProperty {
    UID(UIDProperty),
    DTStart(DTStartProperty),
    DTEnd(DTEndProperty),
    ExDate(ExDateProperty),
    RDate(RDateProperty),
    Duration(DurationProperty),
    RRule(RRuleProperty),
    ExRule(ExRuleProperty),
    Categories(CategoriesProperty),
    Class(ClassProperty),
    Geo(GeoProperty),
    RelatedTo(RelatedToProperty),
    Passive(PassiveProperty),
}

impl ICalendarEntity for EventProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(UIDProperty::parse_ical, Self::UID),
            map(DTStartProperty::parse_ical, Self::DTStart),
            map(DTEndProperty::parse_ical, Self::DTEnd),
            map(ExDateProperty::parse_ical, Self::ExDate),
            map(RDateProperty::parse_ical, Self::RDate),
            map(DurationProperty::parse_ical, Self::Duration),
            map(RRuleProperty::parse_ical, Self::RRule),
            map(ExRuleProperty::parse_ical, Self::ExRule),
            map(CategoriesProperty::parse_ical, Self::Categories),
            map(ClassProperty::parse_ical, Self::Class),
            map(GeoProperty::parse_ical, Self::Geo),
            map(RelatedToProperty::parse_ical, Self::RelatedTo),
            map(PassiveProperty::parse_ical, Self::Passive),
        ))(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::UID(property) => property.render_ical(),
            Self::DTStart(property) => property.render_ical(),
            Self::DTEnd(property) => property.render_ical(),
            Self::ExDate(property) => property.render_ical(),
            Self::RDate(property) => property.render_ical(),
            Self::Duration(property) => property.render_ical(),
            Self::RRule(property) => property.render_ical(),
            Self::ExRule(property) => property.render_ical(),
            Self::Categories(property) => property.render_ical(),
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

impl_icalendar_entity_traits!(EventProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

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
            EventProperty::parse_ical("DESCRIPTION:Description text DTSTART:19960401T150000Z".into()),
            (
                " DTSTART:19960401T150000Z",
                EventProperty::Passive(
                    PassiveProperty::from_str("DESCRIPTION:Description text").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("UID:19960401T080045Z-4000F192713-0052@example.com DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::UID(
                    UIDProperty::from_str("UID:19960401T080045Z-4000F192713-0052@example.com".into()).unwrap(),
                ),
            ),
        );
    }
}
