use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::property_value_data_types::date_time::{DateTime, ValueType};
use crate::property_value_data_types::tzid::Tzid;

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct DTStartPropertyParams {
    pub tzid: Option<Tzid>,
    pub value_type: Option<ValueType>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for DTStartPropertyParams {
    define_property_params_ical_parser!(
        DTStartPropertyParams,
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut DTStartPropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
        (
            pair(tag("VALUE"), cut(preceded(tag("="), ValueType::parse_ical))),
            |params: &mut DTStartPropertyParams, (_key, value): (ParserInput, ValueType)| params.value_type = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut DTStartPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&DTStartPropertyParams> for ContentLineParams {
    fn from(related_to_params: &DTStartPropertyParams) -> Self {
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

impl From<DTStartPropertyParams> for ContentLineParams {
    fn from(related_to_params: DTStartPropertyParams) -> Self {
        ContentLineParams::from(&related_to_params)
    }
}

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
pub struct DTStartProperty {
    pub params: DTStartPropertyParams,
    pub value: DateTime,
}

impl ICalendarEntity for DTStartProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DTSTART",
            preceded(
                tag("DTSTART"),
                cut(
                    map(
                        pair(
                            opt(DTStartPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            DTStartProperty {
                                params: params.unwrap_or(DTStartPropertyParams::default()),
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

impl From<&DTStartProperty> for ContentLine {
    fn from(dtstart_property: &DTStartProperty) -> Self {
        ContentLine::from((
            "DTSTART",
            (
                ContentLineParams::from(&dtstart_property.params),
                dtstart_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(DTStartProperty);

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
            DTStartProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DTStartProperty {
                    params: DTStartPropertyParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                DTStartProperty {
                    params: DTStartPropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(String::from("Europe/London"))),
                        other: HashMap::new(),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                DTStartProperty {
                    params: DTStartPropertyParams {
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

        assert!(DTStartProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("DTSTART:19960401T150000Z"),
        );

        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(String::from("Europe/London"))),
                    other: HashMap::new(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("DTSTART;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams {
                    value_type: Some(ValueType::Date),
                    tzid: None,
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("DTSTART;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }
}
