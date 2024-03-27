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
    PrioParams,
    PrioParam,
    "PRIOPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Priority
//
// Property Name:  PRIORITY
//
// Purpose:  This property defines the relative priority for a calendar
//    component.
//
// Value Type:  INTEGER
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in "VEVENT" and "VTODO"
//    calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     priority   = "PRIORITY" prioparam ":" priovalue CRLF
//     ;Default is zero (i.e., undefined).
//
//     prioparam  = *(";" other-param)
//
//     priovalue   = integer       ;Must be in the range [0..9]
//        ; All other values are reserved for future use.
//
//     PRIORITY:1
//
//    The following is an example of a property with a next highest
//    priority:
//
//     PRIORITY:2
//
//    The following is an example of a property with no priority.  This
//    is equivalent to not specifying the "PRIORITY" property:
//
//     PRIORITY:0
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Percent {
    pub params: PrioParams,
    pub value: Integer,
}

impl ICalendarEntity for Percent {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PRIORITY",
            preceded(
                tag("PRIORITY"),
                cut(
                    map(
                        pair(
                            opt(PrioParams::parse_ical),
                            preceded(colon, Integer::parse_ical),
                        ),
                        |(params, value)| {
                            Percent {
                                params: params.unwrap_or(PrioParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("PRIORITY{}:{}", self.params.render_ical(), self.value.render_ical())
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
            Percent::parse_ical("PRIORITY:9".into()),
            (
                "",
                Percent {
                    params: PrioParams::default(),
                    value: Integer(9_i64),
                },
            ),
        );

        assert_parser_output!(
            Percent::parse_ical("PRIORITY;X-TEST=X_VALUE;TEST=VALUE:2".into()),
            (
                "",
                Percent {
                    params: PrioParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Integer(2_i64),
                },
            ),
        );

        assert!(Percent::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Percent {
                params: PrioParams::default(),
                value: Integer(2_i64),
            }.render_ical(),
            String::from("PRIORITY:2"),
        );

        assert_eq!(
            Percent {
                params: PrioParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Integer(4_i64),
            }.render_ical(),
            String::from("PRIORITY;X-TEST=X_VALUE;TEST=VALUE:4"),
        );
    }
}
