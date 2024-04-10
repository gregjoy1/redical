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

use crate::ical::parser::common;
use crate::ical::parser::common::ParserResult;
use crate::ical::parser::macros::*;
use crate::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializationPreferences, SerializedValue,
};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct ClassProperty {
    pub class: String,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

implement_property_ord_partial_ord_and_hash_traits!(ClassProperty);

impl SerializableICalProperty for ClassProperty {
    fn serialize_to_split_ical(
        &self,
        _preferences: Option<&SerializationPreferences>,
    ) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
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

        let value = SerializedValue::Single(self.class.clone());

        (String::from(ClassProperty::NAME), params, value)
    }
}

impl ClassProperty {
    const NAME: &'static str = "CLASS";

    pub fn parse_ical(input: &str) -> ParserResult<&str, ClassProperty> {
        preceded(
            tag("CLASS"),
            cut(context(
                "CLASS",
                tuple((
                    build_property_params_parser!("CLASS"),
                    common::colon_delimeter,
                    alt((
                        tag("PUBLIC"),
                        tag("PRIVATE"),
                        tag("CONFIDENTIAL"),
                        common::x_name,
                        common::iana_token,
                    )),
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

                let class = String::from(parsed_value.trim());

                let parsed_property = ClassProperty { class, x_params };

                (remaining, parsed_property)
            },
        )
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use nom::error::{ErrorKind, VerboseError, VerboseErrorKind};
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_empty() {
        assert_eq!(
            ClassProperty::parse_ical("CLASS:"),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("", VerboseErrorKind::Nom(ErrorKind::TakeWhile1,),),
                    ("", VerboseErrorKind::Context("IANA token",),),
                    ("", VerboseErrorKind::Nom(ErrorKind::Alt,),),
                    (":", VerboseErrorKind::Context("CLASS",),),
                ]
            }))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            ClassProperty::parse_ical("CLASS:PUBLIC"),
            Ok((
                "",
                ClassProperty {
                    x_params: None,
                    class: String::from("PUBLIC"),
                },
            ))
        );

        assert_eq!(
            ClassProperty::parse_ical("CLASS:PRIVATE"),
            Ok((
                "",
                ClassProperty {
                    x_params: None,
                    class: String::from("PRIVATE"),
                },
            ))
        );

        assert_eq!(
            ClassProperty::parse_ical("CLASS:CONFIDENTIAL"),
            Ok((
                "",
                ClassProperty {
                    x_params: None,
                    class: String::from("CONFIDENTIAL"),
                },
            ))
        );

        assert_eq!(
            ClassProperty::parse_ical("CLASS:X-VALUE"),
            Ok((
                "",
                ClassProperty {
                    x_params: None,
                    class: String::from("X-VALUE"),
                },
            ))
        );

        assert_eq!(
            ClassProperty::parse_ical("CLASS:🎄-VALUE"),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("🎄-VALUE", VerboseErrorKind::Nom(ErrorKind::TakeWhile1,),),
                    ("🎄-VALUE", VerboseErrorKind::Context("IANA token",),),
                    ("🎄-VALUE", VerboseErrorKind::Nom(ErrorKind::Alt,),),
                    (":🎄-VALUE", VerboseErrorKind::Context("CLASS",)),
                ]
            }))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            ClassProperty::parse_ical(
                r#"CLASS;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PUBLIC"#,
            ),
            Ok((
                "",
                ClassProperty {
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
                    class: String::from("PUBLIC"),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            ClassProperty::parse_ical(
                r#"CLASS;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PUBLIC CLASS:Location text"#,
            ),
            Ok((
                " CLASS:Location text",
                ClassProperty {
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
                    class: String::from("PUBLIC"),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_categories_property = ClassProperty::parse_ical(
            r#"CLASS;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PUBLIC"#,
        )
        .unwrap()
        .1;

        assert_eq!(
            parsed_categories_property,
            ClassProperty {
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
                class: String::from("PUBLIC"),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical(None);

        assert_eq!(
            ClassProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"CLASS;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:PUBLIC"#
            ),
        );
    }
}
