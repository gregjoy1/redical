use nom::branch::alt;
use nom::error::context;
use nom::sequence::{preceded, pair, terminated, tuple};
use nom::combinator::{map, cut, opt};

use crate::grammar::{tag, colon, semicolon};

use crate::value_data_types::float::Float;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarGeoProperty, define_property_params_ical_parser};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// DIST = float "KM" / float "MI"
//
// ;Default is 10KM
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DistValue {
    Kilometers(Float), // KM
    Miles(Float),      // MI
}

impl ICalendarEntity for DistValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "OP",
            alt((
                map(terminated(Float::parse_ical, tag("KM")), DistValue::Kilometers),
                map(terminated(Float::parse_ical, tag("MI")), DistValue::Miles),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::Kilometers(value) => format!("{}KM", value.render_ical()),
           Self::Miles(value) => format!("{}MI", value.render_ical()),
        }
    }
}

impl Default for DistValue {
    fn default() -> Self {
        DistValue::Kilometers(Float(10.0_f64))
    }
}

impl_icalendar_entity_traits!(DistValue);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct XGeoPropertyParams {
    pub dist: DistValue,
}

impl ICalendarEntity for XGeoPropertyParams {
    define_property_params_ical_parser!(
        XGeoPropertyParams,
        (
            pair(tag("DIST"), cut(preceded(tag("="), DistValue::parse_ical))),
            |params: &mut XGeoPropertyParams, (_key, value): (ParserInput, DistValue)| params.dist = value,
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XGeoPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        // TODO: handle context.
        content_line_params.insert(String::from("DIST"), self.dist.render_ical());

        content_line_params
    }
}

/// Query GEO-DIST where condition property.
///
/// Currently only UID value is available.
///
/// Example:
///
/// X-GEO;DIST=1.5KM:48.85299;2.36885
/// X-GEO;DIST=30MI:48.85299;2.36885
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XGeoProperty {
    pub params: XGeoPropertyParams,
    pub latitude: Float,
    pub longitude: Float,
}

impl ICalendarEntity for XGeoProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-GEO",
            preceded(
                tag("X-GEO"),
                cut(
                    map(
                        pair(
                            opt(XGeoPropertyParams::parse_ical),
                            preceded(
                                colon,
                                tuple((Float::parse_ical, semicolon, Float::parse_ical)),
                            ),
                        ),
                        |(params, (latitude, _, longitude))| {
                            XGeoProperty {
                                params: params.unwrap_or(XGeoPropertyParams::default()),
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

impl ICalendarProperty for XGeoProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-GEO",
            (
                ContentLineParams::from(&self.params),
                format!("{};{}", self.latitude.render_ical(), self.longitude.render_ical()),
            )
        ))
    }
}

impl ICalendarGeoProperty for XGeoProperty {
    fn get_latitude(&self) -> f64 {
        self.latitude.to_owned().into()
    }

    fn get_longitude(&self) -> f64 {
        self.longitude.to_owned().into()
    }
}

impl std::hash::Hash for XGeoProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XGeoProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XGeoProperty::parse_ical("X-GEO:48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XGeoProperty {
                    params: XGeoPropertyParams { dist: DistValue::Kilometers(Float(10.0_f64)) },
                    latitude: Float(48.85299_f64),
                    longitude: Float(2.36885_f64),
                },
            ),
        );

        assert_parser_output!(
            XGeoProperty::parse_ical("X-GEO;DIST=1.5KM:48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XGeoProperty {
                    params: XGeoPropertyParams { dist: DistValue::Kilometers(Float(1.5_f64)) },
                    latitude: Float(48.85299_f64),
                    longitude: Float(2.36885_f64),
                },
            ),
        );

        assert_parser_output!(
            XGeoProperty::parse_ical("X-GEO;DIST=30MI:48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XGeoProperty {
                    params: XGeoPropertyParams { dist: DistValue::Miles(Float(30.0_f64)) },
                    latitude: Float(48.85299_f64),
                    longitude: Float(2.36885_f64),
                },
            ),
        );

        assert!(XGeoProperty::parse_ical("X-GEO:ssokko".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XGeoProperty {
                params: XGeoPropertyParams { dist: DistValue::Kilometers(Float(1.5_f64)) },
                latitude: Float(48.85299_f64),
                longitude: Float(2.36885_f64),
            }.render_ical(),
            String::from("X-GEO;DIST=1.5KM:48.85299;2.36885"),
        );

        assert_eq!(
            XGeoProperty {
                params: XGeoPropertyParams { dist: DistValue::Miles(Float(30.0_f64)) },
                latitude: Float(48.85299_f64),
                longitude: Float(2.36885_f64),
            }.render_ical(),
            String::from("X-GEO;DIST=30MI:48.85299;2.36885"),
        );
    }
}
