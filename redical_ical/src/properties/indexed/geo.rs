use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{tuple, pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::property_value_data_types::float::Float;

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct GeoPropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for GeoPropertyParams {
    define_property_params_ical_parser!(
        GeoPropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut GeoPropertyParams, key: ParserInput, value: ParserInput| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&GeoPropertyParams> for ContentLineParams {
    fn from(class_params: &GeoPropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in class_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<GeoPropertyParams> for ContentLineParams {
    fn from(class_params: GeoPropertyParams) -> Self {
        ContentLineParams::from(&class_params)
    }
}

// Geographic Position
//
// Property Name:  GEO
//
// Purpose:  This property specifies information related to the global
//    position for the activity specified by a calendar component.
//
// Value Type:  FLOAT.  The value MUST be two SEMICOLON-separated FLOAT
//    values.
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in "VEVENT" or "VTODO"
//    calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     geo        = "GEO" geoparam ":" geovalue CRLF
//
//     geoparam   = *(";" other-param)
//
//     geovalue   = float ";" float
//     ;Latitude and Longitude components
//
// Example:  The following is an example of this property:
//
//     GEO:37.386013;-122.082932
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GeoProperty {
    pub params: GeoPropertyParams,
    pub latitude: Float,
    pub longitude: Float,
}

impl ICalendarEntity for GeoProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "GEO",
            preceded(
                tag("GEO"),
                cut(
                    map(
                        pair(
                            opt(GeoPropertyParams::parse_ical),
                            preceded(
                                colon,
                                tuple((
                                    Float::parse_ical,
                                    semicolon,
                                    Float::parse_ical,
                                ))
                            ),
                        ),
                        |(params, (latitude, _semicolon, longitude))| {
                            GeoProperty {
                                params: params.unwrap_or(GeoPropertyParams::default()),
                                latitude,
                                longitude,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        ContentLine::from(self).render_ical()
    }
}

impl From<&GeoProperty> for ContentLine {
    fn from(geo_property: &GeoProperty) -> Self {
        ContentLine::from((
            "GEO",
            (
                ContentLineParams::from(&geo_property.params),
                format!("{};{}", geo_property.latitude.render_ical(), geo_property.longitude.render_ical()),
            )
        ))
    }
}

impl_icalendar_entity_traits!(GeoProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            GeoProperty::parse_ical(
                r#"GEO:37.386013;-122.082932"#.into()
            ),
            (
                "",
                GeoProperty {
                    params: GeoPropertyParams::default(),
                    latitude: Float(37.386013_f64),
                    longitude: Float(-122.082932_f64),
                },
            ),
        );

        assert_parser_output!(
            GeoProperty::parse_ical("GEO;X-TEST=X_VALUE;TEST=VALUE:37.386013;-122.082932 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                GeoProperty {
                    params: GeoPropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    latitude: Float(37.386013_f64),
                    longitude: Float(-122.082932_f64),
                },
            ),
        );

        assert!(GeoProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            GeoProperty {
                params: GeoPropertyParams::default(),
                latitude: Float(37.386013_f64),
                longitude: Float(-122.082932_f64),
            }.render_ical(),
            String::from("GEO:37.386013;-122.082932"),
        );

        assert_eq!(
            GeoProperty {
                params: GeoPropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                latitude: Float(37.386013_f64),
                longitude: Float(-122.082932_f64),
            }.render_ical(),
            String::from("GEO;TEST=VALUE;X-TEST=X_VALUE:37.386013;-122.082932"),
        );
    }
}
