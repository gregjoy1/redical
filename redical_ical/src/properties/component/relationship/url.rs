use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::uri::Uri;
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    UrlParams,
    UrlParam,
    "URLPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Uniform Resource Locator
//
// Property Name:  URL
//
// Purpose:  This property defines a Uniform Resource Locator (URL)
//    associated with the iCalendar object.
//
// Value Type:  URI
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified once in the "VEVENT",
//    "VTODO", "VJOURNAL", or "VFREEBUSY" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     url        = "URL" urlparam ":" uri CRLF
//
//     urlparam   = *(";" other-param)
//
// Example:  The following is an example of this property:
//
//     URL:http://example.com/pub/calendars/jsmith/mytime.ics
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Url {
    pub params: UrlParams,
    pub value: Uri,
}

impl ICalendarEntity for Url {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "URL",
            preceded(
                tag("URL"),
                cut(
                    map(
                        pair(
                            opt(UrlParams::parse_ical),
                            preceded(colon, Uri::parse_ical),
                        ),
                        |(params, value)| {
                            Url {
                                params: params.unwrap_or(UrlParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("URL{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Url);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Url::parse_ical(
                "URL:http://example.com/pub/calendars/jsmith/mytime.ics".into()
            ),
            (
                "",
                Url {
                    params: UrlParams::default(),
                    value: Uri(String::from("http://example.com/pub/calendars/jsmith/mytime.ics")),
                },
            ),
        );

        assert_parser_output!(
            Url::parse_ical("URL;X-TEST=X_VALUE;TEST=VALUE:http://example.com/pub/calendars/jsmith/mytime.ics".into()),
            (
                "",
                Url {
                    params: UrlParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Uri(String::from("http://example.com/pub/calendars/jsmith/mytime.ics")),
                },
            ),
        );

        assert!(Url::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Url {
                params: UrlParams::default(),
                value: Uri(String::from("http://example.com/pub/calendars/jsmith/mytime.ics")),
            }.render_ical(),
            String::from("URL:http://example.com/pub/calendars/jsmith/mytime.ics"),
        );

        assert_eq!(
            Url {
                params: UrlParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Uri(String::from("http://example.com/pub/calendars/jsmith/mytime.ics")),
            }.render_ical(),
            String::from("URL;X-TEST=X_VALUE;TEST=VALUE:http://example.com/pub/calendars/jsmith/mytime.ics"),
        );
    }
}
