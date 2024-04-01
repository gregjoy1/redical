use nom::branch::alt;
use nom::combinator::map;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::schedule::dtstart::DTStartProperty;
use crate::properties::schedule::dtend::DTEndProperty;
use crate::properties::schedule::duration::DurationProperty;

use crate::properties::indexed::IndexedProperty;
use crate::properties::passive::PassiveProperty;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum EventOccurrenceOverrideProperty {
    DTStart(DTStartProperty),
    DTEnd(DTEndProperty),
    Duration(DurationProperty),
    Indexed(IndexedProperty),
    Passive(PassiveProperty),
}

impl ICalendarEntity for EventOccurrenceOverrideProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(DTStartProperty::parse_ical, Self::DTStart),
            map(DTEndProperty::parse_ical, Self::DTEnd),
            map(DurationProperty::parse_ical, Self::Duration),
            map(IndexedProperty::parse_ical, Self::Indexed),
            map(PassiveProperty::parse_ical, Self::Passive),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::DTStart(property) => property.render_ical(),
            Self::DTEnd(property) => property.render_ical(),
            Self::Duration(property) => property.render_ical(),
            Self::Indexed(property) => property.render_ical(),
            Self::Passive(property) => property.render_ical(),
        }
    }
}

impl_icalendar_entity_traits!(EventOccurrenceOverrideProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            EventOccurrenceOverrideProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventOccurrenceOverrideProperty::DTStart(
                    DTStartProperty::from_str("DTSTART:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventOccurrenceOverrideProperty::parse_ical("DTEND:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventOccurrenceOverrideProperty::DTEnd(
                    DTEndProperty::from_str("DTEND:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventOccurrenceOverrideProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventOccurrenceOverrideProperty::DTStart(
                    DTStartProperty::from_str("DTSTART:19960401T150000Z").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventOccurrenceOverrideProperty::parse_ical("DURATION:PT1H0M0S DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventOccurrenceOverrideProperty::Duration(
                    DurationProperty::from_str("DURATION:PT1H0M0S").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            EventOccurrenceOverrideProperty::parse_ical("DESCRIPTION:Description text DTSTART:19960401T150000Z".into()),
            (
                " DTSTART:19960401T150000Z",
                EventOccurrenceOverrideProperty::Passive(
                    PassiveProperty::from_str("DESCRIPTION:Description text").unwrap()
                ),
            ),
        );

        assert!(
            EventOccurrenceOverrideProperty::parse_ical("RRULE:FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=-1MO,SU;BYHOUR=8,9;BYMINUTE=30 DESCRIPTION:Description text".into()).is_err()
        );
    }
}
