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
    PidParams,
    PidParam,
    "PRODID",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Product Identifier
//
// Property Name:  PRODID
//
// Purpose:  This property specifies the identifier for the product that
//    created the iCalendar object.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  The property MUST be specified once in an iCalendar
//    object.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     prodid     = "PRODID" pidparam ":" pidvalue CRLF
//
//     pidparam   = *(";" other-param)
//
//     pidvalue   = text
//     ;Any text that describes the product and version
//     ;and that is generally assured of being unique.
//
// Example:  The following is an example of this property.  It does not
//    imply that English is the default language.
//
//     PRODID:-//ABC Corporation//NONSGML My Product//EN
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Prodid {
    pub params: PidParams,
    pub value: Text,
}

impl ICalendarEntity for Prodid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PRODID",
            preceded(
                tag("PRODID"),
                cut(
                    map(
                        pair(
                            opt(PidParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Prodid {
                                params: params.unwrap_or(PidParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("PRODID{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Prodid);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Prodid::parse_ical("PRODID:-//ABC Corporation//NONSGML My Product//EN".into()),
            (
                "",
                Prodid {
                    params: PidParams::default(),
                    value: Text(String::from("-//ABC Corporation//NONSGML My Product//EN")),
                },
            ),
        );

        assert_parser_output!(
            Prodid::parse_ical("PRODID;X-TEST=X_VALUE;TEST=VALUE:-//ABC Corporation//NONSGML My Product//EN".into()),
            (
                "",
                Prodid {
                    params: PidParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("-//ABC Corporation//NONSGML My Product//EN")),
                },
            ),
        );

        assert!(Prodid::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Prodid {
                params: PidParams::default(),
                value: Text(String::from("-//ABC Corporation//NONSGML My Product//EN")),
            }.render_ical(),
            String::from("PRODID:-//ABC Corporation//NONSGML My Product//EN"),
        );

        assert_eq!(
            Prodid {
                params: PidParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("-//ABC Corporation//NONSGML My Product//EN")),
            }.render_ical(),
            String::from("PRODID;X-TEST=X_VALUE;TEST=VALUE:-//ABC Corporation//NONSGML My Product//EN"),
        );
    }
}
