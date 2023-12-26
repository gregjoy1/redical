use std::collections::HashMap;

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

use crate::core::parsers::ical_common;
use crate::core::parsers::ical_common::ParserResult;

fn parse_list_values(input: &str) -> ParserResult<&str, &str> {
    alt((ical_common::quoted_string, param_text))(input)
}

fn parse_single_value(input: &str) -> ParserResult<&str, &str> {
    alt((ical_common::quoted_string, param_text))(input)
}

fn look_ahead_property_parser(input: &str) -> ParserResult<&str, &str> {
    alt((
        preceded(ical_common::white_space, tag(")")),
        recognize(tuple((
            ical_common::white_space1,
            alt((tag("AND"), tag("&&"), tag("OR"), tag("||"))),
            ical_common::white_space1,
            tag("X-RELATED-TO"),
            // ical_common::name,
            alt((
                ical_common::colon_delimeter,
                ical_common::semicolon_delimeter,
            )),
        ))),
        ical_common::look_ahead_property_parser,
    ))(input)
}

// paramtext     = *SAFE-CHAR
fn param_text(input: &str) -> ParserResult<&str, &str> {
    ical_common::parse_with_look_ahead_parser(ical_common::param_text, look_ahead_property_parser)(
        input,
    )
}

pub fn values(input: &str) -> ParserResult<&str, Vec<&str>> {
    context("values", separated_list1(char(','), value))(input)
}

// value         = *VALUE-CHAR
fn value(input: &str) -> ParserResult<&str, &str> {
    ical_common::parse_with_look_ahead_parser(ical_common::value, look_ahead_property_parser)(input)
}

#[derive(Debug)]
pub enum ParsedQueryComponent {
    Limit(usize),
    FromDateTime(LowerBoundRangeCondition),
    UntilDateTime(UpperBoundRangeCondition),
    InTimezone(rrule::Tz),
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
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!($property_name)
            )
        )
    };

    ($property_name:tt, $(($param_name:expr, $param_parser:expr)),+ $(,)*) => {
        opt(
            preceded(
                ical_common::semicolon_delimeter,
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
                    ical_common::semicolon_delimeter,
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
                    ical_common::semicolon_delimeter,
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
            tuple((
                ical_common::colon_delimeter,
                ical_common::ParsedValue::parse_timezone,
            )),
        )),
    )(input)
    .map(|(remaining, (_colon_delimeter, parsed_timezone))| {
        let timezone = match parsed_timezone {
            ical_common::ParsedValue::TimeZone(timezone) => timezone,

            _ => rrule::Tz::UTC,
        };

        (remaining, ParsedQueryComponent::InTimezone(timezone))
    })
}

fn parse_limit_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-LIMIT"),
        cut(context(
            "X-LIMIT",
            tuple((ical_common::colon_delimeter, digit1)),
        )),
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

// X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000
// X-FROM;PROP=DTSTART;OP=GTE;TZID=Europe/London;UUID=Event_UUID:19971002T090000
fn parse_from_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-FROM"),
        cut(context(
            "X-FROM",
            tuple((
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-FROM",
                    (
                        "PROP",
                        map(alt((tag("DTSTART"), tag("DTEND"))), |value| {
                            ical_common::ParsedValue::Single(value)
                        })
                    ),
                    (
                        "OP",
                        map(alt((tag("GTE"), tag("GT"))), |value| {
                            ical_common::ParsedValue::Single(value)
                        })
                    ),
                    ("TZID", ical_common::ParsedValue::parse_timezone),
                    (
                        "UUID",
                        ical_common::ParsedValue::parse_single(parse_single_value)
                    ),
                ),
                ical_common::colon_delimeter,
                ical_common::ParsedValue::parse_date_string,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, ical_common::ParsedValue>,
                &str,
                ical_common::ParsedValue,
            ),
        )| {
            let ical_common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let timezone = match parsed_params.get(&"TZID") {
                Some(ical_common::ParsedValue::TimeZone(timezone)) => timezone,
                _ => &rrule::Tz::UTC,
            };

            let datetime_timestamp = parsed_date_string
                .to_date(Some(*timezone), "X-FROM")
                .unwrap_or_else(|error| {
                    panic!(
                        "Parsed date string unable to be converted to timestamp, error: {:#?}",
                        error
                    );
                })
                .timestamp();

            let range_condition_property = match parsed_params.get(&"PROP") {
                Some(ical_common::ParsedValue::Single("DTSTART")) => {
                    RangeConditionProperty::DtStart(datetime_timestamp)
                }
                Some(ical_common::ParsedValue::Single("DTEND")) => {
                    RangeConditionProperty::DtEnd(datetime_timestamp)
                }

                _ => RangeConditionProperty::DtStart(datetime_timestamp),
            };

            let event_uuid = match parsed_params.get(&"UUID") {
                Some(ical_common::ParsedValue::Single(uuid)) => Some(String::from(*uuid)),
                _ => None,
            };

            let lower_bound_range_condition = match parsed_params.get(&"OP") {
                Some(ical_common::ParsedValue::Single("GT")) => {
                    LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uuid)
                }
                Some(ical_common::ParsedValue::Single("GTE")) => {
                    LowerBoundRangeCondition::GreaterEqualThan(range_condition_property, event_uuid)
                }

                _ => LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uuid),
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
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-UNTIL",
                    (
                        "PROP",
                        map(alt((tag("DTSTART"), tag("DTEND"))), |value| {
                            ical_common::ParsedValue::Single(value)
                        })
                    ),
                    (
                        "OP",
                        map(alt((tag("LTE"), tag("LT"))), |value| {
                            ical_common::ParsedValue::Single(value)
                        })
                    ),
                    ("TZID", ical_common::ParsedValue::parse_timezone),
                ),
                ical_common::colon_delimeter,
                ical_common::ParsedValue::parse_date_string,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, ical_common::ParsedValue>,
                &str,
                ical_common::ParsedValue,
            ),
        )| {
            let ical_common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let timezone = match parsed_params.get(&"TZID") {
                Some(ical_common::ParsedValue::TimeZone(timezone)) => timezone,
                _ => &rrule::Tz::UTC,
            };

            let datetime_timestamp = parsed_date_string
                .to_date(Some(*timezone), "X-FROM")
                .unwrap_or_else(|error| {
                    panic!(
                        "Parsed date string unable to be converted to timestamp, error: {:#?}",
                        error
                    );
                })
                .timestamp();

            let range_condition_property = match parsed_params.get(&"PROP") {
                Some(ical_common::ParsedValue::Single("DTSTART")) => {
                    RangeConditionProperty::DtStart(datetime_timestamp)
                }
                Some(ical_common::ParsedValue::Single("DTEND")) => {
                    RangeConditionProperty::DtEnd(datetime_timestamp)
                }

                _ => RangeConditionProperty::DtStart(datetime_timestamp),
            };

            let upper_bound_range_condition = match parsed_params.get(&"OP") {
                Some(ical_common::ParsedValue::Single("LT")) => {
                    UpperBoundRangeCondition::LessThan(range_condition_property)
                }
                Some(ical_common::ParsedValue::Single("LTE")) => {
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
                    ical_common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-CATEGORIES",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                ical_common::ParsedValue::Single(value)
                            })
                        ),
                    ),
                )),
                preceded(
                    ical_common::colon_delimeter,
                    ical_common::ParsedValue::parse_list(parse_list_values),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (parsed_params, parsed_value)): (
            &str,
            (
                Option<HashMap<&str, ical_common::ParsedValue>>,
                ical_common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(ical_common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };
            }

            let ical_common::ParsedValue::List(parsed_categories) = parsed_value else {
                panic!(
                    "Expected categories to be a list of Strings, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_categories: Vec<String> = parsed_categories
                .into_iter()
                .map(|category| String::from(category))
                .collect();

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

// X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID => X-RELATED-TO;OP=AND;RELTYPE=PARENT:PARENT_UUID
fn parse_related_to_query_property_content(
    input: &str,
) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-RELATED-TO"),
        cut(context(
            "X-RELATED-TO",
            tuple((
                opt(preceded(
                    ical_common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-RELATED-TO",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                ical_common::ParsedValue::Single(value)
                            })
                        ),
                        (
                            "RELTYPE",
                            ical_common::ParsedValue::parse_single(parse_single_value)
                        ),
                    ),
                )),
                preceded(
                    ical_common::colon_delimeter,
                    ical_common::ParsedValue::parse_list(parse_list_values),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (parsed_params, parsed_value)): (
            &str,
            (
                Option<HashMap<&str, ical_common::ParsedValue>>,
                ical_common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;
            let mut parsed_reltype = String::from("PARENT");

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(ical_common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };

                parsed_reltype = match parsed_params.get(&"RELTYPE") {
                    Some(ical_common::ParsedValue::Single(reltype)) => String::from(*reltype),

                    _ => String::from("PARENT"),
                };
            };

            let ical_common::ParsedValue::List(parsed_related_to_uuids) = parsed_value else {
                panic!(
                    "Expected related-to UUIDS to be a list of Strings, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_related_to_uuids: Vec<String> = parsed_related_to_uuids
                .into_iter()
                .map(|related_to_uuid| String::from(related_to_uuid))
                .collect();

            (
                remaining,
                ParsedQueryComponent::WhereRelatedTo(
                    parsed_reltype,
                    parsed_related_to_uuids,
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
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-GEO",
                    ("DIST", ical_common::ParsedValue::parse_geo_distance),
                ),
                ical_common::colon_delimeter,
                ical_common::ParsedValue::parse_lat_long,
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, ical_common::ParsedValue>,
                &str,
                ical_common::ParsedValue,
            ),
        )| {
            let parsed_geo_distance = match parsed_params.get(&"DIST") {
                Some(ical_common::ParsedValue::GeoDistance(geo_distance)) => geo_distance.clone(),

                _ => {
                    return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                        input,
                        "expected DIST param to be present",
                        nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                    )))
                }
            };

            let parsed_geo_point = match parsed_value {
                ical_common::ParsedValue::LatLong(latitude, longitude) => {
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
                    ical_common::semicolon_delimeter,
                    build_property_params_value_parser!(
                        "X-CLASS",
                        (
                            "OP",
                            map(alt((tag("AND"), tag("OR"))), |value| {
                                ical_common::ParsedValue::Single(value)
                            })
                        ),
                    ),
                )),
                preceded(
                    ical_common::colon_delimeter,
                    ical_common::ParsedValue::parse_list(
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
                Option<HashMap<&str, ical_common::ParsedValue>>,
                ical_common::ParsedValue,
            ),
        )| {
            // Defaults
            let mut internal_where_operator = WhereOperator::And;

            if let Some(parsed_params) = parsed_params {
                internal_where_operator = match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(ical_common::ParsedValue::Single("OR")) => WhereOperator::Or,

                    _ => WhereOperator::And,
                };
            }

            let ical_common::ParsedValue::List(parsed_classifications) = parsed_value else {
                panic!(
                    "Expected class to be a list of the following: PUBLIC, PRIVATE, and CONFIDENTIAL, received: {:#?}",
                    parsed_value
                );
            };

            let parsed_classification: Vec<String> = parsed_classifications
                .into_iter()
                .map(|class| String::from(class))
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
                ical_common::semicolon_delimeter,
                build_property_params_value_parser!(
                    "X-ORDER-BY",
                    ("GEO", ical_common::ParsedValue::parse_lat_long),
                ),
                ical_common::colon_delimeter,
                map(
                    alt((
                        tag("GEO-DIST-DTSTART"),
                        tag("DTSTART-GEO-DIST"),
                        tag("DTSTART"),
                    )),
                    |value| ical_common::ParsedValue::Single(value),
                ),
            )),
        )),
    )(input)
    .map(
        |(remaining, (_semicolon_delimeter, parsed_params, _colon_delimeter, parsed_value)): (
            &str,
            (
                &str,
                HashMap<&str, ical_common::ParsedValue>,
                &str,
                ical_common::ParsedValue,
            ),
        )| {
            let parsed_geo_point = match parsed_params.get(&"GEO") {
                Some(ical_common::ParsedValue::LatLong(latitude, longitude)) => {
                    Some(GeoPoint::new(*longitude, *latitude))
                }

                _ => None,
            };

            let ordering_condition = match parsed_value {
                ical_common::ParsedValue::Single("DTSTART-GEO-DIST") => {
                    let Some(parsed_geo_point) = parsed_geo_point else {
                        return Err(nom::Err::Error(nom::error::VerboseError::add_context(
                            input,
                            "expected GEO param to be present",
                            nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                        )));
                    };

                    OrderingCondition::DtStartGeoDist(parsed_geo_point)
                }

                ical_common::ParsedValue::Single("GEO-DIST-DTSTART") => {
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
            ical_common::white_space,
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
                related_to_uuids,
                internal_operator,
                _external_operator,
            ) => ParsedQueryComponent::WhereRelatedTo(
                reltype,
                related_to_uuids,
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
        delimited(
            ical_common::white_space,
            char('('),
            ical_common::white_space,
        ),
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
                opt(ical_common::white_space1),
                context(
                    "group subsequent properties",
                    separated_list0(
                        ical_common::white_space1,
                        parse_operator_prefixed_where_query_property_content,
                    ),
                ),
            )),
        )),
        terminated(ical_common::white_space, char(')')),
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
                related_to_uuids,
                internal_operator,
                external_operator,
            ) => (
                where_related_to_uuids_to_where_conditional(
                    reltype,
                    related_to_uuids,
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
// parse_limit_query_property_content
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
            ical_common::white_space1,
            cut(alt((
                parse_timezone_query_property_content,
                parse_limit_query_property_content,
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
                ParsedQueryComponent::Limit(limit) => {
                    query.limit = limit.clone();
                }

                ParsedQueryComponent::FromDateTime(lower_bound_range_condition) => {
                    query.lower_bound_range_condition = Some(lower_bound_range_condition.clone());
                }

                ParsedQueryComponent::UntilDateTime(upper_bound_range_condition) => {
                    query.upper_bound_range_condition = Some(upper_bound_range_condition.clone());
                }

                ParsedQueryComponent::InTimezone(timezone) => {
                    query.in_timezone = timezone.clone();
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
                    related_to_uuids,
                    internal_operator,
                    _external_operator,
                ) => {
                    let Some(mut new_where_conditional) =
                        where_related_to_uuids_to_where_conditional(
                            reltype,
                            related_to_uuids,
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

fn where_related_to_uuids_to_where_conditional(
    reltype: &String,
    related_to_uuids: &Vec<String>,
    operator: &WhereOperator,
) -> Option<WhereConditional> {
    match related_to_uuids.len() {
        0 => None,

        1 => Some(WhereConditional::Property(
            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                reltype.clone(),
                related_to_uuids[0].clone(),
            )),
            None,
        )),

        _ => {
            let mut current_property = WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    reltype.clone(),
                    related_to_uuids[0].clone(),
                )),
                None,
            );

            for related_to_uuid in related_to_uuids[1..].iter() {
                current_property = WhereConditional::Operator(
                    Box::new(current_property),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            reltype.clone(),
                            related_to_uuid.clone(),
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
        let mut test_parser = ical_common::parse_with_look_ahead_parser(
            take_while1(ical_common::is_safe_char),
            recognize(tuple((
                ical_common::white_space,
                tag("OR"),
                ical_common::white_space,
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
    fn test_where_related_to_uuids_to_where_conditional() {
        assert_eq!(
            where_related_to_uuids_to_where_conditional(
                &String::from("PARENT"),
                &vec![],
                &WhereOperator::And,
            ),
            None,
        );

        assert_eq!(
            where_related_to_uuids_to_where_conditional(
                &String::from("PARENT"),
                &vec![String::from("PARENT_UUID_ONE"),],
                &WhereOperator::And,
            ),
            Some(WhereConditional::Property(
                WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                    String::from("PARENT"),
                    String::from("PARENT_UUID_ONE"),
                )),
                None,
            )),
        );

        assert_eq!(
            where_related_to_uuids_to_where_conditional(
                &String::from("PARENT"),
                &vec![
                    String::from("PARENT_UUID_ONE"),
                    String::from("PARENT_UUID_TWO"),
                    String::from("PARENT_UUID_THREE"),
                ],
                &WhereOperator::Or,
            ),
            Some(WhereConditional::Group(
                Box::new(WhereConditional::Operator(
                    Box::new(WhereConditional::Operator(
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UUID_ONE"),
                            )),
                            None,
                        )),
                        Box::new(WhereConditional::Property(
                            WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                                String::from("PARENT"),
                                String::from("PARENT_UUID_TWO"),
                            )),
                            None,
                        )),
                        WhereOperator::Or,
                        None,
                    )),
                    Box::new(WhereConditional::Property(
                        WhereConditionalProperty::RelatedTo(KeyValuePair::new(
                            String::from("PARENT"),
                            String::from("PARENT_UUID_THREE"),
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
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-CATEGORIES;OP=OR:CATEGORY_ONE,CATEGORY_TWO",
            "X-RELATED-TO:PARENT_UUID",
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
                                        String::from("PARENT_UUID"),
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
                    },),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(
                        RangeConditionProperty::DtStart(875779200,),
                        Some(String::from("Event_UUID")),
                    )),

                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(
                        RangeConditionProperty::DtStart(878461200,),
                    )),

                    in_timezone: rrule::Tz::Europe__Vilnius,

                    limit: 50,
                }
            ))
        );
    }

    #[test]
    fn test_parse_query_string_with_grouped_conditionals() {
        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "(",
            "(",
            "X-GEO;DIST=1.5KM:48.85299;2.36885",
            "OR",
            "X-CATEGORIES:CATEGORY_ONE",
            "OR",
            "X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID",
            ")",
            "AND",
            "(",
            "X-CATEGORIES:CATEGORY_TWO",
            "OR",
            "X-RELATED-TO;RELTYPE=CHILD:CHILD_UUID",
            ")",
            ")",
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
                                            String::from("PARENT_UUID"),
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
                                            String::from("CHILD_UUID"),
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
                    },),

                    lower_bound_range_condition: Some(LowerBoundRangeCondition::GreaterThan(
                        RangeConditionProperty::DtStart(875779200,),
                        Some(String::from("Event_UUID")),
                    )),

                    upper_bound_range_condition: Some(UpperBoundRangeCondition::LessEqualThan(
                        RangeConditionProperty::DtStart(878461200,),
                    )),

                    in_timezone: rrule::Tz::Europe__Vilnius,

                    limit: 50,
                }
            ))
        );
    }
}
