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
    range::RangeParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::date_time::DateTime;

use crate::properties::define_property_params;

define_property_params!(
    RidParams,
    RidParam,
    "RIDPARAM",
    (Value, ValueParam, value, Option<ValueParam>),
    (Tzid, TzidParam, tzid, Option<TzidParam>),
    (Range, RangeParam, range, Option<RangeParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Recurrence ID
//
// Property Name:  RECURRENCE-ID
//
// Purpose:  This property is used in conjunction with the "UID" and
//    "SEQUENCE" properties to identify a specific instance of a
//    recurring "VEVENT", "VTODO", or "VJOURNAL" calendar component.
//    The property value is the original value of the "DTSTART" property
//    of the recurrence instance.
//
// Value Type:  The default value type is DATE-TIME.  The value type can
//    be set to a DATE value type.  This property MUST have the same
//    value type as the "DTSTART" property contained within the
//    recurring component.  Furthermore, this property MUST be specified
//    as a date with local time if and only if the "DTSTART" property
//    contained within the recurring component is specified as a date
//    with local time.
//
// Property Parameters:  IANA, non-standard, value data type, time zone
//    identifier, and recurrence identifier range parameters can be
//    specified on this property.
//
// Conformance:  This property can be specified in an iCalendar object
//    containing a recurring calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     recurid    = "RECURRENCE-ID" ridparam ":" ridval CRLF
//
//     ridparam   = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" "VALUE" "=" ("DATE-TIME" / "DATE")) /
//                (";" tzidparam) / (";" rangeparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//     ridval     = date-time / date
//     ;Value MUST match value type
//
// Example:  The following are examples of this property:
//
//     RECURRENCE-ID;VALUE=DATE:19960401
//
//     RECURRENCE-ID;RANGE=THISANDFUTURE:19960120T120000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dtstart {
    pub params: RidParams,
    pub value: DateTime,
}

impl ICalendarEntity for Dtstart {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RECURRENCE-ID",
            preceded(
                tag("RECURRENCE-ID"),
                cut(
                    map(
                        pair(
                            opt(RidParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            Dtstart {
                                params: params.unwrap_or(RidParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RECURRENCE-ID{}:{}", self.params.render_ical(), self.value.render_ical())
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

impl_icalendar_entity_traits!(Dtstart);

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
            Dtstart::parse_ical("RECURRENCE-ID:19960401T150000Z".into()),
            (
                "",
                Dtstart {
                    params: RidParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            Dtstart::parse_ical("RECURRENCE-ID;TZID=Europe/London;RANGE=THISANDFUTURE:19960401T150000".into()),
            (
                "",
                Dtstart {
                    params: RidParams {
                        value: None,
                        tzid: Some(TzidParam(String::from("Europe/London"))),
                        range: Some(RangeParam(())),
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            Dtstart::parse_ical("RECURRENCE-ID;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                Dtstart {
                    params: RidParams {
                        value: Some(ValueParam(Value::Date)),
                        tzid: None,
                        range: None,
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
                },
            ),
        );

        assert!(Dtstart::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Dtstart {
                params: RidParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("RECURRENCE-ID:19960401T150000Z"),
        );

        assert_eq!(
            Dtstart {
                params: RidParams {
                    value: None,
                    tzid: Some(TzidParam(String::from("Europe/London"))),
                    range: Some(RangeParam(())),
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("RECURRENCE-ID;TZID=Europe/London;RANGE=THISANDFUTURE:19960401T150000"),
        );

        assert_eq!(
            Dtstart {
                params: RidParams {
                    value: Some(ValueParam(Value::Date)),
                    tzid: None,
                    range: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("RECURRENCE-ID;VALUE=DATE;X-TEST=X_VALUE;TEST=VALUE:19960401"),
        );
    }
}
