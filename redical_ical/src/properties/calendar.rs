use std::str::FromStr;

use nom::branch::alt;
use nom::combinator::map;
use nom::combinator::all_consuming;
use nom::multi::separated_list1;

use crate::grammar::wsp;

use crate::properties::uid::UIDProperty;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, convert_error};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum CalendarProperty {
    UID(UIDProperty),
}

impl ICalendarEntity for CalendarProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(UIDProperty::parse_ical, Self::UID),
        ))(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::UID(property) => property.render_ical(),
        }
    }
}

impl std::hash::Hash for CalendarProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(CalendarProperty);

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct CalendarProperties(pub Vec<CalendarProperty>);

impl FromStr for CalendarProperties {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_properties =
            all_consuming(separated_list1(wsp, CalendarProperty::parse_ical))(input.into());

        match parsed_properties {
            Ok((_remaining, properties)) => {
                Ok(CalendarProperties(properties))
            },

            Err(error) => {
                if let nom::Err::Error(error) = error {
                    Err(convert_error(input, error))
                } else {
                    Err(error.to_string())
                }
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
    fn parse_ical() {
        assert_parser_output!(
            CalendarProperty::parse_ical("UID:19960401T080045Z-4000F192713-0052@example.com DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                CalendarProperty::UID(
                    UIDProperty::from_str("UID:19960401T080045Z-4000F192713-0052@example.com").unwrap(),
                ),
            ),
        );
    }
}
