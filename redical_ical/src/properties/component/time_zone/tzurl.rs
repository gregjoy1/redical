use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::bytes::complete::tag;
use nom::multi::fold_many0;
use nom::combinator::{opt, map, cut};

use crate::grammar::{colon, semicolon};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::property_value_data_types::uri::Uri;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    TzurlParams,
    TzurlParam,
    "TZURLPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Zone URL
//
// Property Name:  TZURL
//
// Purpose:  This property provides a means for a "VTIMEZONE" component
//    to point to a network location that can be used to retrieve an up-
//    to-date version of itself.
//
// Value Type:  URI
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in a "VTIMEZONE"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     tzurl      = "TZURL" tzurlparam ":" uri CRLF
//
//     tzurlparam = *(";" other-param)
//
// Example:  The following is an example of this property:
//
//  TZURL:http://timezones.example.org/tz/America-Los_Angeles.ics
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzurl {
    pub params: TzurlParams,
    pub value: Uri,
}

impl ICalendarEntity for Tzurl {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZURL",
            preceded(
                tag("TZURL"),
                cut(
                    map(
                        pair(
                            opt(TzurlParams::parse_ical),
                            preceded(
                                colon,
                                Uri::parse_ical,
                            )
                        ),
                        |(params, value)| {
                            Tzurl {
                                params: params.unwrap_or(TzurlParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZURL{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Tzurl);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzurl::parse_ical("TZURL:http://timezones.example.org/tz/America-Los_Angeles.ics TESTING".into()),
            (
                " TESTING",
                Tzurl {
                    params: TzurlParams::default(),
                    value: Uri(String::from("http://timezones.example.org/tz/America-Los_Angeles.ics")),
                },
            )
        );

        assert_parser_output!(
            Tzurl::parse_ical("TZURL;X-TEST=X_VALUE;TEST=VALUE:http://timezones.example.org/tz/America-Los_Angeles.ics TESTING".into()),
            (
                " TESTING",
                Tzurl {
                    params: TzurlParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Uri(String::from("http://timezones.example.org/tz/America-Los_Angeles.ics")),
                },
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzurl {
                params: TzurlParams::default(),
                value: Uri(String::from("http://timezones.example.org/tz/America-Los_Angeles.ics")),
            }.render_ical(),
            String::from("TZURL:http://timezones.example.org/tz/America-Los_Angeles.ics"),
        );

        assert_eq!(
            Tzurl {
                params: TzurlParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Uri(String::from("http://timezones.example.org/tz/America-Los_Angeles.ics")),
            }.render_ical(),
            String::from("TZURL;X-TEST=X_VALUE;TEST=VALUE:http://timezones.example.org/tz/America-Los_Angeles.ics"),
        );
    }
}
