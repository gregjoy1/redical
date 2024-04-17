use std::str::FromStr;

use nom::branch::alt;
use nom::combinator::map;
use nom::combinator::all_consuming;
use nom::multi::separated_list1;

pub mod x_offset;
pub mod x_limit;
pub mod x_distinct;
pub mod x_from;
pub mod x_until;
pub mod x_tzid;
pub mod x_order_by;
pub mod x_categories;
pub mod x_related_to;
pub mod x_geo;
pub mod x_class;
pub mod where_properties_group;


use crate::grammar::wsp;

pub use x_offset::XOffsetProperty;
pub use x_limit::XLimitProperty;
pub use x_distinct::XDistinctProperty;
pub use x_from::{FromRangeOperator, XFromProperty, XFromPropertyParams};
pub use x_until::{UntilRangeOperator, XUntilProperty, XUntilPropertyParams};
pub use x_tzid::XTzidProperty;
pub use x_order_by::XOrderByProperty;
pub use x_categories::{XCategoriesProperty, XCategoriesPropertyParams};
pub use x_related_to::{XRelatedToProperty, XRelatedToPropertyParams};
pub use x_geo::{DistValue, XGeoProperty, XGeoPropertyParams};
pub use x_class::{XClassProperty, XClassPropertyParams};
pub use where_properties_group::{WherePropertiesGroup, GroupedWhereProperty};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, convert_error};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum QueryProperty {
    XOffset(XOffsetProperty),
    XLimit(XLimitProperty),
    XDistinct(XDistinctProperty),
    XFrom(XFromProperty),
    XUntil(XUntilProperty),
    XTzid(XTzidProperty),
    XOrderBy(XOrderByProperty),
    XCategories(XCategoriesProperty),
    XRelatedTo(XRelatedToProperty),
    XGeo(XGeoProperty),
    XClass(XClassProperty),
    WherePropertiesGroup(WherePropertiesGroup),
}

impl ICalendarEntity for QueryProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        alt((
            map(XOffsetProperty::parse_ical, Self::XOffset),
            map(XLimitProperty::parse_ical, Self::XLimit),
            map(XDistinctProperty::parse_ical, Self::XDistinct),
            map(XFromProperty::parse_ical, Self::XFrom),
            map(XUntilProperty::parse_ical, Self::XUntil),
            map(XTzidProperty::parse_ical, Self::XTzid),
            map(XOrderByProperty::parse_ical, Self::XOrderBy),
            map(XCategoriesProperty::parse_ical, Self::XCategories),
            map(XRelatedToProperty::parse_ical, Self::XRelatedTo),
            map(XGeoProperty::parse_ical, Self::XGeo),
            map(XClassProperty::parse_ical, Self::XClass),
            map(WherePropertiesGroup::parse_ical, Self::WherePropertiesGroup),
        ))(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::XOffset(property) => property.render_ical(),
            Self::XLimit(property) => property.render_ical(),
            Self::XDistinct(property) => property.render_ical(),
            Self::XFrom(property) => property.render_ical(),
            Self::XUntil(property) => property.render_ical(),
            Self::XTzid(property) => property.render_ical(),
            Self::XOrderBy(property) => property.render_ical(),
            Self::XCategories(property) => property.render_ical(),
            Self::XRelatedTo(property) => property.render_ical(),
            Self::XGeo(property) => property.render_ical(),
            Self::XClass(property) => property.render_ical(),
            Self::WherePropertiesGroup(property) => property.render_ical(),
        }
    }
}

impl std::hash::Hash for QueryProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(QueryProperty);

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct QueryProperties(pub Vec<QueryProperty>);

impl FromStr for QueryProperties {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_properties =
            all_consuming(separated_list1(wsp, QueryProperty::parse_ical))(input.into());

        match parsed_properties {
            Ok((_remaining, properties)) => {
                Ok(QueryProperties(properties))
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
            WherePropertiesGroup::parse_ical("() DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                WherePropertiesGroup::from_str("()").unwrap(),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-OFFSET:50 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XOffset(XOffsetProperty::from_str("X-OFFSET:50").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-CATEGORIES:APPOINTMENT,EDUCATION DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XCategories(XCategoriesProperty::from_str("X-CATEGORIES:APPOINTMENT,EDUCATION").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-TZID:Europe/London DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XTzid(XTzidProperty::from_str("X-TZID:Europe/London").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-CLASS:PUBLIC,PRIVATE DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XClass(XClassProperty::from_str("X-CLASS:PUBLIC,PRIVATE").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-ORDER-BY:DTSTART DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XOrderBy(XOrderByProperty::DTStart),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-FROM:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XFrom(XFromProperty::from_str("X-FROM:19960401T150000Z").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-UNTIL:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XUntil(XUntilProperty::from_str("X-UNTIL:19960401T150000Z").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-LIMIT:50 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XLimit(XLimitProperty::from_str("X-LIMIT:50").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-RELATED-TO:parent.uid.one,parent.uid.two DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XRelatedTo(XRelatedToProperty::from_str("X-RELATED-TO:parent.uid.one,parent.uid.two").unwrap()),
            ),
        );

        assert_parser_output!(
            QueryProperty::parse_ical("X-DISTINCT:UID DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XDistinct(XDistinctProperty::UID),
            ),
        );


        assert_parser_output!(
            QueryProperty::parse_ical("X-GEO:48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XGeo(XGeoProperty::from_str("X-GEO:48.85299;2.36885").unwrap()),
            ),
        );
    }
}
