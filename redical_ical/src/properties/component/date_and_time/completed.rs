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

use crate::property_value_data_types::date_time::DateTime;

use crate::properties::define_property_params;

define_property_params!(
    CompParams,
    CompParam,
    "COMPPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Date-Time Completed
//
// Property Name:  COMPLETED
//
// Purpose:  This property defines the date and time that a to-do was
//    actually completed.
//
// Value Type:  DATE-TIME
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  The property can be specified in a "VTODO" calendar
//    component.  The value MUST be specified as a date with UTC time.
//
// Description:  This property defines the date and time that a to-do
//    was actually completed.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     completed  = "COMPLETED" compparam ":" date-time CRLF
//
//     compparam  = *(";" other-param)
//
// Example:  The following is an example of this property:
//
//  COMPLETED:19960401T150000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Completed {
    pub params: CompParams,
    pub value: DateTime,
}

impl ICalendarEntity for Completed {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "COMPLETED",
            preceded(
                tag("COMPLETED"),
                cut(
                    map(
                        pair(
                            opt(CompParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            Completed {
                                params: params.unwrap_or(CompParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("COMPLETED{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Completed);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::{
        date::Date,
        time::Time,
    };

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Completed::parse_ical("COMPLETED:19960401T150000Z".into()),
            (
                "",
                Completed {
                    params: CompParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            Completed::parse_ical("COMPLETED:19960401T150000".into()),
            (
                "",
                Completed {
                    params: CompParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            Completed::parse_ical("COMPLETED;X-TEST=X_VALUE;TEST=VALUE:19960401".into()),
            (
                "",
                Completed {
                    params: CompParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
                },
            ),
        );

        assert!(Completed::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Completed {
                params: CompParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("COMPLETED:19960401T150000Z"),
        );

        assert_eq!(
            Completed {
                params: CompParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("COMPLETED:19960401T150000"),
        );

        assert_eq!(
            Completed {
                params: CompParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("COMPLETED;X-TEST=X_VALUE;TEST=VALUE:19960401"),
        );
    }
}
