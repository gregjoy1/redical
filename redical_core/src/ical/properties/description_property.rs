use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{cut, map, opt},
    error::context,
    multi::separated_list1,
    sequence::{preceded, separated_pair, tuple},
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;
use crate::core::ical::parser::macros::*;
use crate::core::ical::parser::properties;
use crate::core::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializationPreferences, SerializedValue,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct DescriptionProperty {
    pub altrep: Option<String>,
    pub language: Option<String>,
    pub description: String,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(DescriptionProperty);

impl SerializableICalProperty for DescriptionProperty {
    fn serialize_to_split_ical(
        &self,
        _preferences: Option<&SerializationPreferences>,
    ) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

        if let Some(altrep) = &self.altrep {
            param_key_value_pairs.push((String::from("ALTREP"), altrep.clone()));
        }

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

        let value = SerializedValue::Single(self.description.clone());

        (String::from(DescriptionProperty::NAME), params, value)
    }
}

impl DescriptionProperty {
    const NAME: &'static str = "DESCRIPTION";

    pub fn parse_ical(input: &str) -> ParserResult<&str, DescriptionProperty> {
        preceded(
            tag("DESCRIPTION"),
            cut(context(
                "DESCRIPTION",
                tuple((
                    build_property_params_parser!(
                        "DESCRIPTION",
                        (
                            "ALTREP",
                            common::ParsedValue::parse_single(common::double_quoted_uri)
                        ),
                        (
                            "LANGUAGE",
                            common::ParsedValue::parse_single(common::language)
                        ),
                    ),
                    common::colon_delimeter,
                    alt((common::quoted_string, properties::value_text)),
                )),
            )),
        )(input)
        .map(
            |(remaining, (parsed_params, _colon_delimeter, parsed_value)): (
                &str,
                (Option<HashMap<&str, common::ParsedValue>>, &str, &str),
            )| {
                let mut altrep: Option<String> = None;
                let mut language: Option<String> = None;
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        match key {
                            "ALTREP" => {
                                let parsed_altrep = value.expect_single();
                                let _ = altrep.insert(String::from(parsed_altrep));
                            }

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

                let mut description = String::from(parsed_value.trim());

                let parsed_property = DescriptionProperty {
                    altrep,
                    language,
                    description,
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
            DescriptionProperty::parse_ical("DESCRIPTION:"),
            Ok((
                "",
                DescriptionProperty {
                    altrep: None,
                    language: None,
                    x_params: None,
                    description: String::from(""),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            DescriptionProperty::parse_ical("DESCRIPTION:Description text."),
            Ok((
                "",
                DescriptionProperty {
                    altrep: None,
                    language: None,
                    x_params: None,
                    description: String::from("Description text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            DescriptionProperty::parse_ical(
                r#"DESCRIPTION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":Description text."#,
            ),
            Ok((
                "",
                DescriptionProperty {
                    altrep: Some(String::from("\"http://xyzcorp.com/conf-rooms/f123.vcf\"")),
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
                    description: String::from("Description text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            DescriptionProperty::parse_ical(
                r#"DESCRIPTION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":Description text. DESCRIPTION:Description text"#,
            ),
            Ok((
                " DESCRIPTION:Description text",
                DescriptionProperty {
                    altrep: Some(String::from("\"http://xyzcorp.com/conf-rooms/f123.vcf\"")),
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
                    description: String::from("Description text."),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_categories_property = DescriptionProperty::parse_ical(
            r#"DESCRIPTION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":Description text."#,
        ).unwrap().1;

        assert_eq!(
            parsed_categories_property,
            DescriptionProperty {
                altrep: Some(String::from("\"http://xyzcorp.com/conf-rooms/f123.vcf\"")),
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
                description: String::from("Description text."),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical(None);

        assert_eq!(
            DescriptionProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"DESCRIPTION;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:Description text."#
            ),
        );
    }
}
