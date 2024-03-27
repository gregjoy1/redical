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
    MetParams,
    MetParam,
    "METHOD",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Method
//
// Property Name:  METHOD
//
// Purpose:  This property defines the iCalendar object method
//    associated with the calendar object.
//
// Value Type:  TEXT
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     method     = "METHOD" metparam ":" metvalue CRLF
//
//     metparam   = *(";" other-param)
//
//     metvalue   = iana-token
//
// Example:  The following is a hypothetical example of this property to
//    convey that the iCalendar object is a scheduling request:
//
//     METHOD:REQUEST
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Method {
    pub params: MetParams,
    pub value: Text,
}

impl ICalendarEntity for Method {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "METHOD",
            preceded(
                tag("METHOD"),
                cut(
                    map(
                        pair(
                            opt(MetParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Method {
                                params: params.unwrap_or(MetParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("METHOD{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Method);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Method::parse_ical("METHOD:REQUEST".into()),
            (
                "",
                Method {
                    params: MetParams::default(),
                    value: Text(String::from("REQUEST")),
                },
            ),
        );

        assert_parser_output!(
            Method::parse_ical("METHOD;X-TEST=X_VALUE;TEST=VALUE:REQUEST".into()),
            (
                "",
                Method {
                    params: MetParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("REQUEST")),
                },
            ),
        );

        assert!(Method::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Method {
                params: MetParams::default(),
                value: Text(String::from("REQUEST")),
            }.render_ical(),
            String::from("METHOD:REQUEST"),
        );

        assert_eq!(
            Method {
                params: MetParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("REQUEST")),
            }.render_ical(),
            String::from("METHOD;X-TEST=X_VALUE;TEST=VALUE:REQUEST"),
        );
    }
}
