use std::str::FromStr;

use nom::error::context;
use nom::combinator::{recognize, eof, opt, not, map, all_consuming};
use nom::sequence::{tuple, preceded};
use nom::multi::{many1, separated_list1};
use nom::branch::alt;

pub mod x_offset;
pub mod x_limit;
pub mod x_distinct;
pub mod x_from;
pub mod x_until;
pub mod x_tzid;
pub mod x_order_by;
pub mod x_categories;
pub mod x_location_type;
pub mod x_related_to;
pub mod x_geo;
pub mod x_class;
pub mod x_uid;
pub mod where_properties_group;


use crate::grammar::{tag, wsp, wsp_1_1, contentline};

pub use x_offset::XOffsetProperty;
pub use x_limit::XLimitProperty;
pub use x_distinct::XDistinctProperty;
pub use x_from::{XFromProperty, XFromPropertyParams};
pub use x_until::{XUntilProperty, XUntilPropertyParams};
pub use x_tzid::XTzidProperty;
pub use x_order_by::XOrderByProperty;
pub use x_categories::{XCategoriesProperty, XCategoriesPropertyParams};
pub use x_location_type::{XLocationTypeProperty, XLocationTypePropertyParams};
pub use x_related_to::{XRelatedToProperty, XRelatedToPropertyParams};
pub use x_geo::{DistValue, XGeoProperty, XGeoPropertyParams};
pub use x_class::{XClassProperty, XClassPropertyParams};
pub use x_uid::{XUIDProperty, XUIDPropertyParams};
pub use where_properties_group::{WherePropertiesGroup, GroupedWhereProperty};

use crate::values::where_operator::WhereOperator;
use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserContext, convert_error};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum QueryProperty {
    XOffset(XOffsetProperty),
    XLimit(XLimitProperty),
    XDistinct(XDistinctProperty),
    XFrom(XFromProperty),
    XUntil(XUntilProperty),
    XTzid(XTzidProperty),
    XOrderBy(XOrderByProperty),
    XUID(XUIDProperty),
    XCategories(XCategoriesProperty),
    XLocationType(XLocationTypeProperty),
    XRelatedTo(XRelatedToProperty),
    XGeo(XGeoProperty),
    XClass(XClassProperty),
    WherePropertiesGroup(WherePropertiesGroup),
}

impl QueryProperty {
    pub fn parser_context_property_lookahead(input: ParserInput) -> ParserResult<ParserInput> {
        context(
            "QUERY PARSER CONTEXT",
            recognize(
                preceded(
                    opt(wsp_1_1),
                    alt((
                        // TODO: HACK HACK HACK HACK - tidy and consolidate
                        recognize(tuple((WhereOperator::parse_ical, opt(wsp), tag("(")))),
                        recognize(tuple((opt(wsp), tag("("), opt(wsp), GroupedWhereProperty::parse_ical))),
                        recognize(tuple((not(contentline), many1(tag(")")), alt((wsp, eof))))),
                        recognize(GroupedWhereProperty::parse_ical),
                        recognize(QueryProperty::parse_ical),
                    )),
                )
            ),
        )(input)
    }
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
            map(XLocationTypeProperty::parse_ical, Self::XLocationType),
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
            Self::XUID(property) => property.render_ical(),
            Self::XLocationType(property) => property.render_ical(),
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

impl std::str::FromStr for QueryProperty {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parser_result = all_consuming(Self::parse_ical)(ParserInput::new_extra(input, ParserContext::Query));

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

impl ToString for QueryProperty {
    fn to_string(&self) -> String {
        self.render_ical()
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct QueryProperties(pub Vec<QueryProperty>);

impl ICalendarEntity for QueryProperties {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(separated_list1(wsp, QueryProperty::parse_ical), QueryProperties)(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.0
            .iter()
            .map(|property| property.render_ical_with_context(context))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl FromStr for QueryProperties {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_properties =
            all_consuming(Self::parse_ical)(ParserInput::new_extra(input, ParserContext::Query));

        match parsed_properties {
            Ok((_remaining, query_properties)) => {
                Ok(query_properties)
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
    fn parse_ical() {
        assert_parser_output!(
            QueryProperty::parse_ical("() DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::WherePropertiesGroup(WherePropertiesGroup::from_str("()").unwrap()),
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
            QueryProperty::parse_ical("X-LOCATION-TYPE:HOTEL,RESTAURANT DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                QueryProperty::XLocationType(XLocationTypeProperty::from_str("X-LOCATION-TYPE:HOTEL,RESTAURANT").unwrap()),
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
