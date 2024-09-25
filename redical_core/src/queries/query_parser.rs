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
                            x_uid_query_property_to_where_conditional(x_uid_property)
                        );
                    }

                    QueryProperty::XLocationType(x_location_type_property) => {
                        query.insert_new_where_conditional(
                            x_location_type_query_property_to_where_conditional(x_location_type_property)
                        );
                    }

                    QueryProperty::XCategories(x_categories_property) => {
                        query.insert_new_where_conditional(
                            x_categories_query_property_to_where_conditional(x_categories_property)
                        );
                    }

                    QueryProperty::XRelatedTo(x_related_to_property) => {
                        query.insert_new_where_conditional(
                            x_related_to_query_property_to_where_conditional(x_related_to_property)
                        );
                    }

                    QueryProperty::XGeo(x_geo_property) => {
                        query.insert_new_where_conditional(
                            x_geo_query_property_to_where_conditional(x_geo_property)
                        );
                    }

                    QueryProperty::XClass(x_class_property) => {
                        query.insert_new_where_conditional(
                            x_class_property_to_where_conditional(x_class_property)
                        );
                    }

                    QueryProperty::WherePropertiesGroup(where_properties_group) => {
                        query.insert_new_where_conditional(
                            where_properties_group_to_where_conditional(where_properties_group)
                        );
                    }
                }

                query
            });

    Ok(query)
}

fn where_properties_group_to_where_conditional(where_properties_group: &WherePropertiesGroup) -> Option<WhereConditional> {
    let mut current_where_conditional: Option<WhereConditional> = None;

    for grouped_where_property in &where_properties_group.properties {
        let (new_where_conditional, external_operator) = match &grouped_where_property {
            GroupedWhereProperty::XLocationType(external_operator, x_location_type_property) => (
                x_location_type_query_property_to_where_conditional(x_location_type_property),
                external_operator,
            ),

            GroupedWhereProperty::XUID(external_operator, x_uid_property) => (
                x_uid_query_property_to_where_conditional(x_uid_property),
                external_operator,
            ),

            GroupedWhereProperty::XCategories(external_operator, x_categories_property) => (
                x_categories_query_property_to_where_conditional(x_categories_property),
                external_operator,
            ),

            GroupedWhereProperty::XRelatedTo(external_operator, x_related_to_property) => (
                x_related_to_query_property_to_where_conditional(x_related_to_property),
                external_operator,
            ),

            GroupedWhereProperty::XGeo(external_operator, x_geo_property) => (
                x_geo_query_property_to_where_conditional(x_geo_property),
                external_operator,
            ),

            GroupedWhereProperty::XClass(external_operator, x_class_property) => (
                x_class_property_to_where_conditional(x_class_property),
                external_operator,
            ),

            GroupedWhereProperty::WherePropertiesGroup(external_operator, nested_where_properties_group) => (
                where_properties_group_to_where_conditional(nested_where_properties_group),
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

fn x_uid_query_property_to_where_conditional(x_uid_property: &XUIDProperty) -> Option<WhereConditional> {
    let uids = x_uid_property.get_uids();

    match uids.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::UID(uids[0].to_owned()),
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::UID(uids[0].to_owned()),
            );

            for uid in uids[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::UID(uid.to_owned()),
                    )),
                    WhereOperator::Or,
                );
            }

            Some(WhereConditional::Group(Box::new(current_property)))
        }
    }
}

fn x_location_type_query_property_to_where_conditional(x_location_type_property: &XLocationTypeProperty) -> Option<WhereConditional> {
    let location_types = x_location_type_property.get_location_types();

    match location_types.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::LocationType(location_types[0].to_owned()),
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::LocationType(location_types[0].to_owned()),
            );

            for location_type in location_types[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::LocationType(location_type.to_owned()),
                    )),
                    x_location_type_property.params.op.to_owned().into(),
                );
            }

            Some(WhereConditional::Group(Box::new(current_property)))
        }
    }
}

fn x_categories_query_property_to_where_conditional(x_categories_property: &XCategoriesProperty) -> Option<WhereConditional> {
    let categories = x_categories_property.get_categories();

    match categories.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::Categories(categories[0].to_owned()),
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::Categories(categories[0].to_owned()),
            );

            for category in categories[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Categories(category.to_owned()),
                    )),
                    x_categories_property.params.op.to_owned().into(),
                );
            }

            Some(WhereConditional::Group(Box::new(current_property)))
        }
    }
}

fn x_related_to_query_property_to_where_conditional(x_related_to_property: &XRelatedToProperty) -> Option<WhereConditional> {
    let reltype = x_related_to_property.get_reltype();
    let related_to_uids = x_related_to_property.get_uids();

    match related_to_uids.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                reltype.to_string(),
                related_to_uids[0].to_owned(),
            )),
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    reltype.to_string(),
                    related_to_uids[0].to_owned(),
                )),
            );

            for related_to_uid in related_to_uids[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            reltype.to_string(),
                            related_to_uid.to_owned(),
                        )),
                    )),
                    x_related_to_property.params.op.to_owned().into(),
                );
            }

            Some(WhereConditional::Group(Box::new(current_property)))
        }
    }
}

fn x_geo_query_property_to_where_conditional(x_geo_property: &XGeoProperty) -> Option<WhereConditional> {
    use redical_ical::properties::query::x_geo::DistValue as XGeoDistValue;

    let x_geo_distance =
        match x_geo_property.params.dist.to_owned() {
            XGeoDistValue::Kilometers(kilometers) => {
                GeoDistance::new_from_kilometers_float(kilometers.into())
            },

            XGeoDistValue::Miles(miles) => {
                GeoDistance::new_from_miles_float(miles.into())
            },
        };

    let Some((latitude, longitude)) = x_geo_property.get_lat_long_pair() else {
        return None;
    };

    Some(WhereConditional::Property(
        WhereConditionalProperty::Geo(x_geo_distance, GeoPoint::from((latitude, longitude))),
    ))
}

fn x_class_property_to_where_conditional(x_class_property: &XClassProperty) -> Option<WhereConditional> {
    let classifications = x_class_property.get_classifications();

    match classifications.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::Class(classifications[0].to_owned()),
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::Class(classifications[0].to_owned()),
            );

            for class in classifications[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Class(class.to_owned()),
                    )),
                    x_class_property.params.op.to_owned().into(),
                );
            }

            Some(WhereConditional::Group(Box::new(current_property)))
        }
    }
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
    fn test_x_class_property_to_where_conditional() {
        assert_eq!(
            x_class_property_to_where_conditional(&build_property_from_ical!(XClassProperty, "X-CLASS:")),
            None,
        );

        assert_eq!(
            x_class_property_to_where_conditional(&build_property_from_ical!(XClassProperty, "X-CLASS:PRIVATE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Class(String::from("PRIVATE")),
            )),
        );

        assert_eq!(
            x_class_property_to_where_conditional(
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
    fn test_x_categories_query_property_to_where_conditional() {
        assert_eq!(
            x_categories_query_property_to_where_conditional(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES:")),
            None,
        );

        assert_eq!(
            x_categories_query_property_to_where_conditional(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES:CATEGORY_ONE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
            )),
        );

        assert_eq!(
            x_categories_query_property_to_where_conditional(&build_property_from_ical!(XCategoriesProperty, "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO,CATEGORY_THREE")),
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
    fn test_x_related_to_query_property_to_where_conditional() {
        assert_eq!(
            x_related_to_query_property_to_where_conditional(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO:")),
            None,
        );

        assert_eq!(
            x_related_to_query_property_to_where_conditional(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO:PARENT_UID_ONE")),
            Some(WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    String::from("PARENT"),
                    String::from("PARENT_UID_ONE"),
                )),
            )),
        );

        assert_eq!(
            x_related_to_query_property_to_where_conditional(&build_property_from_ical!(XRelatedToProperty, "X-RELATED-TO;OP=OR;RELTYPE=CHILD:CHILD_UID_ONE,CHILD_UID_TWO,CHILD_UID_THREE")),
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
            "X-CLASS:PRIVATE",
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
                        Box::new(WhereConditional::Property(
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
            "X-LOCATION-TYPE:ONLINE",
            "OR",
            "X-RELATED-TO;RELTYPE=CHILD:CHILD_UID",
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
                                    Box::new(WhereConditional::Property(
                                        WhereConditionalProperty::LocationType(String::from(
                                            "ONLINE"
                                        )),
                                    )),
                                    Box::new(WhereConditional::Property(
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
