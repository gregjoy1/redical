use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, map_res, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::tzid::Tzid;
use crate::values::list::List;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

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

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for RDatePropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.clone().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(value_type) = self.value_type.as_ref() {
            content_line_params.insert(String::from("VALUE"), value_type.render_ical());
        }

        if let Some(tz) = self.get_context_tz(context) {
            if tz != chrono_tz::Tz::UTC {
                content_line_params.insert(String::from("TZID"), tz.to_string());
            }
        }

        content_line_params
    }
}

impl RDatePropertyParams {
    /// Sometimes we need to overide the timezone that date string within this property is rendered
    /// with. We do this via the optionally provided `RenderingContext`.
    ///
    /// We return the timezone contained within the `RenderingContext` (if present),
    ///   -> falling back to the one originally specified in the TZID param (if present)
    ///     -> falling back to None if nothing exists.
    fn get_context_tz(&self, context: Option<&RenderingContext>) -> Option<chrono_tz::Tz> {
        let mut tz = None;

        if let Some(tzid) = self.tzid.as_ref() {
            tz = Some(tzid.0);
        }

        if let Some(context_tz) = context.and_then(|context| context.tz) {
            tz = Some(context_tz);
        }

        tz
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
    pub date_times: List<DateTime>,
}

impl ICalendarDateTimeProperty for RDateProperty {
    fn new(value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let params =
            RDatePropertyParams {
                value_type: value_type.cloned(),
                tzid: tzid.cloned(),
                other: HashMap::new(),
            };

        RDateProperty {
            params,
            date_times: vec![date_time.to_owned()].into(),
        }
    }

    fn get_tzid(&self) -> Option<&Tzid> {
        self.params.tzid.as_ref()
    }

    fn get_value_type(&self) -> Option<&ValueType> {
        self.params.value_type.as_ref()
    }

    fn get_date_time(&self) -> &DateTime {
        self.date_times.first().unwrap()
    }
}

impl RDateProperty {
    pub fn get_date_times(&self) -> Vec<DateTime> {
        self.date_times.to_vec()
    }
}

impl ICalendarEntity for RDateProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RDATE",
            preceded(
                tag("RDATE"),
                cut(
                    map_res(
                        pair(
                            opt(RDatePropertyParams::parse_ical),
                            preceded(
                                colon,
                                map(
                                    separated_list1(comma, DateTime::parse_ical),
                                    List,
                                ),
                            ),
                        ),
                        |(params, date_times)| {
                            let r_date_property =
                                RDateProperty {
                                    params: params.unwrap_or(RDatePropertyParams::default()),
                                    date_times,
                                };

                            if let Err(error) = ICalendarEntity::validate(&r_date_property) {
                                return Err(
                                    ParserError::new(error, input)
                                );
                            }

                            Ok(r_date_property)
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_with_context(context).render_ical()
    }

    fn validate(&self) -> Result<(), String> {
        for date_time in self.date_times.iter() {
            date_time.validate()?;
        }

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;

            for date_time in self.date_times.iter() {
                tzid.validate_with_datetime_value(date_time)?;
            }
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            for date_time in self.date_times.iter() {
                value_type.validate_against_date_time(date_time)?;
            }
        }

        Ok(())
    }
}

impl ICalendarProperty for RDateProperty {
    /// Build a `ContentLine` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine {
        // To allow this property to be rendered in a different timezone, we first need to know the
        // current timezone[1] to convert from and then the timezone in the rendering context[2] we
        // need to convert to.
        //
        // [1] We get this from the TZID property param - falling back to UTC if undefined.
        // [2] We get this (if provided) from the optionally provided `RenderingContext` - falling
        //     back to the earlier established current timezone.
        let current_tz = self.get_tz().unwrap_or(&chrono_tz::UTC);
        let context_tz = context.and_then(|context| context.tz.as_ref()).unwrap_or(current_tz);

        let context_adjusted_rendered_date_times =
            self.date_times
                .iter()
                .map(|date_time| {
                    date_time.with_timezone(Some(current_tz), context_tz)
                             .render_formatted_date_time(Some(context_tz))
                })
                .collect::<Vec<String>>()
                .join(",");

        ContentLine::from((
            "RDATE",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_rendered_date_times,
            )
        ))
    }
}

impl std::hash::Hash for RDateProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(RDateProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RDateProperty::parse_ical("RDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                RDateProperty {
                    params: RDatePropertyParams::default(),
                    date_times: vec![
                        DateTime::UtcDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                                NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                    ].into(),
                },
            ),
        );

        assert_parser_output!(
            RDateProperty::parse_ical("RDATE:19960401T150000Z,19960403T180000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                RDateProperty {
                    params: RDatePropertyParams::default(),
                    date_times: vec![
                        DateTime::UtcDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                                NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                        DateTime::UtcDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(1996_i32, 4_u32, 3_u32).unwrap(),
                                NaiveTime::from_hms_opt(18_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                    ].into(),
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
                    date_times: vec![
                        DateTime::LocalDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                                NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                    ].into(),
                },
            ),
        );

        assert_parser_output!(
            RDateProperty::parse_ical("RDATE;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401,19960403,19960405".into()),
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
                    date_times: vec![
                        DateTime::LocalDate(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                        ),
                        DateTime::LocalDate(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 3_u32).unwrap()
                        ),
                        DateTime::LocalDate(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 5_u32).unwrap()
                        ),
                    ].into(),
                },
            ),
        );

        assert!(RDateProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn parse_ical_wth_tz_dst_gap_date_time() {
        // Assert impossible date/time fails validation.
        assert_parser_error!(
            RDateProperty::parse_ical("RDATE;TZID=Pacific/Auckland:20240929T020000".into()),
            nom::Err::Failure(
                span: ";TZID=Pacific/Auckland:20240929T020000",
                message: "Error - detected timezone aware datetime within a DST transition gap (supply this as UTC or fully DST adjusted) at \"RDATE;TZID=Pacific/Auckland:20240929T020000\"",
                context: ["RDATE"],
            ),
        );

        // Assert possible date/time does not fail validation.
        assert_parser_output!(
            RDateProperty::parse_ical("RDATE;TZID=Pacific/Auckland:20240929T010000".into()),
            (
                "",
                RDateProperty {
                    params: RDatePropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(Tz::Pacific__Auckland)),
                        other: HashMap::new(),
                    },
                    date_times: vec![
                        DateTime::LocalDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(2024_i32, 9_u32, 29_u32).unwrap(),
                                NaiveTime::from_hms_opt(1_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                    ].into(),
                },
            ),
        );

        assert_parser_output!(
            RDateProperty::parse_ical("RDATE;TZID=Pacific/Auckland:20240929T030000".into()),
            (
                "",
                RDateProperty {
                    params: RDatePropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(Tz::Pacific__Auckland)),
                        other: HashMap::new(),
                    },
                    date_times: vec![
                        DateTime::LocalDateTime(
                            NaiveDateTime::new(
                                NaiveDate::from_ymd_opt(2024_i32, 9_u32, 29_u32).unwrap(),
                                NaiveTime::from_hms_opt(3_u32, 0_u32, 0_u32).unwrap(),
                            )
                        ),
                    ].into(),
                },
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams::default(),
                date_times: vec![
                    DateTime::UtcDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                    DateTime::UtcDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 3_u32).unwrap(),
                            NaiveTime::from_hms_opt(18_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                ].into(),
            }.render_ical(),
            String::from("RDATE:19960401T150000Z,19960403T180000Z"),
        );

        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(Tz::Europe__London)),
                    other: HashMap::new(),
                },
                date_times: vec![
                    DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                ].into(),
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
                date_times: vec![
                    DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                ].into(),
            }.render_ical(),
            String::from("RDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST)
        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams::default(),
                date_times: vec![
                    DateTime::UtcDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                    DateTime::UtcDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 3_u32).unwrap(),
                            NaiveTime::from_hms_opt(18_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                ].into(),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Warsaw), distance_unit: None })),
            String::from("RDATE;TZID=Europe/Warsaw:19960401T170000,19960403T200000"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST)
        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(Tz::Europe__London)),
                    other: HashMap::new(),
                },
                date_times: vec![
                    DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                ].into(),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("RDATE;TZID=America/Phoenix:19960401T070000"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC
        assert_eq!(
            RDateProperty {
                params: RDatePropertyParams {
                    value_type: None,
                    tzid: Some(Tzid(Tz::Europe__London)),
                    other: HashMap::new(),
                },
                date_times: vec![
                    DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                            NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                    DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(1996_i32, 4_u32, 3_u32).unwrap(),
                            NaiveTime::from_hms_opt(18_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                ].into(),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::UTC), distance_unit: None })),
            String::from("RDATE:19960401T140000Z,19960403T170000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST)
        // Presents as previous day (00:00:00 - 7 hours)
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
                date_times: vec![
                    DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                ].into(),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("RDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE;TZID=America/Phoenix:19960331"),
        );
    }
}
