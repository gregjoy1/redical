use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::value_data_types::date_time::{DateTime, ValueType};
use crate::value_data_types::tzid::Tzid;

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

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
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut RDatePropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
        (
            pair(tag("VALUE"), cut(preceded(tag("="), ValueType::parse_ical))),
            |params: &mut RDatePropertyParams, (_key, value): (ParserInput, ValueType)| params.value_type = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut RDatePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
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
    pub date_time: DateTime,
}

impl ICalendarDateTimeProperty for RDateProperty {
    fn get_tzid(&self) -> Option<&Tzid> {
        self.params.tzid.as_ref()
    }

    fn get_value_type(&self) -> Option<&ValueType> {
        self.params.value_type.as_ref()
    }

    fn get_date_time(&self) -> &DateTime {
        &self.date_time
    }
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
                        |(params, date_time)| {
                            RDateProperty {
                                params: params.unwrap_or(RDatePropertyParams::default()),
                                date_time,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        ContentLine::from(self).render_ical()
    }

    fn validate(&self) -> Result<(), String> {
        self.date_time.validate()?;

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            value_type.validate_against_date_time(&self.date_time)?;
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
                rdate_property.date_time.render_formatted_date_time(rdate_property.get_tz())
            )
        ))
    }
}

impl_icalendar_entity_traits!(RDateProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RDateProperty::parse_ical("RDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                RDateProperty {
                    params: RDatePropertyParams::default(),
                    date_time: DateTime::UtcDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
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
                        tzid: Some(Tzid(Tz::Europe__London)),
                        other: HashMap::new(),
                    },
                    date_time: DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
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
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
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
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("RDATE:19960401T150000Z"),
        );

        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(Tz::Europe__London)),
                    other: HashMap::new(),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
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
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("RDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }
}
