use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::tzid::Tzid;
use crate::values::list::List;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ExDatePropertyParams {
    pub tzid: Option<Tzid>,
    pub value_type: Option<ValueType>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for ExDatePropertyParams {
    define_property_params_ical_parser!(
        ExDatePropertyParams,
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut ExDatePropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
        (
            pair(tag("VALUE"), cut(preceded(tag("="), ValueType::parse_ical))),
            |params: &mut ExDatePropertyParams, (_key, value): (ParserInput, ValueType)| params.value_type = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut ExDatePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for ExDatePropertyParams {
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

impl ExDatePropertyParams {
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

// Exception Date-Times
//
// Property Name:  EXDATE
//
// Purpose:  This property defines the list of DATE-TIME exceptions for
//    recurring events, to-dos, journal entries, or time zone
//    definitions.
//
// Value Type:  The default value type for this property is DATE-TIME.
//    The value type can be set to DATE.
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
//     exdate     = "EXDATE" exdtparam ":" exdtval *("," exdtval) CRLF
//
//     exdtparam  = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" "VALUE" "=" ("DATE-TIME" / "DATE")) /
//                ;
//                (";" tzidparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//     exdtval    = date-time / date
//     ;Value MUST match value type
//
// Example:  The following is an example of this property:
//
//     EXDATE:19960402T010000Z,19960403T010000Z,19960404T010000Z
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ExDateProperty {
    pub params: ExDatePropertyParams,
    pub date_times: List<DateTime>,
}

impl ICalendarDateTimeProperty for ExDateProperty {
    fn new(value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let params =
            ExDatePropertyParams {
                value_type: value_type.cloned(),
                tzid: tzid.cloned(),
                other: HashMap::new(),
            };

        ExDateProperty {
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

impl ExDateProperty {
    pub fn get_date_times(&self) -> Vec<DateTime> {
        self.date_times.to_vec()
    }
}

impl ICalendarEntity for ExDateProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "EXDATE",
            preceded(
                tag("EXDATE"),
                cut(
                    map(
                        pair(
                            opt(ExDatePropertyParams::parse_ical),
                            preceded(
                                colon,
                                map(
                                    separated_list1(comma, DateTime::parse_ical),
                                    List,
                                ),
                            ),
                        ),
                        |(params, date_times)| {
                            ExDateProperty {
                                params: params.unwrap_or(ExDatePropertyParams::default()),
                                date_times,
                            }
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
        };

        if let Some(value_type) = self.params.value_type.as_ref() {
            for date_time in self.date_times.iter() {
                value_type.validate_against_date_time(date_time)?;
            }
        }

        Ok(())
    }
}

impl ICalendarProperty for ExDateProperty {
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
            "EXDATE",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_rendered_date_times,
            )
        ))
    }
}

impl std::hash::Hash for ExDateProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(ExDateProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ExDateProperty::parse_ical("EXDATE:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ExDateProperty {
                    params: ExDatePropertyParams::default(),
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
            ExDateProperty::parse_ical("EXDATE:19960401T150000Z,19960403T180000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                ExDateProperty {
                    params: ExDatePropertyParams::default(),
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
            ExDateProperty::parse_ical("EXDATE;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                ExDateProperty {
                    params: ExDatePropertyParams {
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
            ExDateProperty::parse_ical("EXDATE;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401,19960403,19960405".into()),
            (
                "",
                ExDateProperty {
                    params: ExDatePropertyParams {
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

        assert!(ExDateProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams::default(),
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
            String::from("EXDATE:19960401T150000Z,19960403T180000Z"),
        );

        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams {
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
            String::from("EXDATE;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams {
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
            String::from("EXDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST)
        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams::default(),
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
            String::from("EXDATE;TZID=Europe/Warsaw:19960401T170000,19960403T200000"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST)
        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams {
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
            String::from("EXDATE;TZID=America/Phoenix:19960401T070000"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC
        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams {
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
            String::from("EXDATE:19960401T140000Z,19960403T170000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST)
        // Presents as previous day (00:00:00 - 7 hours)
        assert_eq!(
            ExDateProperty {
                params: ExDatePropertyParams {
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
            String::from("EXDATE;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE;TZID=America/Phoenix:19960331"),
        );
    }
}
