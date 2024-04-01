pub mod dtstart;
pub mod dtend;
pub mod exdate;
pub mod rdate;
pub mod duration;
pub mod rrule;
pub mod exrule;

use nom::branch::alt;
use nom::combinator::map;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use dtstart::DTStartProperty;
use dtend::DTEndProperty;
use exdate::ExDateProperty;
use rdate::RDateProperty;
use duration::DurationProperty;
use rrule::RRuleProperty;
use exrule::ExRuleProperty;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ScheduleProperty {
    DTStart(DTStartProperty),
    DTEnd(DTEndProperty),
    ExDate(ExDateProperty),
    RDate(RDateProperty),
    Duration(DurationProperty),
    RRule(RRuleProperty),
    ExRule(ExRuleProperty),
}

impl ICalendarEntity for ScheduleProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(DTStartProperty::parse_ical, Self::DTStart),
            map(DTEndProperty::parse_ical, Self::DTEnd),
            map(ExDateProperty::parse_ical, Self::ExDate),
            map(RDateProperty::parse_ical, Self::RDate),
            map(DurationProperty::parse_ical, Self::Duration),
            map(RRuleProperty::parse_ical, Self::RRule),
            map(ExRuleProperty::parse_ical, Self::ExRule),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::DTStart(property) => property.render_ical(),
            Self::DTEnd(property) => property.render_ical(),
            Self::ExDate(property) => property.render_ical(),
            Self::RDate(property) => property.render_ical(),
            Self::Duration(property) => property.render_ical(),
            Self::RRule(property) => property.render_ical(),
            Self::ExRule(property) => property.render_ical(),
        }
    }
}

impl_icalendar_entity_traits!(ScheduleProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ScheduleProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::DTStart(
                    DTStartProperty::from_str("DTSTART:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("DTEND:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::DTEnd(
                    DTEndProperty::from_str("DTEND:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("RDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::RDate(
                    RDateProperty::from_str("RDATE:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("EXDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::ExDate(
                    ExDateProperty::from_str("EXDATE:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("DURATION:PT1H0M0S DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::Duration(
                    DurationProperty::from_str("DURATION:PT1H0M0S").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::RRule(
                    RRuleProperty::from_str("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            ScheduleProperty::parse_ical("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ScheduleProperty::ExRule(
                    ExRuleProperty::from_str("EXRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30").unwrap()
                ),
            ),
        );
    }
}
