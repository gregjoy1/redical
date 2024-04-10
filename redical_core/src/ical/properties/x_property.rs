use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{cut, map, opt},
    error::context,
    multi::separated_list1,
    sequence::{pair, preceded, separated_pair, tuple},
};

use crate::ical::parser::common;
use crate::ical::parser::common::ParserResult;
use crate::ical::parser::macros::*;
use crate::ical::parser::properties;
use crate::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializationPreferences, SerializedValue,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct XProperty {
    pub language: Option<String>,
    pub name: String,
    pub value: String,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(XProperty);

impl SerializableICalProperty for XProperty {
    fn serialize_to_split_ical(
        &self,
        _preferences: Option<&SerializationPreferences>,
    ) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
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

        let name = self.name.to_owned();

        let params = if param_key_value_pairs.is_empty() {
            None
        } else {
            Some(param_key_value_pairs)
        };

        let value = SerializedValue::Single(self.value.clone());

        (name, params, value)
    }
}

impl XProperty {
    const NAME: &'static str = "X-PROPERTY";

    pub fn parse_ical(input: &str) -> ParserResult<&str, XProperty> {
        pair(
            common::x_name,
            cut(context(
                "X-PROPERTY",
                tuple((
                    build_property_params_parser!(
                        "X-PROPERTY",
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
            |(remaining, (parsed_name, (parsed_params, _colon_delimeter, parsed_value))): (
                &str,
                (
                    &str,
                    (Option<HashMap<&str, common::ParsedValue>>, &str, &str),
                ),
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

                let name = String::from(parsed_name.trim());
                let value = String::from(parsed_value.trim());

                let parsed_property = XProperty {
                    language,
                    name,
                    value,
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
            XProperty::parse_ical("X-PROPERTY:"),
            Ok((
                "",
                XProperty {
                    language: None,
                    x_params: None,
                    name: String::from("X-PROPERTY"),
                    value: String::from(""),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_with_escapes() {
        assert_eq!(
            XProperty::parse_ical("X-PROPERTY:Experimental\\, escaped\\; property\\: text."),
            Ok((
                "",
                XProperty {
                    language: None,
                    x_params: None,
                    name: String::from("X-PROPERTY"),
                    value: String::from("Experimental\\, escaped\\; property\\: text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            XProperty::parse_ical("X-PROPERTY:Experimental property text."),
            Ok((
                "",
                XProperty {
                    language: None,
                    x_params: None,
                    name: String::from("X-PROPERTY"),
                    value: String::from("Experimental property text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            XProperty::parse_ical(
                r#"X-PROPERTY;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":Experimental property text."#,
            ),
            Ok((
                "",
                XProperty {
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
                    name: String::from("X-PROPERTY"),
                    value: String::from("Experimental property text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            XProperty::parse_ical(
                r#"X-PROPERTY;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":Experimental property text. LOCATION:Location text"#,
            ),
            Ok((
                " LOCATION:Location text",
                XProperty {
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
                    name: String::from("X-PROPERTY"),
                    value: String::from("Experimental property text."),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_categories_property = XProperty::parse_ical(
            r#"X-PROPERTY;LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":Experimental property text."#,
        ).unwrap().1;

        assert_eq!(
            parsed_categories_property,
            XProperty {
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
                name: String::from("X-PROPERTY"),
                value: String::from("Experimental property text."),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical(None);

        assert_eq!(
            XProperty::parse_ical(serialized_ical.as_str()).unwrap().1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"X-PROPERTY;LANGUAGE=ENGLISH;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:Experimental property text."#
            ),
        );
    }
}
