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
pub struct DTEndPropertyParams {
    pub tzid: Option<Tzid>,
    pub value_type: Option<ValueType>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for DTEndPropertyParams {
    define_property_params_ical_parser!(
        DTEndPropertyParams,
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut DTEndPropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
        (
            pair(tag("VALUE"), cut(preceded(tag("="), ValueType::parse_ical))),
            |params: &mut DTEndPropertyParams, (_key, value): (ParserInput, ValueType)| params.value_type = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut DTEndPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&DTEndPropertyParams> for ContentLineParams {
    fn from(related_to_params: &DTEndPropertyParams) -> Self {
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

impl From<DTEndPropertyParams> for ContentLineParams {
    fn from(related_to_params: DTEndPropertyParams) -> Self {
        ContentLineParams::from(&related_to_params)
    }
}

// Date-Time End
//
// Property Name:  DTEND
//
// Purpose:  This property specifies the date and time that a calendar
//    component ends.
//
// Value Type:  The default value type is DATE-TIME.  The value type can
//    be set to a DATE value type.
//
// Property Parameters:  IANA, non-standard, value data type, and time
//    zone identifier property parameters can be specified on this
//    property.
//
// Conformance:  This property can be specified in "VEVENT" or
//    "VFREEBUSY" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     dtend      = "DTEND" dtendparam ":" dtendval CRLF
//
//     dtendparam = *(
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
//     dtendval   = date-time / date
//     ;Value MUST match value type
//
// Example:  The following is an example of this property:
//
//     DTEND:19960401T150000Z
//
//     DTEND;VALUE=DATE:19980704
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DTEndProperty {
    pub params: DTEndPropertyParams,
    pub value: DateTime,
}

impl ICalendarEntity for DTEndProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DTEND",
            preceded(
                tag("DTEND"),
                cut(
                    map(
                        pair(
                            opt(DTEndPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, value)| {
                            DTEndProperty {
                                params: params.unwrap_or(DTEndPropertyParams::default()),
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

impl From<&DTEndProperty> for ContentLine {
    fn from(dtend_property: &DTEndProperty) -> Self {
        ContentLine::from((
            "DTEND",
            (
                ContentLineParams::from(&dtend_property.params),
                dtend_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(DTEndProperty);

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
            DTEndProperty::parse_ical("DTEND:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DTEndProperty {
                    params: DTEndPropertyParams::default(),
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
                },
            ),
        );

        assert_parser_output!(
            DTEndProperty::parse_ical("DTEND;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                DTEndProperty {
                    params: DTEndPropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(String::from("Europe/London"))),
                        other: HashMap::new(),
                    },
                    value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
                },
            ),
        );

        assert_parser_output!(
            DTEndProperty::parse_ical("DTEND;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                DTEndProperty {
                    params: DTEndPropertyParams {
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

        assert!(DTEndProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams::default(),
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: true }) },
            }.render_ical(),
            String::from("DTEND:19960401T150000Z"),
        );

        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(String::from("Europe/London"))),
                    other: HashMap::new(),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: Some(Time{ hour: 15_u32, minute: 0_u32, second: 0_u32, is_utc: false }) },
            }.render_ical(),
            String::from("DTEND;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams {
                    value_type: Some(ValueType::Date),
                    tzid: None,
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                value: DateTime { date: Date { year: 1996_i32, month: 4_u32, day: 1_u32 }, time: None },
            }.render_ical(),
            String::from("DTEND;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }
}
