use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};

use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TransValue {
    Opaque,
    Transparent,
}

impl ICalendarEntity for TransValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TRANSVALUE",
            alt((
                map(tag("OPAQUE"), |_| TransValue::Opaque),
                map(tag("TRANSPARENT"), |_| TransValue::Transparent),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Opaque => String::from("OPAQUE"),
            Self::Transparent => String::from("TRANSPARENT"),
        }
    }
}

impl_icalendar_entity_traits!(TransValue);

define_property_params!(
    TransParams,
    TransParam,
    "TRANSPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Transparency
//
// Property Name:  TRANSP
//
// Purpose:  This property defines whether or not an event is
//    transparent to busy time searches.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified once in a "VEVENT"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     transp     = "TRANSP" transparam ":" transvalue CRLF
//
//     transparam = *(";" other-param)
//
//     transvalue = "OPAQUE"
//                 ;Blocks or opaque on busy time searches.
//                 / "TRANSPARENT"
//                 ;Transparent on busy time searches.
//     ;Default value is OPAQUE
//
// Example:  The following is an example of this property for an event
//    that is transparent or does not block on free/busy time searches:
//
//     TRANSP:TRANSPARENT
//
//    The following is an example of this property for an event that is
//    opaque or blocks on free/busy time searches:
//
//     TRANSP:OPAQUE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Transp {
    pub params: TransParams,
    pub value: TransValue,
}

impl ICalendarEntity for Transp {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TRANSP",
            preceded(
                tag("TRANSP"),
                cut(
                    map(
                        pair(
                            opt(TransParams::parse_ical),
                            preceded(
                                colon,
                                TransValue::parse_ical,
                            ),
                        ),
                        |(params, value)| {
                            Transp {
                                params: params.unwrap_or(TransParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TRANSP{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Transp);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Transp::parse_ical("TRANSP:OPAQUE".into()),
            (
                "",
                Transp {
                    params: TransParams::default(),
                    value: TransValue::Opaque,
                },
            ),
        );

        assert_parser_output!(
            Transp::parse_ical("TRANSP;X-TEST=X_VALUE;TEST=VALUE:TRANSPARENT".into()),
            (
                "",
                Transp {
                    params: TransParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: TransValue::Transparent,
                },
            ),
        );

        assert!(Transp::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Transp {
                params: TransParams::default(),
                value: TransValue::Opaque,
            }.render_ical(),
            String::from("TRANSP:OPAQUE"),
        );

        assert_eq!(
            Transp {
                params: TransParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: TransValue::Transparent,
            }.render_ical(),
            String::from("TRANSP;X-TEST=X_VALUE;TEST=VALUE:TRANSPARENT"),
        );
    }
}
