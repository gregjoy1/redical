use chrono_tz::Tz;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::bytes::complete::{tag, take_while1};
use nom::multi::fold_many0;
use nom::combinator::{opt, map, cut, recognize};

use crate::grammar::{is_safe_char, is_wsp_char, solidus, colon, semicolon};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::property_value_data_types::text::Text;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    TzidpropParams,
    TzidpropParam,
    "TZIDPROPPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Zone Identifier
//
// Property Name:  TZID
//
// Purpose:  This property specifies the text value that uniquely
//    identifies the "VTIMEZONE" calendar component in the scope of an
//    iCalendar object.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property MUST be specified in a "VTIMEZONE"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     tzid       = "TZID" tzidpropparam ":" [tzidprefix] text CRLF
//
//     tzidpropparam      = *(";" other-param)
//
//     ;tzidprefix        = "/"
//     ; Defined previously. Just listed here for reader convenience.
//
// Example:  The following are examples of non-globally unique time zone
//    identifiers:
//
//     TZID:America/New_York
//
//     TZID:America/Los_Angeles
//
//    The following is an example of a fictitious globally unique time
//    zone identifier:
//
//     TZID:/example.org/America/New_York
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzid {
    pub params: TzidpropParams,
    pub value: Text,
}

impl ICalendarEntity for Tzid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZID",
            preceded(
                tag("TZID"),
                cut(
                    map(
                        pair(
                            opt(TzidpropParams::parse_ical),
                            preceded(
                                colon,
                                recognize(
                                    pair(
                                        opt(solidus),
                                        // Small hack that allows paramtext chars except whitespace.
                                        take_while1(|input: char| {
                                            is_safe_char(input) && !is_wsp_char(input)
                                        }),
                                    )
                                ),
                            )
                        ),
                        |(params, value)| {
                            Tzid {
                                params: params.unwrap_or(TzidpropParams::default()),
                                value: Text(value.to_string()),
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZID{}:{}", self.params.render_ical(), self.value.render_ical())
    }

    fn validate(&self) -> Result<(), String> {
        if self.value.to_string().parse::<Tz>().is_err() {
            return Err(String::from("Timezone is invalid"))
        }

        Ok(())
    }
}

impl_icalendar_entity_traits!(Tzid);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzid::parse_ical("TZID:America/New_York TESTING".into()),
            (
                " TESTING",
                Tzid {
                    params: TzidpropParams::default(),
                    value: Text(String::from("America/New_York")),
                },
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("TZID:/America/New_York TESTING".into()),
            (
                " TESTING",
                Tzid {
                    params: TzidpropParams::default(),
                    value: Text(String::from("/America/New_York")),
                },
            )
        );

        assert_parser_output!(
            Tzid::parse_ical("TZID;X-TEST=X_VALUE;TEST=VALUE:Etc/GMT+12 TESTING".into()),
            (
                " TESTING",
                Tzid {
                    params: TzidpropParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("Etc/GMT+12")),
                },
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzid {
                params: TzidpropParams::default(),
                value: Text(String::from("America/New_York")),
            }.render_ical(),
            String::from("TZID:America/New_York"),
        );

        assert_eq!(
            Tzid {
                params: TzidpropParams::default(),
                value: Text(String::from("/America/New_York")),
            }.render_ical(),
            String::from("TZID:/America/New_York"),
        );

        assert_eq!(
            Tzid {
                params: TzidpropParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("Etc/GMT+12")),
            }.render_ical(),
            String::from("TZID;X-TEST=X_VALUE;TEST=VALUE:Etc/GMT+12"),
        );
    }
}
