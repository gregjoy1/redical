use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_parameters::{
    value::ValueParam,
    tzid::TzidParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::date_time::DateTime;

use crate::properties::define_property_params;

define_property_params!(
    DueParams,
    DueParam,
    "DUEPARAM",
    (Value, ValueParam, value, Option<ValueParam>),
    (Tzid, TzidParam, tzid, Option<TzidParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Date-Time Due
//
// Property Name:  DUE
//
// Purpose:  This property defines the date and time that a to-do is
//    expected to be completed.
//
// Value Type:  The default value type is DATE-TIME.  The value type can
//    be set to a DATE value type.
//
// Property Parameters:  IANA, non-standard, value data type, and time
//    zone identifier property parameters can be specified on this
//    property.
//
// Conformance:  The property can be specified once in a "VTODO"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     due        = "DUE" dueparam ":" dueval CRLF
//
//     dueparam   = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" "VALUE" "=" ("DATE-TIME" / "DATE")) /
//                (";" tzidparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//     dueval     = date-time / date
//     ;Value MUST match value type
//
// Example:  The following is an example of this property:
//
//     DUE:19980430T000000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Completed {
    pub params: DueParams,
    pub value: DateTime,
}

impl ICalendarEntity for Completed {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DUE",
            preceded(
                tag("DUE"),
                cut(
                    map(
                        pair(
                            opt(DueParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            Completed {
                                params: params.unwrap_or(DueParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DUE{}:{}", self.params.render_ical(), self.value.render_ical())
    }

    fn validate(&self) -> Result<(), String> {
        self.value.validate()?;

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;
        };

        if let Some(value) = self.params.value.as_ref() {
            value.validate_against_date_time(&self.value)?;
        }

        Ok(())
    }
}

impl_icalendar_entity_traits!(Completed);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::value::Value;

    use crate::property_value_data_types::{
        date::Date,
        time::Time,
    };

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Completed::parse_ical("DUE:19960401T150000Z".into()),
            (
                "",
                Completed {
                    params: DueParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            Completed::parse_ical("DUE;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                Completed {
                    params: DueParams {
                        value: None,
                        tzid: Some(TzidParam(String::from("Europe/London"))),
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            Completed::parse_ical("DUE;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                Completed {
                    params: DueParams {
                        value: Some(ValueParam(Value::Date)),
                        tzid: None,
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
                params: DueParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("DUE:19960401T150000Z"),
        );

        assert_eq!(
            Completed {
                params: DueParams {
                    value: None,
                    tzid: Some(TzidParam(String::from("Europe/London"))),
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("DUE;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            Completed {
                params: DueParams {
                    value: Some(ValueParam(Value::Date)),
                    tzid: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("DUE;VALUE=DATE;X-TEST=X_VALUE;TEST=VALUE:19960401"),
        );
    }
}
