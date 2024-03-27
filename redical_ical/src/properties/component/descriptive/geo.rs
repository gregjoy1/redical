use nom::error::context;
use nom::branch::alt;
use nom::sequence::{tuple, pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::float::Float;
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    GeoParams,
    GeoParam,
    "GEOPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

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
pub struct Geo {
    pub params: GeoParams,
    pub latitude: Float,
    pub longitude: Float,
}

impl ICalendarEntity for Geo {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "GEO",
            preceded(
                tag("GEO"),
                cut(
                    map(
                        pair(
                            opt(GeoParams::parse_ical),
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
                            Geo {
                                params: params.unwrap_or(GeoParams::default()),
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
        format!("GEO{}:{};{}", self.params.render_ical(), self.latitude.render_ical(), self.longitude.render_ical())
    }
}

impl_icalendar_entity_traits!(Geo);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Geo::parse_ical(
                r#"GEO:37.386013;-122.082932"#.into()
            ),
            (
                "",
                Geo {
                    params: GeoParams::default(),
                    latitude: Float(37.386013_f64),
                    longitude: Float(-122.082932_f64),
                },
            ),
        );

        assert_parser_output!(
            Geo::parse_ical("GEO;X-TEST=X_VALUE;TEST=VALUE:37.386013;-122.082932".into()),
            (
                "",
                Geo {
                    params: GeoParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    latitude: Float(37.386013_f64),
                    longitude: Float(-122.082932_f64),
                },
            ),
        );

        assert!(Geo::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Geo {
                params: GeoParams::default(),
                latitude: Float(37.386013_f64),
                longitude: Float(-122.082932_f64),
            }.render_ical(),
            String::from("GEO:37.386013;-122.082932"),
        );

        assert_eq!(
            Geo {
                params: GeoParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                latitude: Float(37.386013_f64),
                longitude: Float(-122.082932_f64),
            }.render_ical(),
            String::from("GEO;X-TEST=X_VALUE;TEST=VALUE:37.386013;-122.082932"),
        );
    }
}
