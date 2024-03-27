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
    DtstParams,
    DtstParam,
    "DTSTARTPARAM",
    (Value, ValueParam, value, Option<ValueParam>),
    (Tzid, TzidParam, tzid, Option<TzidParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Date-Time Start
//
// Property Name:  DTSTART
//
// Purpose:  This property specifies when the calendar component begins.
//
// Value Type:  The default value type is DATE-TIME.  The time value
//    MUST be one of the forms defined for the DATE-TIME value type.
//    The value type can be set to a DATE value type.
//
// Property Parameters:  IANA, non-standard, value data type, and time
//    zone identifier property parameters can be specified on this
//    property.
//
// Conformance:  This property can be specified once in the "VEVENT",
//    "VTODO", or "VFREEBUSY" calendar components as well as in the
//    "STANDARD" and "DAYLIGHT" sub-components.  This property is
//    REQUIRED in all types of recurring calendar components that
//    specify the "RRULE" property.  This property is also REQUIRED in
//    "VEVENT" calendar components contained in iCalendar objects that
//    don't specify the "METHOD" property.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     dtstart    = "DTSTART" dtstparam ":" dtstval CRLF
//
//     dtstparam  = *(
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
//     dtstval    = date-time / date
//     ;Value MUST match value type
//
// Example:  The following is an example of this property:
//
//     DTSTART:19980118T073000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Dtstart {
    pub params: DtstParams,
    pub value: DateTime,
}

impl ICalendarEntity for Dtstart {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DTSTART",
            preceded(
                tag("DTSTART"),
                cut(
                    map(
                        pair(
                            opt(DtstParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            Dtstart {
                                params: params.unwrap_or(DtstParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DTSTART{}:{}", self.params.render_ical(), self.value.render_ical())
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
            Dtstart::parse_ical("DTSTART:19960401T150000Z".into()),
            (
                "",
                Dtstart {
                    params: DtstParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            Dtstart::parse_ical("DTSTART;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                Dtstart {
                    params: DtstParams {
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
            Dtstart::parse_ical("DTSTART;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                Dtstart {
                    params: DtstParams {
                        value: Some(ValueParam(Value::Date)),
                        tzid: None,
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
                params: DtstParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("DTSTART:19960401T150000Z"),
        );

        assert_eq!(
            Dtstart {
                params: DtstParams {
                    value: None,
                    tzid: Some(TzidParam(String::from("Europe/London"))),
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("DTSTART;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            Dtstart {
                params: DtstParams {
                    value: Some(ValueParam(Value::Date)),
                    tzid: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("DTSTART;VALUE=DATE;X-TEST=X_VALUE;TEST=VALUE:19960401"),
        );
    }
}
