use nom::branch::alt;
use nom::error::context;
use nom::sequence::{preceded, tuple};
use nom::combinator::{map, cut};
use crate::grammar::{tag, colon, semicolon};

use crate::properties::ICalendarProperty;
use crate::values::float::Float;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

/// Query order property, orders by:
/// * DTSTART
/// * DTSTART / GEO-DIST
/// * GEO-DIST / DTSTART
///
/// Example:
///
// X-ORDER-BY:DTSTART
// X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885
// X-ORDER-BY:GEO-DIST-DTSTART;48.85299;2.36885
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum XOrderByProperty {
    DTStart,
    DTStartGeoDist(Float, Float),
    GeoDistDTStart(Float, Float),
}

impl ICalendarEntity for XOrderByProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-ORDER-BY",
            preceded(
                tag("X-ORDER-BY"),
                cut(
                    preceded(
                        colon,
                        alt((
                            map(
                                preceded(tag("GEO-DIST-DTSTART"), cut(tuple((semicolon, Float::parse_ical, semicolon, Float::parse_ical)))),
                                |(_, latitude, _, longitude)| XOrderByProperty::GeoDistDTStart(latitude, longitude)
                            ),
                            map(
                                preceded(tag("DTSTART-GEO-DIST"), cut(tuple((semicolon, Float::parse_ical, semicolon, Float::parse_ical)))),
                                |(_, latitude, _, longitude)| XOrderByProperty::DTStartGeoDist(latitude, longitude)
                            ),
                            map(
                                tag("DTSTART"),
                                |_| XOrderByProperty::DTStart
                            ),
                        ))
                    )
                )
            )
        )(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_with_context(context).render_ical()
    }
}

impl ICalendarProperty for XOrderByProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        match self {
            Self::DTStart => {
                ContentLine::from(("X-ORDER-BY", (ContentLineParams::default(), String::from("DTSTART"))))
            },

            Self::DTStartGeoDist(latitude, longitude) => {
                ContentLine::from((
                    "X-ORDER-BY",
                    (
                        ContentLineParams::default(),
                        format!("DTSTART-GEO-DIST;{};{}", latitude.render_ical(), longitude.render_ical())
                    )
                ))
            },

            Self::GeoDistDTStart(latitude, longitude) => {
                ContentLine::from((
                    "X-ORDER-BY",
                    (
                        ContentLineParams::default(),
                        format!("GEO-DIST-DTSTART;{};{}", latitude.render_ical(), longitude.render_ical())
                    )
                ))
            },
        }
    }
}

impl std::hash::Hash for XOrderByProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XOrderByProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XOrderByProperty::parse_ical("X-ORDER-BY:DTSTART DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XOrderByProperty::DTStart,
            ),
        );

        assert_parser_output!(
            XOrderByProperty::parse_ical("X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XOrderByProperty::DTStartGeoDist(Float(48.85299_f64), Float(2.36885_f64)),
            ),
        );

        assert_parser_output!(
            XOrderByProperty::parse_ical("X-ORDER-BY:GEO-DIST-DTSTART;48.85299;2.36885 DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XOrderByProperty::GeoDistDTStart(Float(48.85299_f64), Float(2.36885_f64)),
            ),
        );

        assert!(XOrderByProperty::parse_ical("X-ORDER-BY:DTSTART-GEO-DIST".into()).is_err());
        assert!(XOrderByProperty::parse_ical("X-ORDER-BY:GEO-DIST-DTSTART".into()).is_err());
        assert!(XOrderByProperty::parse_ical("X-ORDER-BY:DTSTART-GEO-DIST;48.85299".into()).is_err());
        assert!(XOrderByProperty::parse_ical("X-ORDER-BY:GEO-DIST-DTSTART;48.85299;".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XOrderByProperty::DTStart.render_ical(),
            String::from("X-ORDER-BY:DTSTART"),
        );

        assert_eq!(
            XOrderByProperty::DTStartGeoDist(Float(48.85299_f64), Float(2.36885_f64)).render_ical(),
            String::from("X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885"),
        );

        assert_eq!(
            XOrderByProperty::GeoDistDTStart(Float(48.85299_f64), Float(2.36885_f64)).render_ical(),
            String::from("X-ORDER-BY:GEO-DIST-DTSTART;48.85299;2.36885"),
        );
    }
}
