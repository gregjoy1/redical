use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::property_value_data_types::date_time::DateTime;
use crate::property_parameters::tzid::{TzidParam, Tzid};
use crate::property_parameters::value_type::{ValueTypeParam, ValueType};

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RDatePropertyParams {
    pub tzid: Option<Tzid>,
    pub value_type: Option<ValueType>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for RDatePropertyParams {
    define_property_params_ical_parser!(
        RDatePropertyParams,
        (
            TzidParam::parse_ical,
            |params: &mut RDatePropertyParams, tzid_param: TzidParam| params.tzid = Some(tzid_param.0),
        ),
        (
            ValueTypeParam::parse_ical,
            |params: &mut RDatePropertyParams, value_param: ValueTypeParam| params.value_type = Some(value_param.0),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut RDatePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&RDatePropertyParams> for ContentLineParams {
    fn from(related_to_params: &RDatePropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in related_to_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(value_type) = related_to_params.value_type.as_ref() {
            content_line_params.insert(String::from("VALUE"), value_type.render_ical());
        }

        if let Some(tzid) = related_to_params.tzid.as_ref() {
            content_line_params.insert(String::from("TZID"), tzid.render_ical());
        }

        content_line_params
    }
}

impl From<RDatePropertyParams> for ContentLineParams {
    fn from(related_to_params: RDatePropertyParams) -> Self {
        ContentLineParams::from(&related_to_params)
    }
}

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
pub struct RDateProperty {
    pub params: RDatePropertyParams,
    pub value: DateTime,
}

impl ICalendarEntity for RDateProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RDATE",
            preceded(
                tag("RDATE"),
                cut(
                    map(
                        pair(
                            opt(RDatePropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            RDateProperty {
                                params: params.unwrap_or(RDatePropertyParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        ContentLine::from(self).render_ical()
    }

    fn validate(&self) -> Result<(), String> {
        self.value.validate()?;

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            value_type.validate_against_date_time(&self.value)?;
        }

        Ok(())
    }
}

impl From<&RDateProperty> for ContentLine {
    fn from(rdate_property: &RDateProperty) -> Self {
        ContentLine::from((
            "RDATE",
            (
                ContentLineParams::from(&rdate_property.params),
                rdate_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(RDateProperty);

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
            RDateProperty::parse_ical("RDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                RDateProperty {
                    params: RDatePropertyParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            RDateProperty::parse_ical("RDATE;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                RDateProperty {
                    params: RDatePropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(String::from("Europe/London"))),
                        other: HashMap::new(),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            RDateProperty::parse_ical("RDATE;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                RDateProperty {
                    params: RDatePropertyParams {
                        value_type: Some(ValueType::Date),
                        tzid: None,
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
                },
            ),
        );

        assert!(RDateProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("RDATE:19960401T150000Z"),
        );

        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(String::from("Europe/London"))),
                    other: HashMap::new(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("RDATE;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: Some(ValueType::Date),
                    tzid: None,
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("RDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }
}
