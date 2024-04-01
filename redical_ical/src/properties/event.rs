use nom::branch::alt;
use nom::combinator::map;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::uid::UIDProperty;
use crate::properties::schedule::ScheduleProperty;
use crate::properties::indexed::IndexedProperty;
use crate::properties::passive::PassiveProperty;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum EventProperty {
    UID(UIDProperty),
    Schedule(ScheduleProperty),
    Indexed(IndexedProperty),
    Passive(PassiveProperty),
}

impl ICalendarEntity for EventProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(UIDProperty::parse_ical, Self::UID),
            map(ScheduleProperty::parse_ical, Self::Schedule),
            map(IndexedProperty::parse_ical, Self::Indexed),
            map(PassiveProperty::parse_ical, Self::Passive),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::UID(property) => property.render_ical(),
            Self::Schedule(property) => property.render_ical(),
            Self::Indexed(property) => property.render_ical(),
            Self::Passive(property) => property.render_ical(),
        }
    }
}

impl_icalendar_entity_traits!(EventProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    use crate::properties::schedule::dtstart::DTStartProperty;
    use crate::properties::indexed::categories::CategoriesProperty;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            EventProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Schedule(
                    ScheduleProperty::DTStart(
                        DTStartProperty::from_str("DTSTART:19960401T150000Z").unwrap()
                    )
                ),
            ),
        );

        assert_parser_output!(
            EventProperty::parse_ical("CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                EventProperty::Indexed(
                    IndexedProperty::Categories(
                        CategoriesProperty::from_str("CATEGORIES:APPOINTMENT,EDUCATION").unwrap()
                    )
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
