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

use crate::property_value_data_types::integer::Integer;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    PctParams,
    PctParam,
    "PCTPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Percent Complete
//
// Property Name:  PERCENT-COMPLETE
//
// Purpose:  This property is used by an assignee or delegatee of a
//    to-do to convey the percent completion of a to-do to the
//    "Organizer".
//
// Value Type:  INTEGER
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified once in a "VTODO"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     percent = "PERCENT-COMPLETE" pctparam ":" integer CRLF
//
//     pctparam   = *(";" other-param)
//
// Example:  The following is an example of this property to show 39%
//    completion:
//
//     PERCENT-COMPLETE:39
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Percent {
    pub params: PctParams,
    pub value: Integer,
}

impl ICalendarEntity for Percent {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PERCENT-COMPLETE",
            preceded(
                tag("PERCENT-COMPLETE"),
                cut(
                    map(
                        pair(
                            opt(PctParams::parse_ical),
                            preceded(colon, Integer::parse_ical),
                        ),
                        |(params, value)| {
                            Percent {
                                params: params.unwrap_or(PctParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("PERCENT-COMPLETE{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Percent);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Percent::parse_ical("PERCENT-COMPLETE:39".into()),
            (
                "",
                Percent {
                    params: PctParams::default(),
                    value: Integer(39_i64),
                },
            ),
        );

        assert_parser_output!(
            Percent::parse_ical("PERCENT-COMPLETE;X-TEST=X_VALUE;TEST=VALUE:22".into()),
            (
                "",
                Percent {
                    params: PctParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Integer(22_i64),
                },
            ),
        );

        assert!(Percent::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Percent {
                params: PctParams::default(),
                value: Integer(22_i64),
            }.render_ical(),
            String::from("PERCENT-COMPLETE:22"),
        );

        assert_eq!(
            Percent {
                params: PctParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Integer(44_i64),
            }.render_ical(),
            String::from("PERCENT-COMPLETE;X-TEST=X_VALUE;TEST=VALUE:44"),
        );
    }
}
