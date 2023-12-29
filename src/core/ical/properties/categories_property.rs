use std::collections::{HashMap, HashSet};

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{cut, map, opt},
    error::context,
    multi::{separated_list0, separated_list1},
    sequence::{preceded, separated_pair, tuple},
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;
use crate::core::ical::parser::macros::*;
use crate::core::ical::parser::properties;

#[derive(Debug, PartialEq)]
pub struct CategoriesProperty {
    language: Option<String>,
    categories: HashSet<String>,
    x_params: Option<HashMap<String, Vec<String>>>,
}

impl CategoriesProperty {
    fn parse_ical(input: &str) -> ParserResult<&str, CategoriesProperty> {
        preceded(
            tag("CATEGORIES"),
            cut(context(
                "CATEGORIES",
                tuple((
                    build_property_params_parser!(
                        "CATEGORIES",
                        (
                            "LANGUAGE",
                            common::ParsedValue::parse_single(common::language)
                        ),
                    ),
                    common::colon_delimeter,
                    separated_list0(
                        char(','),
                        alt((common::quoted_string, properties::value_text)),
                    ),
                )),
            )),
        )(input)
        .map(
            |(remaining, (parsed_params, _colon_delimeter, parsed_value_list)): (
                &str,
                (Option<HashMap<&str, common::ParsedValue>>, &str, Vec<&str>),
            )| {
                let mut language: Option<String> = None;
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        match key {
                            "LANGUAGE" => {
                                let parsed_language = value.expect_single();
                                let _ = language.insert(String::from(parsed_language));
                            }

                            _ => {
                                let parsed_x_param_value = value.expect_list().iter().map(|value| String::from(*value)).collect();

                                x_params
                                    .get_or_insert(HashMap::new())
                                    .insert(String::from(key), parsed_x_param_value);
                            }
                        }
                    }
                }

                let mut categories: HashSet<String> = parsed_value_list
                    .into_iter()
                    .map(|category| String::from(category.trim()))
                    .collect();

                categories.retain(|category| !category.is_empty());

                let parsed_property = CategoriesProperty {
                    language,
                    categories,
                    x_params,
                };

                (remaining, parsed_property)
            },
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_empty() {
        assert_eq!(
            CategoriesProperty::parse_ical("CATEGORIES:"),
            Ok((
                "",
                CategoriesProperty {
                    language: None,
                    x_params: None,
                    categories: HashSet::from([]),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            CategoriesProperty::parse_ical("CATEGORIES:APPOINTMENT"),
            Ok((
                "",
                CategoriesProperty {
                    language: None,
                    x_params: None,
                    categories: HashSet::from([
                        String::from("APPOINTMENT"),
                    ]),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            CategoriesProperty::parse_ical(
                r#"CATEGORIES;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
            ),
            Ok((
                "",
                CategoriesProperty {
                    language: Some(String::from("ENGLISH")),
                    x_params: Some(HashMap::from([
                        (String::from("X-TEST-KEY-TWO"), vec![String::from("KEY -🎄- TWO")]),
                        (String::from("X-TEST-KEY-ONE"), vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]),
                    ])),
                    categories: HashSet::from([
                        String::from("APPOINTMENT"),
                        String::from("EDUCATION"),
                        String::from("TESTING\\nESCAPED\\,CHARS:OK"),
                        String::from("QUOTED, + 🎄 STRING"),
                    ]),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            CategoriesProperty::parse_ical(
                r#"CATEGORIES;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK SUMMARY:Summary text"#,
            ),
            Ok((
                " SUMMARY:Summary text",
                CategoriesProperty {
                    language: Some(String::from("ENGLISH")),
                    x_params: Some(HashMap::from([
                        (String::from("X-TEST-KEY-TWO"), vec![String::from("KEY -🎄- TWO")]),
                        (String::from("X-TEST-KEY-ONE"), vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]),
                    ])),
                    categories: HashSet::from([
                        String::from("APPOINTMENT"),
                        String::from("EDUCATION"),
                        String::from("TESTING\\nESCAPED\\,CHARS:OK"),
                        String::from("QUOTED, + 🎄 STRING"),
                    ]),
                },
            ))
        );
    }
}
