use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::tzid::Tzid;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

// TODO: Potentially accomodate RANGE param if required.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RecurrenceIDPropertyParams {
    pub tzid: Option<Tzid>,
    pub value_type: Option<ValueType>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for RecurrenceIDPropertyParams {
    define_property_params_ical_parser!(
        RecurrenceIDPropertyParams,
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut RecurrenceIDPropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
        (
            pair(tag("VALUE"), cut(preceded(tag("="), ValueType::parse_ical))),
            |params: &mut RecurrenceIDPropertyParams, (_key, value): (ParserInput, ValueType)| params.value_type = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut RecurrenceIDPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for RecurrenceIDPropertyParams {
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

impl RecurrenceIDPropertyParams {
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
pub struct RecurrenceIDProperty {
    pub params: RecurrenceIDPropertyParams,
    pub date_time: DateTime,
}

impl RecurrenceIDProperty {
    fn enforce_value_type_param(&mut self) {
        if self.params.value_type.is_none() {
            self.params.value_type = Some(ValueType::new_from_date_time(&self.date_time));
        }
    }
}

impl ICalendarDateTimeProperty for RecurrenceIDProperty {
    fn new(value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let params =
            RecurrenceIDPropertyParams {
                tzid: tzid.cloned(),
                value_type: value_type.cloned(),
                other: HashMap::new(),
            };

        let mut recurrence_id_property = RecurrenceIDProperty {
            params,
            date_time: date_time.to_owned(),
        };

        // Always ensure VALUE param is defined.
        recurrence_id_property.enforce_value_type_param();

        recurrence_id_property
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

impl ICalendarEntity for RecurrenceIDProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RECURRENCE-ID",
            preceded(
                tag("RECURRENCE-ID"),
                cut(
                    map(
                        pair(
                            opt(RecurrenceIDPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, date_time)| {
                            let mut recurrence_id_property =
                                RecurrenceIDProperty {
                                    params: params.unwrap_or(RecurrenceIDPropertyParams::default()),
                                    date_time,
                                };

                            // Always ensure VALUE param is defined.
                            recurrence_id_property.enforce_value_type_param();

                            recurrence_id_property
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
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            value_type.validate_against_date_time(&self.date_time)?;
        }

        Ok(())
    }
}

impl ICalendarProperty for RecurrenceIDProperty {
    /// Build a `ContentLine` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine {
        let mut recurrence_id_property = self.clone();

        // To allow this property to be rendered in a different timezone, we first need to know the
        // current timezone[1] to convert from and then the timezone in the rendering context[2] we
        // need to convert to.
        //
        // [1] We get this from the TZID property param - falling back to UTC if undefined.
        // [2] We get this (if provided) from the optionally provided `RenderingContext` - falling
        //     back to the earlier established current timezone.
        let current_tz = self.get_tz().unwrap_or(&chrono_tz::UTC);
        let context_tz = context.and_then(|context| context.tz.as_ref()).unwrap_or(current_tz);

        let context_adjusted_date_time = recurrence_id_property.date_time.with_timezone(Some(current_tz), context_tz);

        // Always ensure VALUE param is defined and present when rendering.
        recurrence_id_property.enforce_value_type_param();

        ContentLine::from((
            "RECURRENCE-ID",
            (
                recurrence_id_property.params.to_content_line_params_with_context(context),
                context_adjusted_date_time.render_formatted_date_time(Some(context_tz)),
            )
        ))
    }
}

impl std::hash::Hash for RecurrenceIDProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(RecurrenceIDProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RecurrenceIDProperty::parse_ical("RECURRENCE-ID:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                RecurrenceIDProperty {
                    params: RecurrenceIDPropertyParams {
                        value_type: Some(ValueType::DateTime),
                        tzid: None,
                        other: HashMap::new(),
                    },
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
            RecurrenceIDProperty::parse_ical("RECURRENCE-ID;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                RecurrenceIDProperty {
                    params: RecurrenceIDPropertyParams {
                        value_type: Some(ValueType::DateTime),
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
            RecurrenceIDProperty::parse_ical("RECURRENCE-ID;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                RecurrenceIDProperty {
                    params: RecurrenceIDPropertyParams {
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

        assert!(RecurrenceIDProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RecurrenceIDProperty {
                    params: RecurrenceIDPropertyParams {
                        value_type: Some(ValueType::DateTime),
                        tzid: None,
                        other: HashMap::new(),
                    },
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("RECURRENCE-ID;VALUE=DATE-TIME:19960401T150000Z"),
        );

        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams {
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
            String::from("RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams {
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
            String::from("RECURRENCE-ID;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST)
        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Warsaw), distance_unit: None })),
            String::from("RECURRENCE-ID;VALUE=DATE-TIME;TZID=Europe/Warsaw:19960401T170000"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST)
        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams {
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
            String::from("RECURRENCE-ID;VALUE=DATE-TIME;TZID=America/Phoenix:19960401T070000"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC
        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams {
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
            String::from("RECURRENCE-ID;VALUE=DATE-TIME:19960401T140000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST)
        // Presents as previous day (00:00:00 - 7 hours)
        assert_eq!(
            RecurrenceIDProperty {
                params: RecurrenceIDPropertyParams {
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
            String::from("RECURRENCE-ID;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE;TZID=America/Phoenix:19960331"),
        );
    }
}
