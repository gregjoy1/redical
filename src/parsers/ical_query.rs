use std::collections::HashMap;

use nom::{
    error::{context, ParseError, ContextError, ErrorKind, VerboseError},
    multi::separated_list1,
    sequence::{preceded, terminated, tuple, separated_pair},
    branch::alt,
    combinator::{cut, opt, map},
    bytes::complete::tag,
    character::complete::{char, digit1},
    number::complete::recognize_float,
};

use crate::data_types::{KeyValuePair, GeoPoint};
use crate::queries::query::Query;
use crate::queries::results_ordering::OrderingCondition;
use crate::queries::results_range_bounds::{LowerBoundRangeCondition, UpperBoundRangeCondition, RangeConditionProperty};
use crate::queries::indexed_property_filters::{WhereOperator, WhereConditional};

use crate::parsers::ical_common;
use crate::parsers::ical_common::ParserResult;

use crate::parsers::datetime::ParsedDateString;

#[derive(Debug)]
pub enum ParsedQueryComponent {
    Limit(usize),
    FromDateTime(LowerBoundRangeCondition),
    UntilDateTime(UpperBoundRangeCondition),
    InTimezone(rrule::Tz),
    Order(OrderingCondition),
    WhereCategories(Vec<String>, WhereOperator),
    WhereRelatedTo(KeyValuePair, WhereOperator),
    WhereGroup(Box<Vec<Self>>),
    WhereAnd(Box<Self>),
    WhereOr(Box<Self>),
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
        cut(
            context(
                "X-TZID",
                tuple(
                    (
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_timezone,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_colon_delimeter, parsed_timezone))| {
            let timezone = match parsed_timezone {
                ical_common::ParsedValue::TimeZone(timezone) => timezone,

                _ => rrule::Tz::UTC,
            };

            (
                remaining,
                ParsedQueryComponent::InTimezone(timezone)
            )
        }
    )
}

fn parse_limit_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-LIMIT"),
        cut(
            context(
                "X-LIMIT",
                tuple(
                    (
                        ical_common::colon_delimeter,
                        digit1,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_colon_delimeter, parsed_value))| {
            let Ok(limit) = str::parse(parsed_value) else {
                return Err(
                    nom::Err::Error(
                        nom::error::VerboseError::add_context(
                            parsed_value,
                            "parsed limit digit value",
                            nom::error::VerboseError::from_error_kind(input, ErrorKind::Digit),
                        )
                    )
                )
            };

            Ok(
                (remaining, ParsedQueryComponent::Limit(limit))
            )
        }
    )?
}

 // X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000
 // X-FROM;PROP=DTSTART;OP=GTE;TZID=Europe/London;UUID=Event_UUID:19971002T090000
fn parse_from_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-FROM"),
        cut(
            context(
                "X-FROM",
                tuple(
                    (
                        ical_common::semicolon_delimeter,
                        build_property_params_value_parser!(
                            "X-FROM",
                            ("PROP", map(alt((tag("DTSTART"), tag("DTEND"))), |value| ical_common::ParsedValue::Single(value))),
                            ("OP",   map(alt((tag("GTE"),     tag("GT"))),    |value| ical_common::ParsedValue::Single(value))),
                            ("TZID", ical_common::ParsedValue::parse_timezone),
                            ("UUID", ical_common::ParsedValue::parse_single_param),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_date_string,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_semicolon_delimeter, parsed_params,_colon_delimeter, parsed_value)): (&str, (&str, HashMap<&str, ical_common::ParsedValue>, &str, ical_common::ParsedValue))| {
            let ical_common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let timezone =
                match parsed_params.get(&"TZID") {
                    Some(ical_common::ParsedValue::TimeZone(timezone)) => timezone,
                    _ => &rrule::Tz::UTC,
                };

            let datetime_timestamp = parsed_date_string.to_date(Some(*timezone), "X-FROM").unwrap_or_else(|error| {
                panic!("Parsed date string unable to be converted to timestamp, error: {:#?}", error);
            }).timestamp();

            let range_condition_property =
                match parsed_params.get(&"PROP") {
                    Some(ical_common::ParsedValue::Single("DTSTART")) => RangeConditionProperty::DtStart(datetime_timestamp),
                    Some(ical_common::ParsedValue::Single("DTEND"))   => RangeConditionProperty::DtEnd(datetime_timestamp),

                    _ => RangeConditionProperty::DtStart(datetime_timestamp),
                };

            let event_uuid =
                match parsed_params.get(&"UUID") {
                    Some(ical_common::ParsedValue::Single(uuid)) => Some(String::from(*uuid)),
                    _ => None,
                };

            let lower_bound_range_condition =
                match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("GT"))  => LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uuid),
                    Some(ical_common::ParsedValue::Single("GTE")) => LowerBoundRangeCondition::GreaterEqualThan(range_condition_property, event_uuid),

                    _ => LowerBoundRangeCondition::GreaterThan(range_condition_property, event_uuid),
                };

            (remaining, ParsedQueryComponent::FromDateTime(lower_bound_range_condition))
        }
    )
}

// X-UNTIL:19971002T090000Z        => X-UNTIL;PROP=DTSTART;OP=LT;TZID=UTC:19971002T090000
// X-UNTIL;OP=LTE:19971002T090000Z => X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971002T090000
fn parse_until_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-UNTIL"),
        cut(
            context(
                "X-UNTIL",
                tuple(
                    (
                        ical_common::semicolon_delimeter,
                        build_property_params_value_parser!(
                            "X-UNTIL",
                            ("PROP", map(alt((tag("DTSTART"), tag("DTEND"))), |value| ical_common::ParsedValue::Single(value))),
                            ("OP",   map(alt((tag("LTE"),     tag("LT"))),    |value| ical_common::ParsedValue::Single(value))),
                            ("TZID", ical_common::ParsedValue::parse_timezone),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_date_string,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_semicolon_delimeter, parsed_params,_colon_delimeter, parsed_value)): (&str, (&str, HashMap<&str, ical_common::ParsedValue>, &str, ical_common::ParsedValue))| {
            let ical_common::ParsedValue::DateString(parsed_date_string) = parsed_value else {
                panic!("Expected parsed date string, received: {:#?}", parsed_value);
            };

            let timezone =
                match parsed_params.get(&"TZID") {
                    Some(ical_common::ParsedValue::TimeZone(timezone)) => timezone,
                    _ => &rrule::Tz::UTC,
                };

            let datetime_timestamp = parsed_date_string.to_date(Some(*timezone), "X-FROM").unwrap_or_else(|error| {
                panic!("Parsed date string unable to be converted to timestamp, error: {:#?}", error);
            }).timestamp();

            let range_condition_property =
                match parsed_params.get(&"PROP") {
                    Some(ical_common::ParsedValue::Single("DTSTART")) => RangeConditionProperty::DtStart(datetime_timestamp),
                    Some(ical_common::ParsedValue::Single("DTEND"))   => RangeConditionProperty::DtEnd(datetime_timestamp),

                    _ => RangeConditionProperty::DtStart(datetime_timestamp),
                };

            let upper_bound_range_condition =
                match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("LT"))  => UpperBoundRangeCondition::LessThan(range_condition_property),
                    Some(ical_common::ParsedValue::Single("LTE")) => UpperBoundRangeCondition::LessEqualThan(range_condition_property),

                    _ => UpperBoundRangeCondition::LessThan(range_condition_property),
                };

            (remaining, ParsedQueryComponent::UntilDateTime(upper_bound_range_condition))
        }
    )
}

// X-CATEGORIES:CATEGORY_ONE,CATEGORY_TWO  => X-CATEGORIES;OP=AND:CATEGORY_ONE,CATEGORY_TWO
fn parse_categories_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-CATEGORIES"),
        cut(
            context(
                "X-CATEGORIES",
                tuple(
                    (
                        ical_common::semicolon_delimeter,
                        build_property_params_value_parser!(
                            "X-CATEGORIES",
                            ("OP", map(alt((tag("AND"), tag("OR"))), |value| ical_common::ParsedValue::Single(value))),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_list,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_semicolon_delimeter, parsed_params,_colon_delimeter, parsed_value)): (&str, (&str, HashMap<&str, ical_common::ParsedValue>, &str, ical_common::ParsedValue))| {
            let where_operator =
                match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(ical_common::ParsedValue::Single("OR"))  => WhereOperator::Or,

                    _ => WhereOperator::And,
                };

            let ical_common::ParsedValue::List(parsed_categories) = parsed_value else {
                panic!("Expected categories to be a list of Strings, received: {:#?}", parsed_value);
            };

            let parsed_categories: Vec<String> = parsed_categories.into_iter().map(|category| String::from(category)).collect();

            (remaining, ParsedQueryComponent::WhereCategories(parsed_categories, where_operator))
        }
    )
}

// X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID => X-RELATED-TO;OP=AND;RELTYPE=PARENT:PARENT_UUID
fn parse_related_to_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-RELATED-TO"),
        cut(
            context(
                "X-RELATED-TO",
                tuple(
                    (
                        ical_common::semicolon_delimeter,
                        build_property_params_value_parser!(
                            "X-RELATED-TO",
                            ("OP",      map(alt((tag("AND"), tag("OR"))), |value| ical_common::ParsedValue::Single(value))),
                            ("RELTYPE", ical_common::ParsedValue::parse_single_param),
                        ),
                        ical_common::colon_delimeter,
                        ical_common::ParsedValue::parse_single_param,
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_semicolon_delimeter, parsed_params,_colon_delimeter, parsed_value)): (&str, (&str, HashMap<&str, ical_common::ParsedValue>, &str, ical_common::ParsedValue))| {
            let where_operator =
                match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single("AND")) => WhereOperator::And,
                    Some(ical_common::ParsedValue::Single("OR"))  => WhereOperator::Or,

                    _ => WhereOperator::And,
                };

            let parsed_reltype =
                match parsed_params.get(&"OP") {
                    Some(ical_common::ParsedValue::Single(reltype)) => String::from(*reltype),

                    _ => String::from("PARENT"),
                };

            let parsed_related_to_uuid = match parsed_value {
                ical_common::ParsedValue::Single(related_to_uuid) => String::from(related_to_uuid),

                _ => String::from("PARENT"),
            };

            (remaining, ParsedQueryComponent::WhereRelatedTo(KeyValuePair::new(parsed_reltype, parsed_related_to_uuid), where_operator))
        }
    )
}

// X-ORDER-BY:DTSTART
// X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST
// X-ORDER-BY;GEO=48.85299;2.36885:GEO-DIST-DTSTART
fn parse_order_to_query_property_content(input: &str) -> ParserResult<&str, ParsedQueryComponent> {
    preceded(
        tag("X-ORDER-BY"),
        cut(
            context(
                "X-ORDER-BY",
                tuple(
                    (
                        ical_common::semicolon_delimeter,
                        build_property_params_value_parser!(
                            "X-ORDER-BY",
                            ("GEO", ical_common::ParsedValue::parse_lat_long),
                        ),
                        ical_common::colon_delimeter,
                        map(
                            alt(
                                (
                                    tag("GEO-DIST-DTSTART"),
                                    tag("DTSTART-GEO-DIST"),
                                    tag("DTSTART"),
                                )
                            ),
                            |value| ical_common::ParsedValue::Single(value)
                        )
                    )
                )
            )
        )
    )(input).map(
        |(remaining, (_semicolon_delimeter, parsed_params,_colon_delimeter, parsed_value)): (&str, (&str, HashMap<&str, ical_common::ParsedValue>, &str, ical_common::ParsedValue))| {
            let parsed_geo_point =
                match parsed_params.get(&"GEO") {
                    Some(ical_common::ParsedValue::LatLong(latitude, longitude)) => {
                        Some(GeoPoint::new(*longitude, *latitude))
                    },

                    _ => None,
                };

            let ordering_condition = match parsed_value {
                ical_common::ParsedValue::Single("DTSTART-GEO-DIST") => {
                    let Some(parsed_geo_point) = parsed_geo_point else {
                        return Err(
                            nom::Err::Error(
                                nom::error::VerboseError::add_context(
                                    input,
                                    "expected GEO param to be present",
                                    nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                                )
                            )
                        )
                    };

                    OrderingCondition::DtStartGeoDist(parsed_geo_point)
                },

                ical_common::ParsedValue::Single("GEO-DIST-DTSTART") => {
                    let Some(parsed_geo_point) = parsed_geo_point else {
                        return Err(
                            nom::Err::Error(
                                nom::error::VerboseError::add_context(
                                    input,
                                    "expected GEO param to be present",
                                    nom::error::VerboseError::from_error_kind(input, ErrorKind::Satisfy),
                                )
                            )
                        )
                    };

                    OrderingCondition::GeoDistDtStart(parsed_geo_point)
                },

                _ => OrderingCondition::DtStart,
            };

            Ok(
                (
                    remaining,
                    ParsedQueryComponent::Order(ordering_condition)
                )
            )
        }
    )?
}

// parse_timezone_query_property_content
// parse_limit_query_property_content
// parse_from_query_property_content
// parse_until_query_property_content
// parse_categories_query_property_content
// parse_related_to_query_property_content
// parse_order_to_query_property_content

fn parse_query_string(input: &str) -> ParserResult<&str, Query> {
    let (remaining, query_properties) =
        terminated(
            separated_list1(
                tag(" "),
                alt(
                    (
                        parse_timezone_query_property_content,
                        parse_limit_query_property_content,
                        parse_from_query_property_content,
                        parse_until_query_property_content,
                        parse_order_to_query_property_content,
                    )
                )
            ),
            opt(tag(" ")),
        )(input)?;

    let query =
        query_properties.iter()
                        .fold(
                            Query::default(),
                            |mut query, query_property| {
                                match query_property {
                                    ParsedQueryComponent::Limit(limit) => {
                                        query.limit = limit.clone();
                                    },

                                    ParsedQueryComponent::FromDateTime(lower_bound_range_condition) => {
                                        query.lower_bound_range_condition = Some(lower_bound_range_condition.clone());
                                    },

                                    ParsedQueryComponent::UntilDateTime(upper_bound_range_condition) => {
                                        query.upper_bound_range_condition = Some(upper_bound_range_condition.clone());
                                    },

                                    ParsedQueryComponent::InTimezone(timezone) => {
                                        query.in_timezone = timezone.clone();
                                    },

                                    ParsedQueryComponent::Order(ordering_condition) => {
                                        query.ordering_condition = ordering_condition.clone();
                                    },

                                    _ => {
                                        panic!("Unexpected query property: {:#?}", query_property);
                                    },
                                }

                                query
                            }
                        );

    Ok(
        (
            remaining,
            query,
        )
    )
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_query_string() {
        let query_string = [
            "X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London;UUID=Event_UUID:19971002T090000",
            "X-UNTIL;PROP=DTSTART;OP=LTE;TZID=UTC:19971102T090000",
            "X-LIMIT:50",
            "X-TZID:Europe/Vilnius",
            "X-ORDER-BY;GEO=48.85299;2.36885:DTSTART-GEO-DIST",
        ].join(" ");

        assert_eq!(
            parse_query_string(query_string.as_str()),
            Ok(
                (
                    "",
                    Query {
                        where_conditional: None,

                        ordering_condition: OrderingCondition::DtStartGeoDist(
                            GeoPoint {
                                long: 2.36885,
                                lat: 48.85299,
                            },
                        ),

                        lower_bound_range_condition: Some(
                            LowerBoundRangeCondition::GreaterThan(
                                RangeConditionProperty::DtStart(
                                    875779200,
                                ),
                                Some(
                                    String::from("Event_UUID"),
                                ),
                            ),
                        ),

                        upper_bound_range_condition: Some(
                            UpperBoundRangeCondition::LessEqualThan(
                                RangeConditionProperty::DtStart(
                                    878461200,
                                ),
                            ),
                        ),

                        in_timezone: rrule::Tz::Europe__Vilnius,

                        limit: 50,
                    }
                )
            )
        );
    }
}
