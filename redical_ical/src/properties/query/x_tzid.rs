use nom::error::context;
use nom::sequence::preceded;
use nom::combinator::{map, cut};

use crate::grammar::{tag, colon};

use crate::properties::ICalendarProperty;
use crate::value_data_types::tzid::Tzid;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

/// Query distinct property, groups by the specified value.
///
/// Currently only UID value is available.
///
/// Example:
///
/// X-TZID:UID
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XTzidProperty {
    pub tzid: Tzid,
}

impl ICalendarEntity for XTzidProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-TZID",
            preceded(
                tag("X-TZID"),
                cut(
                    map(
                        preceded(colon, Tzid::parse_ical),
                        |tzid| {
                            XTzidProperty { tzid }
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

impl ICalendarProperty for XTzidProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "X-TZID",
            (
                ContentLineParams::default(),
                self.tzid.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for XTzidProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XTzidProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XTzidProperty::parse_ical("X-TZID:UTC DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XTzidProperty {
                    tzid: Tzid(chrono_tz::Tz::UTC),
                },
            ),
        );

        assert_parser_output!(
            XTzidProperty::parse_ical("X-TZID:Europe/London DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XTzidProperty {
                    tzid: Tzid(chrono_tz::Tz::Europe__London),
                },
            ),
        );

        assert!(XTzidProperty::parse_ical("X-TZID:ssokko".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XTzidProperty {
                tzid: Tzid(chrono_tz::Tz::UTC),
            }.render_ical(),
            String::from("X-TZID:UTC"),
        );

        assert_eq!(
            XTzidProperty {
                tzid: Tzid(chrono_tz::Tz::Europe__London),
            }.render_ical(),
            String::from("X-TZID:Europe/London"),
        );
    }
}
