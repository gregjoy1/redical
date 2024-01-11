use std::hash::{Hash, Hasher};
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

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct RelatedToProperty {
    pub reltype: Option<String>,
    pub uid: String,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

impl Hash for RelatedToProperty {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serialize_to_ical().hash(state);
    }
}

impl SerializableICalProperty for RelatedToProperty {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        let mut param_key_value_pairs: Vec<(String, String)> = Vec::new();

        if let Some(reltype) = &self.reltype {
            param_key_value_pairs.push((
                String::from("RELTYPE"),
                quote_string_if_needed(reltype, Self::reltype_param_value),
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

        let value =
            SerializedValue::Single(quote_string_if_needed(&self.uid, properties::value_text));

        (String::from(RelatedToProperty::NAME), params, value)
    }
}

impl RelatedToProperty {
    const NAME: &'static str = "RELATED-TO";
    const DEFAULT_RELTYPE: &'static str = "PARENT";

    // reltypeparam       = "RELTYPE" "="
    //                     ("PARENT"      ; Parent relationship. Default.
    //                    / "CHILD"       ; Child relationship
    //                    / "SIBLING      ; Sibling relationship
    //                    / iana-token    ; Some other IANA registered
    //                                    ; iCalendar relationship type
    //                    / x-name)       ; A non-standard, experimental
    //                                    ; relationship type
    fn reltype_param_value(input: &str) -> ParserResult<&str, &str> {
        context(
            "reltypeparam",
            alt((
                tag("PARENT"),
                tag("CHILD"),
                tag("SIBLING"),
                properties::known_iana_properties,
                common::x_name,
            )),
        )(input)
    }

    pub fn parse_ical(input: &str) -> ParserResult<&str, RelatedToProperty> {
        preceded(
            tag("RELATED-TO"),
            cut(context(
                "RELATED-TO",
                tuple((
                    build_property_params_parser!(
                        "RELATED-TO",
                        (
                            "RELTYPE",
                            common::ParsedValue::parse_single(Self::reltype_param_value)
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
                let mut reltype: Option<String> = None;
                let mut x_params: Option<HashMap<String, Vec<String>>> = None;

                if let Some(parsed_params) = parsed_params.clone() {
                    for (key, value) in parsed_params {
                        match key {
                            "RELTYPE" => {
                                let parsed_reltype = value.expect_single();
                                let _ = reltype.insert(String::from(parsed_reltype));
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

                let uid = String::from(parsed_value.trim());

                let parsed_property = RelatedToProperty {
                    reltype,
                    uid,
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
            RelatedToProperty::parse_ical("RELATED-TO:"),
            Ok((
                "",
                RelatedToProperty {
                    reltype: None,
                    x_params: None,
                    uid: String::from(""),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            RelatedToProperty::parse_ical("RELATED-TO:UID"),
            Ok((
                "",
                RelatedToProperty {
                    reltype: None,
                    x_params: None,
                    uid: String::from("UID"),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            RelatedToProperty::parse_ical(
                r#"RELATED-TO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";RELTYPE=X-CUSTOM-RELTYPE;X-TEST-KEY-TWO="KEY -🎄- TWO":  UID "#,
            ),
            Ok((
                "",
                RelatedToProperty {
                    reltype: Some(String::from("X-CUSTOM-RELTYPE")),
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
                    uid: String::from("UID"),
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            RelatedToProperty::parse_ical(
                r#"RELATED-TO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";RELTYPE=X-CUSTOM-RELTYPE;X-TEST-KEY-TWO="KEY -🎄- TWO":  UID  SUMMARY:Summary text"#,
            ),
            Ok((
                "  SUMMARY:Summary text",
                RelatedToProperty {
                    reltype: Some(String::from("X-CUSTOM-RELTYPE")),
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
                    uid: String::from("UID"),
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_categories_property = RelatedToProperty::parse_ical(
            r#"RELATED-TO;RELTYPE=X-CUSTOM-RELTYPE;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":  UID "#,
        ).unwrap().1;

        assert_eq!(
            parsed_categories_property,
            RelatedToProperty {
                reltype: Some(String::from("X-CUSTOM-RELTYPE")),
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
                uid: String::from("UID"),
            },
        );

        let serialized_ical = parsed_categories_property.serialize_to_ical();

        assert_eq!(
            RelatedToProperty::parse_ical(serialized_ical.as_str())
                .unwrap()
                .1,
            parsed_categories_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"RELATED-TO;RELTYPE=X-CUSTOM-RELTYPE;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:UID"#
            ),
        );
    }
}
