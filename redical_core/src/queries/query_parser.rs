use std::str::FromStr;

use crate::queries::indexed_property_filters::{
    WhereConditional, WhereConditionalProperty, WhereOperator,
};

use crate::geo_index::GeoPoint;
use crate::queries::query::Query;
use crate::queries::results::QueryableEntity;

use crate::{GeoDistance, KeyValuePair};

use redical_ical::properties::ICalendarGeoProperty;

use redical_ical::properties::query::{
    QueryProperty,
    QueryProperties,
    XUIDProperty,
    XDistinctProperty,
    XCategoriesProperty,
    XLocationTypeProperty,
    XRelatedToProperty,
    XGeoProperty,
    XClassProperty,
    WherePropertiesGroup,
    GroupedWhereProperty,
};

pub fn parse_query_string<T: QueryableEntity, Q: Query<T>>(input: &str) -> Result<Q, String> {
    // Just return the default Query (return everything) if passed empty string ("").
    if input.is_empty() {
        return Ok(Q::default());
    }

    let query_properties = QueryProperties::from_str(input)?;

    let query =
        query_properties
            .0
            .iter()
            .fold(Q::default(), |mut query, query_property| {
                match query_property {
                    QueryProperty::XOffset(x_offset_property) => {
                        query.set_offset(x_offset_property.into());
                    }

                    QueryProperty::XLimit(x_limit_property) => {
                        query.set_limit(x_limit_property.into());
                    }

                    QueryProperty::XDistinct(XDistinctProperty::UID) => {
                        query.set_distinct_uids(true);
                    }

                    QueryProperty::XFrom(x_from_property) => {
                        query.set_lower_bound_range_condition(Some(x_from_property.into()));
                    }

                    QueryProperty::XUntil(x_until_property) => {
                        query.set_upper_bound_range_condition(Some(x_until_property.into()));
                    }

                    QueryProperty::XTzid(x_tzid_property) => {
                        query.set_in_timezone(x_tzid_property.into());
                    }

                    QueryProperty::XOrderBy(x_order_by_property) => {
                        query.set_ordering_condition(x_order_by_property.into());
                    }

                    QueryProperty::XUID(x_uid_property) => {
                        query.insert_new_where_conditional(
                            build_uid_property_condition(x_uid_property)
                        );
                    }

                    QueryProperty::XLocationType(x_location_type_property) => {
                        query.insert_new_where_conditional(
                            build_location_type_property_condition(x_location_type_property)
                        );
                    }

                    QueryProperty::XCategories(x_categories_property) => {
                        query.insert_new_where_conditional(
                            build_categories_property_condition(x_categories_property)
                        );
                    }

                    QueryProperty::XRelatedTo(x_related_to_property) => {
                        query.insert_new_where_conditional(
                            build_related_to_property_condition(x_related_to_property)
                        );
                    }

                    QueryProperty::XGeo(x_geo_property) => {
                        query.insert_new_where_conditional(
                            build_geo_property_condition(x_geo_property)
                        );
                    }

                    QueryProperty::XClass(x_class_property) => {
                        query.insert_new_where_conditional(
                            build_class_property_condition(x_class_property)
                        );
                    }

                    QueryProperty::WherePropertiesGroup(where_properties_group) => {
                        query.insert_new_where_conditional(
                            build_grouped_conditional(where_properties_group)
                        );
                    }
                }

                query
            });

    Ok(query)
}

macro_rules! fold_terms {
    ($variant:ident, $terms:expr, $op:expr) => {{
        if $terms.len() == 0 { return None }

        let condition = WhereConditional::Property(
            WhereConditionalProperty::$variant($terms[0].to_owned())
        );

        if $terms.len() == 1 { return Some(condition) }

        let condition = $terms[1..].iter().fold(condition, |last, term|
            WhereConditional::Operator(
                Box::new(last),
                Box::new(WhereConditional::Property(
                    WhereConditionalProperty::$variant(term.to_owned())
                )),
                $op,
            )
        );

        Some(WhereConditional::Group(Box::new(condition)))
    }}
}

macro_rules! fold_negated_terms {
    ($variant:ident, $terms:expr, $op:expr) => {{
        if $terms.len() == 0 { return None }

        let condition = WhereConditional::NegatedProperty(
            WhereConditionalProperty::$variant($terms[0].to_owned())
        );

        if $terms.len() == 1 { return Some(condition) }

        let condition = $terms[1..].iter().fold(condition, |last, term|
            WhereConditional::Operator(
                Box::new(last),
                Box::new(WhereConditional::NegatedProperty(
                    WhereConditionalProperty::$variant(term.to_owned())
                )),
                $op,
            )
        );

        Some(WhereConditional::Group(Box::new(condition)))
    }}
}

// Operator is hardcoded for UIDs due to the mutual exclusivity between separate
// events (ie an event can't have multiple UIDs).
fn build_uid_property_condition(property: &XUIDProperty) -> Option<WhereConditional> {
    if property.negated {
        fold_negated_terms!(
            UID,
            property.get_uids(),
            WhereOperator::And
        )
    } else {
        fold_terms!(
            UID,
            property.get_uids(),
            WhereOperator::Or
        )
    }
}

fn build_location_type_property_condition(property: &XLocationTypeProperty) -> Option<WhereConditional> {
    if property.negated {
        fold_negated_terms!(
            LocationType,
            property.get_location_types(),
            property.params.op.clone().into()
        )
    } else {
        fold_terms!(
            LocationType,
            property.get_location_types(),
            property.params.op.clone().into()
        )
    }
}

fn build_categories_property_condition(property: &XCategoriesProperty) -> Option<WhereConditional> {
    if property.negated {
        fold_negated_terms!(
            Categories,
            property.get_categories(),
            property.params.op.clone().into()
        )
    } else {
        fold_terms!(
            Categories,
            property.get_categories(),
            property.params.op.clone().into()
        )
    }
}

fn build_related_to_property_condition(property: &XRelatedToProperty) -> Option<WhereConditional> {
    let reltype = property.get_reltype();

    let key_values: Vec<_> = property.get_uids()
        .into_iter()
        .map(|uid| KeyValuePair::new(reltype.to_string(), uid))
        .collect();

    if property.negated {
        fold_negated_terms!(
            RelatedTo,
            key_values,
            property.params.op.clone().into()
        )
    } else {
        fold_terms!(
            RelatedTo,
            key_values,
            property.params.op.clone().into()
        )
    }
}

fn build_geo_property_condition(property: &XGeoProperty) -> Option<WhereConditional> {
    use redical_ical::properties::query::x_geo::DistValue as XGeoDistValue;

    let x_geo_distance =
        match property.params.dist.to_owned() {
            XGeoDistValue::Kilometers(kilometers) => {
                GeoDistance::new_from_kilometers_float(kilometers.into())
            },

            XGeoDistValue::Miles(miles) => {
                GeoDistance::new_from_miles_float(miles.into())
            },
        };

    let (latitude, longitude) = property.get_lat_long_pair()?;

    let where_conditional_property = WhereConditionalProperty::Geo(
        x_geo_distance,
        GeoPoint::from((latitude, longitude))
    );

    if property.negated {
        Some(WhereConditional::NegatedProperty(where_conditional_property))
    } else {
        Some(WhereConditional::Property(where_conditional_property))
    }
}

fn build_class_property_condition(property: &XClassProperty) -> Option<WhereConditional> {
    if property.negated {
        fold_negated_terms!(
            Class,
            property.get_classifications(),
            property.params.op.to_owned().into()
        )
    } else {
        fold_terms!(
            Class,
            property.get_classifications(),
            property.params.op.to_owned().into()
        )
    }
}

fn build_grouped_conditional(where_properties_group: &WherePropertiesGroup) -> Option<WhereConditional> {
    let mut current_where_conditional: Option<WhereConditional> = None;

    for grouped_where_property in &where_properties_group.properties {
        let (new_where_conditional, external_operator) = match &grouped_where_property {
            GroupedWhereProperty::XLocationType(external_operator, x_location_type_property) => (
                build_location_type_property_condition(x_location_type_property),
                external_operator,
            ),

            GroupedWhereProperty::XUID(external_operator, x_uid_property) => (
                build_uid_property_condition(x_uid_property),
                external_operator,
            ),

            GroupedWhereProperty::XCategories(external_operator, x_categories_property) => (
                build_categories_property_condition(x_categories_property),
                external_operator,
            ),

            GroupedWhereProperty::XRelatedTo(external_operator, x_related_to_property) => (
                build_related_to_property_condition(x_related_to_property),
                external_operator,
            ),

            GroupedWhereProperty::XGeo(external_operator, x_geo_property) => (
                build_geo_property_condition(x_geo_property),
                external_operator,
            ),

            GroupedWhereProperty::XClass(external_operator, x_class_property) => (
                build_class_property_condition(x_class_property),
                external_operator,
            ),

            GroupedWhereProperty::WherePropertiesGroup(external_operator, nested_where_properties_group) => (
                build_grouped_conditional(nested_where_properties_group),
                external_operator,
            ),
        };

        // Massage Option<[ICalendar]WhereOperator> value type into [Query]WhereOperator -
        // defaulting to And operator is None.
        let external_operator = external_operator.to_owned().map(WhereOperator::from).unwrap_or(WhereOperator::And);

        if let Some(new_where_conditional) = new_where_conditional {
            if let Some(existing_where_conditional) = current_where_conditional {
                current_where_conditional = Some(WhereConditional::Operator(
                    Box::new(existing_where_conditional),
                    Box::new(new_where_conditional),
                    external_operator.clone(),
                ))
            } else {
                current_where_conditional = Some(new_where_conditional);
            }
        }
    }

    current_where_conditional.map(|where_conditional| {
        WhereConditional::Group(Box::new(where_conditional))
    })
}

#[cfg(test)]
mod test {
    use crate::queries::event_instance_query::EventInstanceQuery;

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    use std::str::FromStr;

    use crate::testing::macros::build_property_from_ical;

    use crate::queries::indexed_property_filters::{
        WhereConditional, WhereConditionalProperty, WhereOperator,
    };

    use crate::queries::results_ordering::OrderingCondition;
    use crate::queries::results_range_bounds::{
        LowerBoundRangeCondition, RangeConditionProperty, UpperBoundRangeCondition,
    };

    use crate::{GeoDistance, KeyValuePair};

    #[test]
    fn test_build_class_property_condition_condition() {
        assert_eq!(
            build_class_property_condition(&build_property_from_ical!(XClassProperty, "X-CLASS:")),
            None,
        );

        assert_eq!(
            build_class_property_condition(&build_property_from_ical!(XClassProperty, "X-CLASS:PRIVATE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Class(String::from("PRIVATE")),
            )),
        );

        assert_eq!(
            build_class_property_condition(
                &build_property_from_ical!(XClassProperty, "X-CLASS;OP=OR:PUBLIC,PRIVATE,CONFIDENTIAL")
            ),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Class(String::from("PUBLIC")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Class(String::from("PRIVATE")),
                        )),
                        WhereOperator::Or,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Class(String::from("CONFIDENTIAL")),
                    )),
                    WhereOperator::Or,
                )),
            )),
        );
    }

    #[test]
    fn test_build_categories_property_condition_condition() {
        assert_eq!(
            build_categories_property_condition(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES:")),
            None,
        );

        assert_eq!(
            build_categories_property_condition(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES:CATEGORY_ONE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
            )),
        );

        assert_eq!(
            build_categories_property_condition(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_TWO")),
                        )),
                        WhereOperator::Or,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Categories(String::from("CATEGORY_THREE")),
                    )),
                    WhereOperator::Or,
                )),
            )),
        );
    }

    #[test]
    fn test_build_related_to_property_condition_condition() {
        assert_eq!(
            build_related_to_property_condition(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO:")),
            None,
        );

        assert_eq!(
            build_related_to_property_condition(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO:PARENT_UID_ONE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    String::from("PARENT"),
                    String::from("PARENT_UID_ONE"),
                )),
            )),
        );

        assert_eq!(
            build_related_to_property_condition(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO;OP=OR;RELTYPE=CHILD:CHILD_UID_ONE,CHILD_UID_TWO,CHILD_UID_THREE")),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("CHILD"),
                                String::from("CHILD_UID_ONE"),
                            )),
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("CHILD"),
                                String::from("CHILD_UID_TWO"),
                            )),
                        )),
                        WhereOperator::Or,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            String::from("CHILD"),
                            String::from("CHILD_UID_THREE"),
                        )),
                    )),
                    WhereOperator::Or,
                )),
            )),
        );
    }

    #[test]
    fn test_parse_query_string() {
        assert_eq!(parse_query_string(""), Ok(EventInstanceQuery::default()));

        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO",
            "X-RELATED-TO:PARENT_UID",
            "X-GEO;DIST=1.5KM:48.85299;2.36885",
            "X-CLASS-NOT:PRIVATE",
            "X-LIMIT:50",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885",
        ]
        .join(" ");

        assert_eq!(
            parse_query_string(query_string.as_str()),
            Ok(
                EventInstanceQuery {
                    where_conditional: Some(WhereConditional::Operator(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Operator(
                                Box::new(WhereConditional::Group(
                                    Box::new(WhereConditional::Operator(
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_ONE"
                                            )),
                                        )),
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_TWO"
                                            )),
                                        )),
                                        WhereOperator::Or,
                                    )),
                                )),
                                Box::new(WhereConditional::Property(
                                    WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                        String::from("PARENT"),
                                        String::from("PARENT_UID"),
                                    )),
                                )),
                                WhereOperator::And,
                            )),
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Geo(
                                    GeoDistance::new_from_kilometers_float(1.5),
                                    GeoPoint {
                                        long: 2.36885,
                                        lat: 48.85299,
                                    },
                                ),
                            )),
                            WhereOperator::And,
                        )),
                        Box::new(WhereConditional::NegatedProperty(
                            WhereConditionalProperty::Class(String::from("PRIVATE")),
                        )),
                        WhereOperator::And,
                    )),

                    ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                        long: 2.36885,
                        lat: 48.85299,
                    }),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(RangeConditionProperty::DtStart(875779200))),
                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(RangeConditionProperty::DtStart(878461200))),

                    in_timezone: chrono_tz::Tz::Europe__Vilnius,

                    distinct_uids: false,

                    offset: 0,
                    limit: 50,
                }
            )
        );
    }

    #[test]
    fn test_parse_query_string_with_grouped_conditionals() {
        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "(",
            "(",
            "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
            "OR",
            "X-CATEGORIES:CATEGORY_ONE",
            "OR",
            "X-GEO;DIST=1.5KM:48.85299;2.36885",
            ")",
            "AND",
            "(",
            "X-LOCATION-TYPE-NOT:ONLINE",
            "OR",
            "X-RELATED-TO-NOT;RELTYPE=CHILD:CHILD_UID",
            ")",
            ")",
            "X-LIMIT:50",
            "X-OFFSET:10",
            "X-DISTINCT:UID",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885",
        ]
        .join(" ");

        assert_eq!(
            parse_query_string(query_string.as_str()),
            Ok(
                EventInstanceQuery {
                    where_conditional: Some(WhereConditional::Group(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Group(
                                Box::new(WhereConditional::Operator(
                                    Box::new(WhereConditional::Operator(
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                                String::from("PARENT"),
                                                String::from("PARENT_UID"),
                                            )),
                                        )),
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_ONE"
                                            )),
                                        )),
                                        WhereOperator::Or,
                                    )),
                                    Box::new(WhereConditional::Property(
                                        WhereConditionalProperty::Geo(
                                            GeoDistance::new_from_kilometers_float(1.5),
                                            GeoPoint {
                                                long: 2.36885,
                                                lat: 48.85299,
                                            },
                                        ),
                                    )),
                                    WhereOperator::Or,
                                )),
                            )),
                            Box::new(WhereConditional::Group(
                                Box::new(WhereConditional::Operator(
                                    Box::new(WhereConditional::NegatedProperty(
                                        WhereConditionalProperty::LocationType(String::from(
                                            "ONLINE"
                                        )),
                                    )),
                                    Box::new(WhereConditional::NegatedProperty(
                                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                            String::from("CHILD"),
                                            String::from("CHILD_UID"),
                                        )),
                                    )),
                                    WhereOperator::Or,
                                )),
                            )),
                            WhereOperator::And,
                        )),
                    )),

                    ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                        long: 2.36885,
                        lat: 48.85299,
                    }),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(RangeConditionProperty::DtStart(875779200))),
                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(RangeConditionProperty::DtStart(878461200))),

                    in_timezone: chrono_tz::Tz::Europe__Vilnius,

                    distinct_uids: true,

                    offset: 10,
                    limit: 50,
                }
            )
        );
    }
}
