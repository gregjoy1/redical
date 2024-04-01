pub mod categories;
pub mod class;
pub mod geo;
pub mod related_to;

use nom::branch::alt;
use nom::combinator::map;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use categories::CategoriesProperty;
use class::ClassProperty;
use geo::GeoProperty;
use related_to::RelatedToProperty;

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum IndexedProperty {
    Categories(CategoriesProperty),
    Class(ClassProperty),
    Geo(GeoProperty),
    RelatedTo(RelatedToProperty),
}

impl ICalendarEntity for IndexedProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(CategoriesProperty::parse_ical, Self::Categories),
            map(ClassProperty::parse_ical, Self::Class),
            map(GeoProperty::parse_ical, Self::Geo),
            map(RelatedToProperty::parse_ical, Self::RelatedTo),
        ))(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Categories(property) => property.render_ical(),
            Self::Class(property) => property.render_ical(),
            Self::Geo(property) => property.render_ical(),
            Self::RelatedTo(property) => property.render_ical(),
        }
    }
}

impl_icalendar_entity_traits!(IndexedProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use std::str::FromStr;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            IndexedProperty::parse_ical("CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                IndexedProperty::Categories(
                    CategoriesProperty::from_str("CATEGORIES:APPOINTMENT,EDUCATION").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            IndexedProperty::parse_ical("CLASS:PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                IndexedProperty::Class(
                    ClassProperty::from_str("CLASS:PRIVATE").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            IndexedProperty::parse_ical("GEO:37.386013;-122.082932 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                IndexedProperty::Geo(
                    GeoProperty::from_str("GEO:37.386013;-122.082932").unwrap()
                ),
            ),
        );

        assert_parser_output!(
            IndexedProperty::parse_ical("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                IndexedProperty::RelatedTo(
                    RelatedToProperty::from_str("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com").unwrap()
                ),
            ),
        );
    }
}
