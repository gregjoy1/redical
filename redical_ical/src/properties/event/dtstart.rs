use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map_res, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::tzid::Tzid;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

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

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for DTStartPropertyParams {
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

impl DTStartPropertyParams {
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
    pub date_time: DateTime,
}

impl ICalendarDateTimeProperty for DTStartProperty {
    fn new(value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let params =
            DTStartPropertyParams {
                value_type: value_type.cloned(),
                tzid: tzid.cloned(),
                other: HashMap::new(),
            };

        DTStartProperty {
            params,
            date_time: date_time.to_owned(),
        }
    }

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

impl ICalendarEntity for DTStartProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DTSTART",
            preceded(
                tag("DTSTART"),
                cut(
                    map_res(
                        pair(
                            opt(DTStartPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, date_time)| {
                            let dtstart_property =
                                DTStartProperty {
                                    params: params.unwrap_or(DTStartPropertyParams::default()),
                                    date_time,
                                };

                            if let Err(error) = ICalendarEntity::validate(&dtstart_property) {
                                return Err(
                                    ParserError::new(error, input)
                                );
                            }

                            Ok(dtstart_property)
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
        self.date_time.validate()?;

        if let Some(tzid) = self.params.tzid.as_ref() {
            tzid.validate()?;

            tzid.validate_with_datetime_value(&self.date_time)?;
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            value_type.validate_against_date_time(&self.date_time)?;
        }

        Ok(())
    }
}

impl ICalendarProperty for DTStartProperty {
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

        let context_adjusted_date_time = self.date_time.with_timezone(Some(current_tz), context_tz);

        ContentLine::from((
            "DTSTART",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_date_time.render_formatted_date_time(Some(context_tz)),
            )
        ))
    }
}

impl std::hash::Hash for DTStartProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(DTStartProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DTStartProperty {
                    params: DTStartPropertyParams::default(),
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
            DTStartProperty::parse_ical("DTSTART;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                DTStartProperty {
                    params: DTStartPropertyParams {
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
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                },
            ),
        );

        assert!(DTStartProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn parse_ical_wth_tz_dst_gap_date_time() {
        // Assert impossible date/time fails validation.
        assert_parser_error!(
            DTStartProperty::parse_ical("DTSTART;TZID=Pacific/Auckland:20240929T020000".into()),
            nom::Err::Failure(
                span: ";TZID=Pacific/Auckland:20240929T020000",
                message: "Error - detected timezone aware datetime within a DST transition gap (supply this as UTC or fully DST adjusted) at \"DTSTART;TZID=Pacific/Auckland:20240929T020000\"",
                context: ["DTSTART"],
            ),
        );

        // Assert possible date/time does not fail validation.
        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART;TZID=Pacific/Auckland:20240929T010000".into()),
            (
                "",
                DTStartProperty {
                    params: DTStartPropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(Tz::Pacific__Auckland)),
                        other: HashMap::new(),
                    },
                    date_time: DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(2024_i32, 9_u32, 29_u32).unwrap(),
                            NaiveTime::from_hms_opt(1_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                },
            ),
        );

        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART;TZID=Pacific/Auckland:20240929T030000".into()),
            (
                "",
                DTStartProperty {
                    params: DTStartPropertyParams {
                        value_type: None,
                        tzid: Some(Tzid(Tz::Pacific__Auckland)),
                        other: HashMap::new(),
                    },
                    date_time: DateTime::LocalDateTime(
                        NaiveDateTime::new(
                            NaiveDate::from_ymd_opt(2024_i32, 9_u32, 29_u32).unwrap(),
                            NaiveTime::from_hms_opt(3_u32, 0_u32, 0_u32).unwrap(),
                        )
                    ),
                },
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("DTSTART:19960401T150000Z"),
        );

        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams {
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
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("DTSTART;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST)
        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Warsaw), distance_unit: None })),
            String::from("DTSTART;TZID=Europe/Warsaw:19960401T170000"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST)
        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams {
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
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("DTSTART;TZID=America/Phoenix:19960401T070000"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC
        assert_eq!(
            DTStartProperty {
                params: DTStartPropertyParams {
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
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::UTC), distance_unit: None })),
            String::from("DTSTART:19960401T140000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST)
        // Presents as previous day (00:00:00 - 7 hours)
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
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("DTSTART;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE;TZID=America/Phoenix:19960331"),
        );
    }
}
