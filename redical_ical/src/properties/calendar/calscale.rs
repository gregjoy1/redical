use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    CalParams,
    CalParam,
    "CALSCALE",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Calendar Scale
//
// Property Name:  CALSCALE
//
// Purpose:  This property defines the calendar scale used for the
//    calendar information specified in the iCalendar object.
//
// Value Type:  TEXT
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     calscale   = "CALSCALE" calparam ":" calvalue CRLF
//
//     calparam   = *(";" other-param)
//
//     calvalue   = "GREGORIAN"
//
// Example:  The following is an example of this property:
//
//     CALSCALE:GREGORIAN
//
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Calscale {
    pub params: CalParams,
    pub value: Text,
}

impl ICalendarEntity for Calscale {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CALSCALE",
            preceded(
                tag("CALSCALE"),
                cut(
                    map(
                        pair(
                            opt(CalParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Calscale {
                                params: params.unwrap_or(CalParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CALSCALE{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Calscale);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Calscale::parse_ical("CALSCALE:GREGORIAN".into()),
            (
                "",
                Calscale {
                    params: CalParams::default(),
                    value: Text(String::from("GREGORIAN")),
                },
            ),
        );

        assert_parser_output!(
            Calscale::parse_ical("CALSCALE;X-TEST=X_VALUE;TEST=VALUE:GREGORIAN".into()),
            (
                "",
                Calscale {
                    params: CalParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("GREGORIAN")),
                },
            ),
        );

        assert!(Calscale::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Calscale {
                params: CalParams::default(),
                value: Text(String::from("GREGORIAN")),
            }.render_ical(),
            String::from("CALSCALE:GREGORIAN"),
        );

        assert_eq!(
            Calscale {
                params: CalParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("GREGORIAN")),
            }.render_ical(),
            String::from("CALSCALE;X-TEST=X_VALUE;TEST=VALUE:GREGORIAN"),
        );
    }
}
