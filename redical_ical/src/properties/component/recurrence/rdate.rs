use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon, List};
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
    Rdtparams,
    Rdtparam,
    "RDTPARAM",
    (Value, ValueParam, value, Option<ValueParam>),
    (Tzid, TzidParam, tzid, Option<TzidParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Recurrence Date-Times
//
// Property Name:  RDATE
//
// Purpose:  This property defines the list of DATE-TIME values for
//    recurring events, to-dos, journal entries, or time zone
//    definitions.
//
// Value Type:  The default value type for this property is DATE-TIME.
//    The value type can be set to DATE or PERIOD.
//
// Property Parameters:  IANA, non-standard, value data type, and time
//    zone identifier property parameters can be specified on this
//    property.
//
// Conformance:  This property can be specified in recurring "VEVENT",
//    "VTODO", and "VJOURNAL" calendar components as well as in the
//    "STANDARD" and "DAYLIGHT" sub-components of the "VTIMEZONE"
//    calendar component.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     rdate      = "RDATE" rdtparam ":" rdtval *("," rdtval) CRLF
//
//     rdtparam   = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" "VALUE" "=" ("DATE-TIME" / "DATE" / "PERIOD")) /
//                (";" tzidparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//     rdtval     = date-time / date / period
//     ;Value MUST match value type
//
// Example:  The following are examples of this property:
//
//     RDATE:19970714T123000Z
//     RDATE;TZID=America/New_York:19970714T083000
//
//     RDATE;VALUE=PERIOD:19960403T020000Z/19960403T040000Z,
//      19960404T010000Z/PT3H
//
//     RDATE;VALUE=DATE:19970101,19970120,19970217,19970421
//      19970526,19970704,19970901,19971014,19971128,19971129,19971225
//
// TODO: Implement PERIOD VALUE type.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Rdate {
    pub params: Rdtparams,
    pub value: List<DateTime>,
}

impl ICalendarEntity for Rdate {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RDATE",
            preceded(
                tag("RDATE"),
                cut(
                    map(
                        pair(
                            opt(Rdtparams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, value)| {
                            Rdate {
                                params: params.unwrap_or(Rdtparams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RDATE{}:{}", self.params.render_ical(), self.value.render_ical())
    }

    fn validate(&self) -> Result<(), String> {
        self.value.validate()?;

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;
        };

        if let Some(value) = self.params.value.as_ref() {
            for datetime in self.value.iter() {
                value.validate_against_date_time(&datetime)?;
            }
        }

        Ok(())
    }
}

impl_icalendar_entity_traits!(Rdate);

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
            Rdate::parse_ical("RDATE:19960401T150000Z,19960401T180000Z,19960401T200000Z".into()),
            (
                "",
                Rdate {
                    params: Rdtparams::default(),
                    value: List::from(
                        vec![
                            DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                            DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 18_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                            DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 20_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                        ]
                    ),
                },
            ),
        );

        assert_parser_output!(
            Rdate::parse_ical("RDATE;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                Rdate {
                    params: Rdtparams {
                        value: None,
                        tzid: Some(TzidParam(String::from("Europe/London"))),
                        iana: IanaParams::default(),
                        x: XParams::default(),
                    },
                    value: List::from(
                        vec![
                            DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) }
                        ]
                    ),
                },
            ),
        );

        assert_parser_output!(
            Rdate::parse_ical("RDATE;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                Rdate {
                    params: Rdtparams {
                        value: Some(ValueParam(Value::Date)),
                        tzid: None,
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: List::from(
                        vec![
                            DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None }
                        ]
                    ),
                },
            ),
        );

        assert!(Rdate::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Rdate {
                params: Rdtparams::default(),
                value: List::from(
                    vec![
                        DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                        DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 18_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                        DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 20_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                    ]
                ),
            }.render_ical(),
            String::from("RDATE:19960401T150000Z,19960401T180000Z,19960401T200000Z"),
        );

        assert_eq!(
            Rdate {
                params: Rdtparams {
                    value: None,
                    tzid: Some(TzidParam(String::from("Europe/London"))),
                    iana: IanaParams::default(),
                    x: XParams::default(),
                },
                value: List::from(
                    vec![
                        DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) }
                    ]
                ),
            }.render_ical(),
            String::from("RDATE;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            Rdate {
                params: Rdtparams {
                    value: Some(ValueParam(Value::Date)),
                    tzid: None,
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: List::from(
                    vec![
                        DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None }
                    ]
                ),
            }.render_ical(),
            String::from("RDATE;VALUE=DATE;X-TEST=X_VALUE;TEST=VALUE:19960401"),
        );
    }
}
