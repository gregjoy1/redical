use std::collections::HashMap;

use chrono_tz::Tz;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::{char, digit1},
    combinator::{cut, map, opt, recognize},
    error::{context, ContextError, ErrorKind, ParseError},
    multi::{separated_list0, separated_list1},
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
};

use crate::core::queries::indexed_property_filters::{
    WhereConditional, WhereConditionalProperty, WhereOperator,
};
use crate::core::queries::query::Query;
use crate::core::queries::results_ordering::OrderingCondition;
use crate::core::queries::results_range_bounds::{
    LowerBoundRangeCondition, RangeConditionProperty, UpperBoundRangeCondition,
};
use crate::core::{GeoDistance, GeoPoint, KeyValuePair};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;

fn parse_list_values(input: &str) -> ParserResult<&str, &str> {
    alt((common::quoted_string, param_text))(input)
}

fn parse_single_value(input: &str) -> ParserResult<&str, &str> {
    alt((common::quoted_string, param_text))(input)
}

fn look_ahead_property_parser(input: &str) -> ParserResult<&str, &str> {
    alt((
        preceded(common::white_space, tag(")")),
        recognize(tuple((
            common::white_space1,
            alt((tag("AND"), tag("&&"), tag("OR"), tag("||"))),
            common::white_space1,
            tag("X-RELATED-TO"),
            // common::name,
            alt((common::colon_delimeter, common::semicolon_delimeter)),
        ))),
        common::look_ahead_property_parser,
    ))(input)
}

// paramtext     = *SAFE-CHAR
fn param_text(input: &str) -> ParserResult<&str, &str> {
    common::parse_with_look_ahead_parser(common::param_text, look_ahead_property_parser)(input)
}

pub fn values(input: &str) -> ParserResult<&str, Vec<&str>> {
    context("values", separated_list1(char(','), value))(input)
}

// value         = *VALUE-CHAR
fn value(input: &str) -> ParserResult<&str, &str> {
    common::parse_with_look_ahead_parser(common::value, look_ahead_property_parser)(input)
}

#[derive(Debug)]
pub enum ParsedQueryComponent {
    Offset(usize),
    Limit(usize),
    DistinctUID,
    FromDateTime(LowerBoundRangeCondition),
    UntilDateTime(UpperBoundRangeCondition),
    InTimezone(Tz),
    Order(OrderingCondition),
    WhereCategories(Vec<String>, WhereOperator, WhereOperator),
    WhereRelatedTo(String, Vec<String>, WhereOperator, WhereOperator),
    WhereGeo(GeoDistance, GeoPoint, WhereOperator),
    WhereClass(Vec<String>, WhereOperator, WhereOperator),
    WhereGroup(Vec<Self>, WhereOperator),
}

macro_rules! build_property_params_parser {
    ($property_name:tt) => {
        opt(
            preceded(
                common::semicolon_delimeter,
                build_property_params_value_parser!($property_name)
            )
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        opt(
            preceded(
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    $property_name,
                    $(
                        ($param_name, $param_parser),
                    )+
                )
            )
        )
    }
}

macro_rules! build_property_params_value_parser {
    ($property_name:tt, ($param_name:expr, $param_parser:expr)$(,)*) => {
        context(
            concat!($property_name, " params"),
            map(
                separated_list1(
                    common::semicolon_delimeter,
                    context(
                        concat!($property_name, " param"),
                        separated_pair(
                            tag($param_name),
                            char('='),
                            cut($param_parser),
                        ),
                    ),
                ),
                |parsed_params| {
                    parsed_params.into_iter()
                                 .map(|(key, value)| (key, value))
                                 .collect()
                }
            ),
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        context(
            concat!($property_name, " params"),
            map(
                separated_list1(
                    common::semicolon_delimeter,
                    alt(
                        (
                            $(
                                context(
                                    concat!($property_name, " param"),
                                    separated_pair(
                                        tag($param_name),
                                        char('='),
                                        cut($param_parser),
                                    ),
                                ),
                            )+
                        ),
                    ),
                ),
                |parsed_params| {
                    parsed_params.into_iter()
                                 .map(|(key, value)| (key, value))
                                 .collect()
                }
            ),
        )
    }
}
// X-TZID:Europe/London
fn parse_timezone_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-TZID"),
        cut(context(
            "X-TZID",
            tuple((common::colon_delimeter, common::ParsedValue::parse_timezone)),
        )),
    )(input)
    .map(|(remaining, (_colon_delimeter, parsed_value))| {
        let parsed_timezone = parsed_value.expect_timezone();

        (remaining, ParsedQueryComponent::InTimezone(parsed_timezone))
    })
}

// X-LIMIT:50
fn parse_limit_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-LIMIT"),
        cut(context("X-LIMIT", tuple((common::colon_delimeter, digit1)))),
    )(input)
    .map(|(remaining, (_colon_delimeter, parsed_value))| {
        let Ok(limit) = str::parse(parsed_value) else {
            return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                parsed_value,
                "parsed limit digit value",
                nom::error::VerboseError::from_error_kind(input, ErrorKind::Digit),
            )));
        };

        Ok((remaining, ParsedQueryComponent::Limit(limit)))
    })?
}

// X-OFFSET:50
fn parse_offset_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-OFFSET"),
        cut(context(
            "X-OFFSET",
            tuple((common::colon_delimeter, digit1)),
        )),
    )(input)
    .map(|(remaining, (_colon_delimeter, parsed_value))| {
        let Ok(offset) = str::parse(parsed_value) else {
            return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                parsed_value,
                "parsed offset digit value",
                nom::error::VerboseError::from_error_kind(input, ErrorKind::Digit),
            )));
        };

        Ok((remaining, ParsedQueryComponent::Offset(offset)))
    })?
}

// X-DISTINCT:UID
fn parse_distinct_uid_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-DISTINCT"),
        cut(context(
            "X-DISTINCT",
            tuple((common::colon_delimeter, tag("UID"))),
        )),
    )(input)
    .map(|(remaining, (_colon_delimeter, _parsed_value))| {
        Ok((remaining, ParsedQueryComponent::DistinctUID))
    })?
}

// X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UID=Event_UID:19971002T090000
// X-FROM;PROP=DTSTART;OP=GTE;TZID=Europe/London;UID=Event_UID:19971002T090000
fn parse_from_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-FROM"),
        cut(context(
            "X-FROM",
            tuple((
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-FROM",
                    (
                        "PROP",
                        map(alt((tag("DTSTART"), tag("DTEND"))), |value| {
                            common::ParsedValue::Single(value)
                        })
                    ),
                    (
                        "OP",
                        map(alt((tag("GTE"), tag("GT"))), |value| {
                            common::ParsedValue::Single(value)
                        })
                    ),
                    ("TZID", common::ParsedValue::parse_timezone),
                    ("UID", common::ParsedValue::parse_single(parse_single_value)),
                ),
                common::colon_delimeter,
                common::ParsedValue::parse_date_string,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, common::ParsedValue>,
                &str,
                common::ParsedValue,
            ),
        )| {
            let common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let parsed_timezone = parsed_params
                .get(&"TZID")
                .and_then(|parsed_value| Some(parsed_value.expect_timezone()))
                .unwrap_or(Tz::UTC);

            let datetime_timestamp = parsed_date_string
                .to_date(Some(parsed_timezone.into()), "X-FROM")
                .unwrap_or_else(|error| {
                    panic!(
                        "Parsed date string unable to be converted to timestamp, error: {:#?}",
                        error
                    );
                })
                .timestamp();

            let range_condition_property = match parsed_params.get(&"PROP") {
                Some(common::ParsedValue::Single("DTSTART")) => {
                    RangeConditionProperty::DtStart(datetime_timestamp)
                }
                Some(common::ParsedValue::Single("DTEND")) => {
                    RangeConditionProperty::DtEnd(datetime_timestamp)
                }

                _ => RangeConditionProperty::DtStart(datetime_timestamp),
            };

            let event_uid = match parsed_params.get(&"UID") {
                Some(common::ParsedValue::Single(uid)) => Some(String::from(*uid)),
                _ => None,
            };

            let lower_bound_range_condition = match parsed_params.get(&"OP") {
                Some(common::ParsedValue::Single("GT")) => {
                    LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uid)
                }
                Some(common::ParsedValue::Single("GTE")) => {
                    LowerBoundRangeCondition::GreaterEqualThan(range_condition_property, event_uid)
                }

                _ => LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uid),
            };

            (
                remaining,
                ParsedQueryComponent::FromDateTime(lower_bound_range_condition),
            )
        },
    )
}

// X-UNTIL:19971002T090000Z        => X-UNTIL;PROP=DTSTART;OP=LT;TZID=UTC:19971002T090000
// X-UNTIL;OP=LTE:19971002T090000Z => X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971002T090000
fn parse_until_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-UNTIL"),
        cut(context(
            "X-UNTIL",
            tuple((
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-UNTIL",
                    (
                        "PROP",
                        map(alt((tag("DTSTART"), tag("DTEND"))), |value| {
                            common::ParsedValue::Single(value)
                        })
                    ),
                    (
                        "OP",
                        map(alt((tag("LTE"), tag("LT"))), |value| {
                            common::ParsedValue::Single(value)
                        })
                    ),
                    ("TZID", common::ParsedValue::parse_timezone),
                ),
                common::colon_delimeter,
                common::ParsedValue::parse_date_string,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, common::ParsedValue>,
                &str,
                common::ParsedValue,
            ),
        )| {
            let common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let parsed_timezone = parsed_params
                .get(&"TZID")
                .and_then(|parsed_value| Some(parsed_value.expect_timezone()))
                .unwrap_or(Tz::UTC);

            let datetime_timestamp = parsed_date_string
                .to_date(Some(parsed_timezone.into()), "X-FROM")
                .unwrap_or_else(|error| {
                    panic!(
                        "Parsed date string unable to be converted to timestamp, error: {:#?}",
                        error
                    );
                })
                .timestamp();

            let range_condition_property = match parsed_params.get(&"PROP") {
                Some(common::ParsedValue::Single("DTSTART")) => {
                    RangeConditionProperty::DtStart(datetime_timestamp)
                }
                Some(common::ParsedValue::Single("DTEND")) => {
                    RangeConditionProperty::DtEnd(datetime_timestamp)
                }

                _ => RangeConditionProperty::DtStart(datetime_timestamp),
            };

            let upper_bound_range_condition = match parsed_params.get(&"OP") {
                Some(common::ParsedValue::Single("LT")) => {
                    UpperBoundRangeCondition::LessThan(range_condition_property)
                }
                Some(common::ParsedValue::Single("LTE")) => {
                    UpperBoundRangeCondition::LessEqualThan(range_condition_property)
                }

                _ => UpperBoundRangeCondition::LessThan(range_condition_property),
            };

            (
                remaining,
                ParsedQueryComponent::UntilDateTime(upper_bound_range_condition),
            )
        },
    )
}

// X-CATEGORIES:CATEGORY_ONE,CATEGORY_TWO  => X-CATEGORIES;OP=AND:CATEGORY_ONE,CATEGORY_TWO
fn parse_categories_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-CATEGORIES"),
        cut(context(
            "X-CATEGORIES",
            tuple((
                opt(preceded(
                    common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-CATEGORIES",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                common::ParsedValue::Single(value)
                            })
                        ),
                    ),
                )),
                preceded(
                    common::colon_delimeter,
                    common::ParsedValue::parse_list(parse_list_values),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (parsed_params, parsed_value)): (
            &str,
            (
                Option<HashMap<&str, common::ParsedValue>>,
                common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };
            }

            let common::ParsedValue::List(parsed_categories) = parsed_value else {
                panic!(
                    "Expected categories to be a list of Strings, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_categories: Vec<String> =
                parsed_categories.into_iter().map(String::from).collect();

            (
                remaining,
                ParsedQueryComponent::WhereCategories(
                    parsed_categories,
                    internal_where_operator,
                    WhereOperator::And,
                ),
            )
        },
    )
}

// X-RELATED-TO;RELTYPE=PARENT:PARENT_UID => X-RELATED-TO;OP=AND;RELTYPE=PARENT:PARENT_UID
fn parse_related_to_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-RELATED-TO"),
        cut(context(
            "X-RELATED-TO",
            tuple((
                opt(preceded(
                    common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-RELATED-TO",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                common::ParsedValue::Single(value)
                            })
                        ),
                        (
                            "RELTYPE",
                            common::ParsedValue::parse_single(parse_single_value)
                        ),
                    ),
                )),
                preceded(
                    common::colon_delimeter,
                    common::ParsedValue::parse_list(parse_list_values),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (parsed_params, parsed_value)): (
            &str,
            (
                Option<HashMap<&str, common::ParsedValue>>,
                common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;
            let mut parsed_reltype = String::from("PARENT");

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };

                parsed_reltype = match parsed_params.get(&"RELTYPE") {
                    Some(common::ParsedValue::Single(reltype)) => String::from(*reltype),

                    _ => String::from("PARENT"),
                };
            };

            let common::ParsedValue::List(parsed_related_to_uids) = parsed_value else {
                panic!(
                    "Expected related-to UIDS to be a list of Strings, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_related_to_uids: Vec<String> = parsed_related_to_uids
                .into_iter()
                .map(String::from)
                .collect();

            (
                remaining,
                ParsedQueryComponent::WhereRelatedTo(
                    parsed_reltype,
                    parsed_related_to_uids,
                    internal_where_operator,
                    WhereOperator::And,
                ),
            )
        },
    )
}

// X-GEO;DIST=1.5KM:48.85299;2.36885
// X-GEO;DIST=30MI:48.85299;2.36885
fn parse_geo_distance_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-GEO"),
        cut(context(
            "X-GEO",
            tuple((
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-GEO",
                    ("DIST", common::ParsedValue::parse_geo_distance),
                ),
                common::colon_delimeter,
                common::ParsedValue::parse_lat_long,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, common::ParsedValue>,
                &str,
                common::ParsedValue,
            ),
        )| {
            let parsed_geo_distance = match parsed_params.get(&"DIST") {
                Some(common::ParsedValue::GeoDistance(geo_distance)) => geo_distance.clone(),

                _ => {
                    return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                        input,
                        "expected DIST param to be present",
                        nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                    )))
                }
            };

            let parsed_geo_point = match parsed_value {
                common::ParsedValue::LatLong(latitude, longitude) => {
                    GeoPoint::new(longitude, latitude)
                }

                _ => {
                    return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                        input,
                        "expected latitude and longitude to be present",
                        nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                    )))
                }
            };

            Ok((
                remaining,
                ParsedQueryComponent::WhereGeo(
                    parsed_geo_distance,
                    parsed_geo_point,
                    WhereOperator::And,
                ),
            ))
        },
    )?
}

// X-CLASS:PUBLIC,CONFIDENTIAL  => X-CLASS;OP=AND:PUBLIC,CONFIDENTIAL
fn parse_class_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-CLASS"),
        cut(context(
            "X-CLASS",
            tuple((
                opt(preceded(
                    common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-CLASS",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                common::ParsedValue::Single(value)
                            })
                        ),
                    ),
                )),
                preceded(
                    common::colon_delimeter,
                    common::ParsedValue::parse_list(
                        alt(
                            (
                                tag("PUBLIC"),
                                tag("PRIVATE"),
                                tag("CONFIDENTIAL"),
                            )
                        ),
                    ),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (parsed_params, parsed_value)): (
            &str,
            (
                Option<HashMap<&str, common::ParsedValue>>,
                common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };
            }

            let common::ParsedValue::List(parsed_classifications) = parsed_value else {
                panic!(
                    "Expected class to be a list of the following: PUBLIC, PRIVATE, and CONFIDENTIAL, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_classification: Vec<String> = parsed_classifications
                .into_iter()
                .map(String::from)
                .collect();

            (
                remaining,
                ParsedQueryComponent::WhereClass(
                    parsed_classification,
                    internal_where_operator,
                    WhereOperator::And,
                ),
            )
        },
    )
}

// X-ORDER-BY:DTSTART
// X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST
// X-ORDER-BY;GEO=48.85299;2.36885:GEO-DIST-DTSTART
fn parse_order_to_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-ORDER-BY"),
        cut(context(
            "X-ORDER-BY",
            tuple((
                common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-ORDER-BY",
                    ("GEO", common::ParsedValue::parse_lat_long),
                ),
                common::colon_delimeter,
                map(
                    alt((
                        tag("GEO-DIST-DTSTART"),
                        tag("DTSTART-GEO-DIST"),
                        tag("DTSTART"),
                    )),
                    |value| common::ParsedValue::Single(value),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, common::ParsedValue>,
                &str,
                common::ParsedValue,
            ),
        )| {
            let parsed_geo_point = match parsed_params.get(&"GEO") {
                Some(common::ParsedValue::LatLong(latitude, longitude)) => {
                    Some(GeoPoint::new(*longitude, *latitude))
                }

                _ => None,
            };

            let ordering_condition = match parsed_value {
                common::ParsedValue::Single("DTSTART-GEO-DIST") => {
                    let Some(parsed_geo_point) = parsed_geo_point else {
                        return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                            input,
                            "expected GEO param to be present",
                            nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                        )));
                    };

                    OrderingCondition::DtStartGeoDist(parsed_geo_point)
                }

                common::ParsedValue::Single("GEO-DIST-DTSTART") => {
                    let Some(parsed_geo_point) = parsed_geo_point else {
                        return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                            input,
                            "expected GEO param to be present",
                            nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                        )));
                    };

                    OrderingCondition::GeoDistDtStart(parsed_geo_point)
                }

                _ => OrderingCondition::DtStart,
            };

            Ok((remaining, ParsedQueryComponent::Order(ordering_condition)))
        },
    )?
}

fn parse_operator_prefixed_where_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    tuple((
        terminated(
            alt((tag("AND"), tag("&&"), tag("OR"), tag("||"))),
            common::white_space,
        ),
        context(
            "operator prefix",
            alt((
                parse_categories_query_property_content,
                parse_related_to_query_property_content,
                parse_geo_distance_query_property_content,
                parse_class_query_property_content,
                parse_group_query_property_component,
            )),
        ),
    ))(input)
    .map(|(remaining, (parsed_operator, parsed_query_component))| {
        let parsed_external_where_operator = match parsed_operator {
            "AND" | "&&" => WhereOperator::And,
            "OR" | "||" => WhereOperator::Or,

            _ => panic!(
                "Expected operator to be either 'AND', '&&', 'OR', '||' - received: {:#?}",
                parsed_operator
            ),
        };

        let parsed_where_query_component = match parsed_query_component {
            ParsedQueryComponent::WhereCategories(
                categories,
                internal_operator,
                _external_operator,
            ) => ParsedQueryComponent::WhereCategories(
                categories,
                internal_operator,
                parsed_external_where_operator,
            ),

            ParsedQueryComponent::WhereRelatedTo(
                reltype,
                related_to_uids,
                internal_operator,
                _external_operator,
            ) => ParsedQueryComponent::WhereRelatedTo(
                reltype,
                related_to_uids,
                internal_operator,
                parsed_external_where_operator,
            ),

            ParsedQueryComponent::WhereGroup(parsed_query_properties, _external_operator) => {
                ParsedQueryComponent::WhereGroup(
                    parsed_query_properties,
                    parsed_external_where_operator,
                )
            }

            _ => panic!("Expected where query property."),
        };

        (remaining, parsed_where_query_component)
    })
}

fn parse_group_query_property_component(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    delimited(
        delimited(common::white_space, char('('), common::white_space),
        cut(context(
            "group",
            tuple((
                context(
                    "group initial property",
                    alt((
                        parse_categories_query_property_content,
                        parse_related_to_query_property_content,
                        parse_geo_distance_query_property_content,
                        parse_class_query_property_content,
                        parse_group_query_property_component,
                    )),
                ),
                opt(common::white_space1),
                context(
                    "group subsequent properties",
                    separated_list0(
                        common::white_space1,
                        parse_operator_prefixed_where_query_property_content,
                    ),
                ),
            )),
        )),
        terminated(common::white_space, char(')')),
    )(input)
    .map(
        |(remaining, (initial_parsed_query_property, _seperator, parsed_query_properties))| {
            let mut parsed_query_properties = parsed_query_properties;

            parsed_query_properties.insert(0, initial_parsed_query_property);

            (
                remaining,
                ParsedQueryComponent::WhereGroup(parsed_query_properties, WhereOperator::And),
            )
        },
    )
}

fn where_group_to_where_conditional(
    parsed_query_properties: &Vec<ParsedQueryComponent>,
) -> Option<WhereConditional> {
    let mut current_where_conditional: Option<WhereConditional> = None;

    for query_property in parsed_query_properties {
        let (new_where_conditional, external_operator) = match &query_property {
            ParsedQueryComponent::WhereCategories(
                categories,
                internal_operator,
                external_operator,
            ) => (
                where_categories_to_where_conditional(categories, internal_operator),
                external_operator,
            ),

            ParsedQueryComponent::WhereRelatedTo(
                reltype,
                related_to_uids,
                internal_operator,
                external_operator,
            ) => (
                where_related_to_uids_to_where_conditional(
                    reltype,
                    related_to_uids,
                    internal_operator,
                ),
                external_operator,
            ),

            ParsedQueryComponent::WhereGeo(distance, long_lat, external_operator) => (
                where_geo_distance_to_where_conditional(distance, long_lat),
                external_operator,
            ),

            ParsedQueryComponent::WhereGroup(parsed_query_properties, external_operator) => (
                where_group_to_where_conditional(parsed_query_properties),
                external_operator,
            ),

            _ => panic!("Expected where query property."),
        };

        if let Some(new_where_conditional) = new_where_conditional {
            if let Some(existing_where_conditional) = current_where_conditional {
                current_where_conditional = Some(WhereConditional::Operator(
                    Box::new(existing_where_conditional),
                    Box::new(new_where_conditional),
                    external_operator.clone(),
                    None,
                ))
            } else {
                current_where_conditional = Some(new_where_conditional);
            }
        }
    }

    current_where_conditional.and_then(|where_conditional| {
        Some(WhereConditional::Group(Box::new(where_conditional), None))
    })
}

// parse_timezone_query_property_content
// parse_offset_query_property_content
// parse_limit_query_property_content
// parse_distinct_uid_query_property_content
// parse_from_query_property_content
// parse_until_query_property_content
// parse_categories_query_property_content
// parse_related_to_query_property_content
// parse_geo_distance_query_property_content
// parse_class_query_property_content
// parse_order_to_query_property_content
// parse_group_query_property_component

pub fn parse_query_string(input: &str) -> ParserResult<&str, Query> {
    let (remaining, query_properties) = context(
        "outer parse query string",
        separated_list1(
            common::white_space1,
            cut(alt((
                parse_timezone_query_property_content,
                parse_offset_query_property_content,
                parse_limit_query_property_content,
                parse_distinct_uid_query_property_content,
                parse_from_query_property_content,
                parse_until_query_property_content,
                parse_order_to_query_property_content,
                parse_categories_query_property_content,
                parse_related_to_query_property_content,
                parse_geo_distance_query_property_content,
                parse_class_query_property_content,
                parse_group_query_property_component,
            ))),
        ),
    )(input)?;

    let query = query_properties
        .iter()
        .fold(Query::default(), |mut query, query_property| {
            match query_property {
                ParsedQueryComponent::Offset(offset) => {
                    query.offset = offset.clone();
                }

                ParsedQueryComponent::Limit(limit) => {
                    query.limit = limit.clone();
                }

                ParsedQueryComponent::DistinctUID => {
                    query.distinct_uids = true;
                }

                ParsedQueryComponent::FromDateTime(lower_bound_range_condition) => {
                    query.lower_bound_range_condition = Some(lower_bound_range_condition.clone());
                }

                ParsedQueryComponent::UntilDateTime(upper_bound_range_condition) => {
                    query.upper_bound_range_condition = Some(upper_bound_range_condition.clone());
                }

                ParsedQueryComponent::InTimezone(parsed_timezone) => {
                    query.in_timezone = parsed_timezone.to_owned();
                }

                ParsedQueryComponent::Order(ordering_condition) => {
                    query.ordering_condition = ordering_condition.clone();
                }

                ParsedQueryComponent::WhereCategories(
                    categories,
                    internal_operator,
                    _external_operator,
                ) => {
                    let Some(mut new_where_conditional) =
                        where_categories_to_where_conditional(categories, internal_operator)
                    else {
                        return query;
                    };

                    new_where_conditional =
                        if let Some(current_where_conditional) = query.where_conditional {
                            WhereConditional::Operator(
                                Box::new(current_where_conditional),
                                Box::new(new_where_conditional),
                                WhereOperator::And,
                                None,
                            )
                        } else {
                            new_where_conditional
                        };

                    query.where_conditional = Some(new_where_conditional);
                }

                ParsedQueryComponent::WhereRelatedTo(
                    reltype,
                    related_to_uids,
                    internal_operator,
                    _external_operator,
                ) => {
                    let Some(mut new_where_conditional) =
                        where_related_to_uids_to_where_conditional(
                            reltype,
                            related_to_uids,
                            internal_operator,
                        )
                    else {
                        return query;
                    };

                    new_where_conditional =
                        if let Some(current_where_conditional) = query.where_conditional {
                            WhereConditional::Operator(
                                Box::new(current_where_conditional),
                                Box::new(new_where_conditional),
                                WhereOperator::And,
                                None,
                            )
                        } else {
                            new_where_conditional
                        };

                    query.where_conditional = Some(new_where_conditional);
                }

                ParsedQueryComponent::WhereGeo(distance, long_lat, _external_operator) => {
                    let Some(mut new_where_conditional) =
                        where_geo_distance_to_where_conditional(distance, long_lat)
                    else {
                        return query;
                    };

                    new_where_conditional =
                        if let Some(current_where_conditional) = query.where_conditional {
                            WhereConditional::Operator(
                                Box::new(current_where_conditional),
                                Box::new(new_where_conditional),
                                WhereOperator::And,
                                None,
                            )
                        } else {
                            new_where_conditional
                        };

                    query.where_conditional = Some(new_where_conditional);
                }

                ParsedQueryComponent::WhereClass(
                    classifications,
                    internal_operator,
                    _external_operator,
                ) => {
                    let Some(mut new_where_conditional) =
                        where_class_to_where_conditional(classifications, internal_operator)
                    else {
                        return query;
                    };

                    new_where_conditional =
                        if let Some(current_where_conditional) = query.where_conditional {
                            WhereConditional::Operator(
                                Box::new(current_where_conditional),
                                Box::new(new_where_conditional),
                                WhereOperator::And,
                                None,
                            )
                        } else {
                            new_where_conditional
                        };

                    query.where_conditional = Some(new_where_conditional);
                }

                ParsedQueryComponent::WhereGroup(parsed_query_properties, _external_operator) => {
                    let Some(mut new_where_conditional) =
                        where_group_to_where_conditional(parsed_query_properties)
                    else {
                        return query;
                    };

                    new_where_conditional =
                        if let Some(current_where_conditional) = query.where_conditional {
                            WhereConditional::Operator(
                                Box::new(current_where_conditional),
                                Box::new(new_where_conditional),
                                WhereOperator::And,
                                None,
                            )
                        } else {
                            new_where_conditional
                        };

                    query.where_conditional = Some(new_where_conditional);
                }
            }

            query
        });

    Ok((remaining, query))
}

fn where_categories_to_where_conditional(
    categories: &Vec<String>,
    operator: &WhereOperator,
) -> Option<WhereConditional> {
    match categories.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::Categories(categories[0].clone()),
            None,
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::Categories(categories[0].clone()),
                None,
            );

            for category in categories[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Categories(category.clone()),
                        None,
                    )),
                    operator.clone(),
                    None,
                );
            }

            Some(WhereConditional::Group(Box::new(current_property), None))
        }
    }
}

fn where_related_to_uids_to_where_conditional(
    reltype: &String,
    related_to_uids: &Vec<String>,
    operator: &WhereOperator,
) -> Option<WhereConditional> {
    match related_to_uids.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                reltype.clone(),
                related_to_uids[0].clone(),
            )),
            None,
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    reltype.clone(),
                    related_to_uids[0].clone(),
                )),
                None,
            );

            for related_to_uid in related_to_uids[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            reltype.clone(),
                            related_to_uid.clone(),
                        )),
                        None,
                    )),
                    operator.clone(),
                    None,
                );
            }

            Some(WhereConditional::Group(Box::new(current_property), None))
        }
    }
}

fn where_geo_distance_to_where_conditional(
    distance: &GeoDistance,
    long_lat: &GeoPoint,
) -> Option<WhereConditional> {
    Some(WhereConditional::Property(
        WhereConditionalProperty::Geo(distance.clone(), long_lat.clone()),
        None,
    ))
}

fn where_class_to_where_conditional(
    classifications: &Vec<String>,
    operator: &WhereOperator,
) -> Option<WhereConditional> {
    match classifications.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::Class(classifications[0].clone()),
            None,
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::Class(classifications[0].clone()),
                None,
            );

            for class in classifications[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Class(class.clone()),
                        None,
                    )),
                    operator.clone(),
                    None,
                );
            }

            Some(WhereConditional::Group(Box::new(current_property), None))
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    use nom::bytes::complete::take_while1;
    use nom::combinator::recognize;

    #[test]
    fn test_parse_with_look_ahead_parser() {
        let mut test_parser = common::parse_with_look_ahead_parser(
            take_while1(common::is_safe_char),
            recognize(tuple((
                common::white_space,
                tag("OR"),
                common::white_space,
                tag("X-CATEGORIES:"),
            ))),
        );

        assert_eq!(
            test_parser("Test Category Text ONE OR X-CATEGORIES:Test Category Text TWO"),
            Ok((
                " OR X-CATEGORIES:Test Category Text TWO",
                "Test Category Text ONE",
            ))
        );

        assert_eq!(
            test_parser("Test Category Text ONE"),
            Ok(("", "Test Category Text ONE",))
        );

        assert_eq!(
            test_parser(""),
            Err(nom::Err::Error(nom::error::VerboseError {
                errors: vec![(
                    "",
                    nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::TakeWhile1,),
                ),],
            },))
        );

        assert_eq!(
            test_parser("::: TEST"),
            Err(nom::Err::Error(nom::error::VerboseError {
                errors: vec![(
                    "::: TEST",
                    nom::error::VerboseErrorKind::Nom(nom::error::ErrorKind::TakeWhile1,),
                ),],
            },))
        );
    }

    #[test]
    fn test_where_class_to_where_conditional() {
        assert_eq!(
            where_class_to_where_conditional(&vec![], &WhereOperator::And,),
            None,
        );

        assert_eq!(
            where_class_to_where_conditional(&vec![String::from("PRIVATE"),], &WhereOperator::And,),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Class(String::from("PRIVATE")),
                None,
            )),
        );

        assert_eq!(
            where_class_to_where_conditional(
                &vec![
                    String::from("PUBLIC"),
                    String::from("PRIVATE"),
                    String::from("CONFIDENTIAL"),
                ],
                &WhereOperator::Or,
            ),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Class(String::from("PUBLIC")),
                            None,
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Class(String::from("PRIVATE")),
                            None,
                        )),
                        WhereOperator::Or,
                        None,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Class(String::from("CONFIDENTIAL")),
                        None,
                    )),
                    WhereOperator::Or,
                    None,
                )),
                None,
            )),
        );
    }

    #[test]
    fn test_where_categories_to_where_conditional() {
        assert_eq!(
            where_categories_to_where_conditional(&vec![], &WhereOperator::And,),
            None,
        );

        assert_eq!(
            where_categories_to_where_conditional(
                &vec![String::from("CATEGORY_ONE"),],
                &WhereOperator::And,
            ),
            Some(WhereConditional::Property(
                WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                None,
            )),
        );

        assert_eq!(
            where_categories_to_where_conditional(
                &vec![
                    String::from("CATEGORY_ONE"),
                    String::from("CATEGORY_TWO"),
                    String::from("CATEGORY_THREE"),
                ],
                &WhereOperator::Or,
            ),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_ONE")),
                            None,
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Categories(String::from("CATEGORY_TWO")),
                            None,
                        )),
                        WhereOperator::Or,
                        None,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::Categories(String::from("CATEGORY_THREE")),
                        None,
                    )),
                    WhereOperator::Or,
                    None,
                )),
                None,
            )),
        );
    }

    #[test]
    fn test_where_related_to_uids_to_where_conditional() {
        assert_eq!(
            where_related_to_uids_to_where_conditional(
                &String::from("PARENT"),
                &vec![],
                &WhereOperator::And,
            ),
            None,
        );

        assert_eq!(
            where_related_to_uids_to_where_conditional(
                &String::from("PARENT"),
                &vec![String::from("PARENT_UID_ONE"),],
                &WhereOperator::And,
            ),
            Some(WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    String::from("PARENT"),
                    String::from("PARENT_UID_ONE"),
                )),
                None,
            )),
        );

        assert_eq!(
            where_related_to_uids_to_where_conditional(
                &String::from("PARENT"),
                &vec![
                    String::from("PARENT_UID_ONE"),
                    String::from("PARENT_UID_TWO"),
                    String::from("PARENT_UID_THREE"),
                ],
                &WhereOperator::Or,
            ),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UID_ONE"),
                            )),
                            None,
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UID_TWO"),
                            )),
                            None,
                        )),
                        WhereOperator::Or,
                        None,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("PARENT_UID_THREE"),
                        )),
                        None,
                    )),
                    WhereOperator::Or,
                    None,
                )),
                None,
            )),
        );
    }

    #[test]
    fn test_parse_query_string() {
        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UID=Event_UID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO",
            "X-RELATED-TO:PARENT_UID",
            "X-GEO;DIST=1.5KM:48.85299;2.36885",
            "X-CLASS:PRIVATE",
            "X-LIMIT:50",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST",
        ]
        .join(" ");

        assert_eq!(
            parse_query_string(query_string.as_str()),
            Ok((
                "",
                Query {
                    where_conditional: Some(WhereConditional::Operator(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Operator(
                                Box::new(WhereConditional::Group(
                                    Box::new(WhereConditional::Operator(
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_ONE"
                                            )),
                                            None,
                                        )),
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_TWO"
                                            )),
                                            None,
                                        )),
                                        WhereOperator::Or,
                                        None,
                                    )),
                                    None,
                                )),
                                Box::new(WhereConditional::Property(
                                    WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                        String::from("PARENT"),
                                        String::from("PARENT_UID"),
                                    )),
                                    None,
                                )),
                                WhereOperator::And,
                                None,
                            )),
                            Box::new(WhereConditional::Property(
                                WhereConditionalProperty::Geo(
                                    GeoDistance::new_from_kilometers_float(1.5),
                                    GeoPoint {
                                        long: 2.36885,
                                        lat: 48.85299,
                                    },
                                ),
                                None,
                            )),
                            WhereOperator::And,
                            None,
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::Class(String::from(String::from("PRIVATE"),)),
                            None,
                        )),
                        WhereOperator::And,
                        None,
                    )),

                    ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                        long: 2.36885,
                        lat: 48.85299,
                    }),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(
                        RangeConditionProperty::DtStart(875779200,),
                        Some(String::from("Event_UID")),
                    )),

                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(
                        RangeConditionProperty::DtStart(878461200,),
                    )),

                    in_timezone: chrono_tz::Tz::Europe__Vilnius,

                    distinct_uids: false,

                    offset: 0,
                    limit: 50,
                }
            ))
        );
    }

    #[test]
    fn test_parse_query_string_with_grouped_conditionals() {
        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UID=Event_UID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "(",
            "(",
            "X-GEO;DIST=1.5KM:48.85299;2.36885",
            "OR",
            "X-CATEGORIES:CATEGORY_ONE",
            "OR",
            "X-RELATED-TO;RELTYPE=PARENT:PARENT_UID",
            ")",
            "AND",
            "(",
            "X-CATEGORIES:CATEGORY_TWO",
            "OR",
            "X-RELATED-TO;RELTYPE=CHILD:CHILD_UID",
            ")",
            ")",
            "X-LIMIT:50",
            "X-OFFSET:10",
            "X-DISTINCT:UID",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST",
        ]
        .join(" ");

        assert_eq!(
            parse_query_string(query_string.as_str()),
            Ok((
                "",
                Query {
                    where_conditional: Some(WhereConditional::Group(
                        Box::new(WhereConditional::Operator(
                            Box::new(WhereConditional::Group(
                                Box::new(WhereConditional::Operator(
                                    Box::new(WhereConditional::Operator(
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Geo(
                                                GeoDistance::new_from_kilometers_float(1.5),
                                                GeoPoint {
                                                    long: 2.36885,
                                                    lat: 48.85299,
                                                },
                                            ),
                                            None,
                                        )),
                                        Box::new(WhereConditional::Property(
                                            WhereConditionalProperty::Categories(String::from(
                                                "CATEGORY_ONE"
                                            )),
                                            None,
                                        )),
                                        WhereOperator::Or,
                                        None,
                                    )),
                                    Box::new(WhereConditional::Property(
                                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                            String::from("PARENT"),
                                            String::from("PARENT_UID"),
                                        )),
                                        None
                                    )),
                                    WhereOperator::Or,
                                    None,
                                )),
                                None
                            )),
                            Box::new(WhereConditional::Group(
                                Box::new(WhereConditional::Operator(
                                    Box::new(WhereConditional::Property(
                                        WhereConditionalProperty::Categories(String::from(
                                            "CATEGORY_TWO"
                                        )),
                                        None,
                                    )),
                                    Box::new(WhereConditional::Property(
                                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                            String::from("CHILD"),
                                            String::from("CHILD_UID"),
                                        )),
                                        None,
                                    )),
                                    WhereOperator::Or,
                                    None,
                                )),
                                None
                            )),
                            WhereOperator::And,
                            None,
                        )),
                        None
                    )),

                    ordering_condition: OrderingCondition::DtStartGeoDist(GeoPoint {
                        long: 2.36885,
                        lat: 48.85299,
                    }),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(
                        RangeConditionProperty::DtStart(875779200,),
                        Some(String::from("Event_UID")),
                    )),

                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(
                        RangeConditionProperty::DtStart(878461200,),
                    )),

                    in_timezone: chrono_tz::Tz::Europe__Vilnius,

                    distinct_uids: true,

                    offset: 10,
                    limit: 50,
                }
            ))
        );
    }
}
