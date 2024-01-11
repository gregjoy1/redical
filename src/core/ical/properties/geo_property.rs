use std::hash::{Hash, Hasher};
use std::collections::HashMap;

use nom::{
    branch::alt,
    bytes::complete::tag,
    character::complete::char,
    combinator::{cut, map, opt},
    error::context,
    multi::separated_list1,
    number::complete::double,
    sequence::{preceded, separated_pair, tuple},
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;
use crate::core::ical::parser::macros::*;
use crate::core::ical::serializer::{
    quote_string_if_needed, SerializableICalProperty, SerializedValue,
};

#[derive(Debug, Clone)]
pub struct GeoProperty {
    pub latitude: f64,
    pub longitude: f64,
    pub x_params: Option<HashMap<String, Vec<String>>>,
}

impl PartialEq for GeoProperty {
    fn eq(&self, other: &Self) -> bool {
        self.latitude.total_cmp(&other.latitude).is_eq() &&
        self.longitude.total_cmp(&other.longitude).is_eq() &&
        self.x_params == other.x_params
    }
}

impl Eq for GeoProperty {}

impl Hash for GeoProperty {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serialize_to_ical().hash(state);
    }
}

impl SerializableICalProperty for GeoProperty {
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

        let value = SerializedValue::Single(format!("{};{}", self.latitude, self.longitude));

        (String::from(GeoProperty::NAME), params, value)
    }
}

impl GeoProperty {
    const NAME: &'static str = "GEO";

    pub fn parse_ical(input: &str) -> ParserResult<&str, GeoProperty> {
        preceded(
            tag("GEO"),
            cut(context(
                "GEO",
                tuple((
                    build_property_params_parser!("GEO"),
                    common::colon_delimeter,
                    tuple((double, common::semicolon_delimeter, double)),
                )),
            )),
        )(input)
        .map(
            |(
                remaining,
                (parsed_params, _colon_delimeter, (latitude, _semicolon_delimeter, longitude)),
            ): (
                &str,
                (
                    Option<HashMap<&str, common::ParsedValue>>,
                    &str,
                    (f64, &str, f64),
                ),
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

                let parsed_property = GeoProperty {
                    latitude,
                    longitude,
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
    use nom::error::{ErrorKind, VerboseError, VerboseErrorKind};
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_empty() {
        assert_eq!(
            GeoProperty::parse_ical("GEO:"),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("", VerboseErrorKind::Nom(ErrorKind::Float,),),
                    (":", VerboseErrorKind::Context("GEO",),),
                ]
            }))
        );
    }

    #[test]
    fn test_parse_ical_invalid() {
        assert_eq!(
            GeoProperty::parse_ical("GEO:37.386013;bad"),
            Err(nom::Err::Failure(VerboseError {
                errors: vec![
                    ("bad", VerboseErrorKind::Nom(ErrorKind::Float)),
                    (":37.386013;bad", VerboseErrorKind::Context("GEO",),),
                ]
            }))
        );
    }

    #[test]
    fn test_parse_ical_minimal() {
        assert_eq!(
            GeoProperty::parse_ical("GEO:37.386013;-122.082932"),
            Ok((
                "",
                GeoProperty {
                    x_params: None,
                    latitude: 37.386013,
                    longitude: -122.082932,
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full() {
        assert_eq!(
            GeoProperty::parse_ical(
                r#"GEO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":37.386013;-122.082932"#
            ),
            Ok((
                "",
                GeoProperty {
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
                    latitude: 37.386013,
                    longitude: -122.082932,
                },
            ))
        );
    }

    #[test]
    fn test_parse_ical_full_with_lookahead() {
        assert_eq!(
            GeoProperty::parse_ical(
                r#"GEO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":37.386013;-122.082932 LOCATION:Location text"#
            ),
            Ok((
                " LOCATION:Location text",
                GeoProperty {
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
                    latitude: 37.386013,
                    longitude: -122.082932,
                },
            ))
        );
    }

    #[test]
    fn test_serialize_to_ical() {
        let parsed_geo_property = GeoProperty::parse_ical(
            r#"GEO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":37.386013;-122.082932"#
        ).unwrap().1;

        assert_eq!(
            parsed_geo_property,
            GeoProperty {
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
                latitude: 37.386013,
                longitude: -122.082932,
            },
        );

        let serialized_ical = parsed_geo_property.serialize_to_ical();

        assert_eq!(
            GeoProperty::parse_ical(serialized_ical.as_str()).unwrap().1,
            parsed_geo_property
        );

        assert_eq!(
            serialized_ical,
            String::from(
                r#"GEO;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:37.386013;-122.082932"#
            ),
        );
    }
}
