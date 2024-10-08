use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{tuple, pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::float::Float;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarGeoProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

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
            |params: &mut GeoPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for GeoPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.clone().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<GeoPropertyParams> for ContentLineParams {
    fn from(geo_params: GeoPropertyParams) -> Self {
        geo_params.to_content_line_params()
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
    pub latitude: Option<Float>,
    pub longitude: Option<Float>,
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
                                // Parse either populated `GEO:<LAT>;<LONG>` or blank `GEO:;`.
                                alt((
                                    map(
                                        tuple((Float::parse_ical, semicolon, Float::parse_ical)),
                                        |(latitude, semicolon, longitude)| (Some(latitude), semicolon, Some(longitude)),
                                    ),
                                    map(
                                        semicolon,
                                        |semicolon| (None, semicolon, None),
                                    ),
                                )),
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

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_with_context(context).render_ical()
    }
}

impl ICalendarProperty for GeoProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "GEO",
            (
                ContentLineParams::from(&self.params),
                format!("{};{}", self.latitude.render_ical(), self.longitude.render_ical()),
            )
        ))
    }
}

impl ICalendarGeoProperty for GeoProperty {
    fn is_blank(&self) -> bool {
        self.latitude.is_none() || self.longitude.is_none()
    }

    fn get_latitude(&self) -> Option<f64> {
        self.latitude.to_owned().map(f64::from)
    }

    fn get_longitude(&self) -> Option<f64> {
        self.longitude.to_owned().map(f64::from)
    }
}

impl std::hash::Hash for GeoProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(GeoProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            GeoProperty::parse_ical(
                "GEO:;".into()
            ),
            (
                "",
                GeoProperty {
                    params: GeoPropertyParams::default(),
                    latitude: None,
                    longitude: None,
                },
            ),
        );

        assert_parser_output!(
            GeoProperty::parse_ical(
                "GEO:37.386013;-122.082932".into()
            ),
            (
                "",
                GeoProperty {
                    params: GeoPropertyParams::default(),
                    latitude: Some(Float(37.386013_f64)),
                    longitude: Some(Float(-122.082932_f64)),
                },
            ),
        );

        assert_parser_error!(
            GeoProperty::parse_ical("GEO:37.386013;".into()),
            nom::Err::Failure(
                span: "37.386013;",
                message: "expected iCalendar RFC-5545 SEMICOLON char",
                context: ["GEO"],
            ),
        );

        assert_parser_output!(
            GeoProperty::parse_ical(
                "GEO:;-122.082932".into()
            ),
            (
                "-122.082932",
                GeoProperty {
                    params: GeoPropertyParams::default(),
                    latitude: None,
                    longitude: None,
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
                    latitude: Some(Float(37.386013_f64)),
                    longitude: Some(Float(-122.082932_f64)),
                },
            ),
        );

        assert_parser_output!(
            GeoProperty::parse_ical("GEO;X-TEST=X_VALUE;TEST=VALUE:; DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                GeoProperty {
                    params: GeoPropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    latitude: None,
                    longitude: None,
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
                latitude: Some(Float(37.386013_f64)),
                longitude: Some(Float(-122.082932_f64)),
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
                latitude: Some(Float(37.386013_f64)),
                longitude: Some(Float(-122.082932_f64)),
            }.render_ical(),
            String::from("GEO;TEST=VALUE;X-TEST=X_VALUE:37.386013;-122.082932"),
        );
    }
}
