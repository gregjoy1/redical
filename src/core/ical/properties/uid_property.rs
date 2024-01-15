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
    quote_string_if_needed, SerializableICalProperty, SerializedValue,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone)]
pub struct UIDProperty {
    pub uid: String,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(UIDProperty);

impl SerializableICalProperty for UIDProperty {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

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

        let value = SerializedValue::Single(self.uid.clone());

        (String::from(UIDProperty::NAME), params, value)
    }
}

impl UIDProperty {
    const NAME: &'static str = "UID";

    pub fn parse_ical(input: &str) -> ParserResult<&str, UIDProperty> {
        preceded(
            tag("UID"),
            cut(context(
                "UID",
                tuple((
                    build_property_params_parser!("UID"),
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
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        let parsed_x_param_value =
                            value.expect_list().into_iter().map(String::from).collect();

                        x_params
                            .get_or_insert(HashMap::new())
                            .insert(String::from(key), parsed_x_param_value);
                    }
                }

                let uid = String::from(parsed_value.trim());

                let parsed_property = UIDProperty { uid, x_params };

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
            UIDProperty::parse_ical("UID:"),
            Ok((
                "",
                UIDProperty {
                    x_params: None,
                    uid: String::from(""),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            UIDProperty::parse_ical("UID:UID text."),
            Ok((
                "",
                UIDProperty {
                    x_params: None,
                    uid: String::from("UID text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            UIDProperty::parse_ical(
                r#"UID;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":UID text."#,
            ),
            Ok((
                "",
                UIDProperty {
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
                    uid: String::from("UID text."),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            UIDProperty::parse_ical(
                r#"UID;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":UID text. SUMMARY:Summary text"#,
            ),
            Ok((
                " SUMMARY:Summary text",
                UIDProperty {
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
                    uid: String::from("UID text."),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_categories_property = UIDProperty::parse_ical(
            r#"UID;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":UID text."#,
        )
        .unwrap()
        .1;

        assert_eq!(
            parsed_categories_property,
            UIDProperty {
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
                uid: String::from("UID text."),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical();

        assert_eq!(
            UIDProperty::parse_ical(serialized_ical.as_str()).unwrap().1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"UID;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:UID text."#
            ),
        );
    }
}
