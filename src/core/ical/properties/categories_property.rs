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
use crate::core::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializedValue,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct CategoriesProperty {
    pub language: Option<String>,
    pub categories: HashSet<String>,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(CategoriesProperty);

impl SerializableICalProperty for CategoriesProperty {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

        if let Some(language) = &self.language {
            param_key_value_pairs.push((
                String::from("LANGUAGE"),
                quote_string_if_needed(language, common::language),
            ));
        }

        if let Some(x_params) = &self.x_params {
            for (key, values) in x_params {
                let param_value = values
                    .iter()
                    .map(|value| quote_string_if_needed(value, common::param_value))
                    .collect::<Vec<String>>()
                    .join(",");

                param_key_value_pairs.push((key.clone(), param_value));
            }
        }

        param_key_value_pairs.sort();

        let params = if param_key_value_pairs.is_empty() {
            None
        } else {
            Some(param_key_value_pairs)
        };

        let mut values = Vec::new();

        for category in &self.categories {
            values.push(quote_string_if_needed(category, properties::value_text));
        }

        values.sort();

        let value = SerializedValue::List(values);

        (String::from(CategoriesProperty::NAME), params, value)
    }
}

impl CategoriesProperty {
    const NAME: &'static str = "CATEGORIES";

    pub fn parse_ical(input: &str) -> ParserResult<&str, CategoriesProperty> {
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
                                let parsed_x_param_value =
                                    value.expect_list().into_iter().map(String::from).collect();

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
                    categories: HashSet::from([String::from("APPOINTMENT"),]),
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
                        (
                            String::from("X-TEST-KEY-TWO"),
                            vec![String::from("KEY -🎄- TWO")]
                        ),
                        (
                            String::from("X-TEST-KEY-ONE"),
                            vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                        ),
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
                        (
                            String::from("X-TEST-KEY-TWO"),
                            vec![String::from("KEY -🎄- TWO")]
                        ),
                        (
                            String::from("X-TEST-KEY-ONE"),
                            vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                        ),
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
    fn test_serialize_to_ical() {
        let parsed_categories_property = CategoriesProperty::parse_ical(
            r#"CATEGORIES;LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
        ).unwrap().1;

        assert_eq!(
            parsed_categories_property,
            CategoriesProperty {
                language: Some(String::from("ENGLISH")),
                x_params: Some(HashMap::from([
                    (
                        String::from("X-TEST-KEY-TWO"),
                        vec![String::from("KEY -🎄- TWO")]
                    ),
                    (
                        String::from("X-TEST-KEY-ONE"),
                        vec![String::from("VALUE_ONE"), String::from("VALUE_TWO")]
                    ),
                ])),
                categories: HashSet::from([
                    String::from("APPOINTMENT"),
                    String::from("EDUCATION"),
                    String::from("TESTING\\nESCAPED\\,CHARS:OK"),
                    String::from("QUOTED, + 🎄 STRING"),
                ]),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical();

        assert_eq!(
            CategoriesProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"CATEGORIES;LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:"QUOTED, + 🎄 STRING",APPOINTMENT,EDUCATION,TESTING\nESCAPED\,CHARS:OK"#
            ),
        );
    }
}
