use chrono_tz::Tz;

use nom::error::context;
use nom::sequence::pair;
use nom::combinator::{opt, map_res, recognize};
use nom::bytes::complete::take_while1;

use crate::grammar::{is_safe_char, is_wsp_char, solidus};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzid(pub Tz);

impl ICalendarEntity for Tzid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZID",
            map_res(
                recognize(
                    pair(
                        opt(solidus),
                        // Small hack that allows paramtext chars except whitespace.
                        take_while1(|input: char| {
                            is_safe_char(input) && !is_wsp_char(input)
                        }),
                    )
                ),
                |tzid: ParserInput| {
                    if let Ok(tz) = tzid.to_string().parse::<Tz>() {
                        Ok(Self(tz))
                    } else {
                        Err(String::from("Timezone is invalid"))
                    }
                }
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        self.0.to_string()
    }
}

impl From<Tzid> for Tz {
    fn from(tzid: Tzid) -> Self {
        tzid.0.to_owned()
    }
}

impl From<&Tzid> for Tz {
    fn from(tzid: &Tzid) -> Self {
        Tz::from(tzid.to_owned())
    }
}

impl_icalendar_entity_traits!(Tzid);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzid::parse_ical("America/New_York TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::America__New_York),
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("Etc/GMT+12 TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::Etc__GMTPlus12),
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("UTC TESTING".into()),
            (
                " TESTING",
                Tzid(Tz::UTC),
            )
        );

        assert_parser_error!(
            Tzid::parse_ical("INVALID TESTING".into()),
            nom::Err::Error(
                span: "INVALID TESTING",
                message: "Timezone is invalid",
                context: ["TZID"],
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzid(Tz::America__New_York).render_ical(),
            String::from("America/New_York"),
        );
    }
}
