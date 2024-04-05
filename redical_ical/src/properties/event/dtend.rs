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

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

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

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for DTEndPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(value_type) = self.value_type.as_ref() {
            content_line_params.insert(String::from("VALUE"), value_type.render_ical());
        }

        if let Some(tz) = self.get_context_tz(context) {
            content_line_params.insert(String::from("TZID"), tz.to_string());
        }

        content_line_params
    }
}

impl DTEndPropertyParams {
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
    pub date_time: DateTime,
}

impl ICalendarDateTimeProperty for DTEndProperty {
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
                        |(params, date_time)| {
                            DTEndProperty {
                                params: params.unwrap_or(DTEndPropertyParams::default()),
                                date_time,
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

impl ICalendarProperty for DTEndProperty {
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
            "DTEND",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_date_time.render_formatted_date_time(Some(context_tz)),
            )
        ))
    }
}

impl_icalendar_entity_traits!(DTEndProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DTEndProperty::parse_ical("DTEND:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DTEndProperty {
                    params: DTEndPropertyParams::default(),
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
            DTEndProperty::parse_ical("DTEND;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                DTEndProperty {
                    params: DTEndPropertyParams {
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
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
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
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("DTEND:19960401T150000Z"),
        );

        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams {
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
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("DTEND;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE:19960401"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST)
        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Warsaw) })),
            String::from("DTEND;TZID=Europe/Warsaw:19960401T170000"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST)
        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams {
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
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix) })),
            String::from("DTEND;TZID=America/Phoenix:19960401T070000"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC
        assert_eq!(
            DTEndProperty {
                params: DTEndPropertyParams {
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
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::UTC) })),
            String::from("DTEND;TZID=UTC:19960401T140000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST)
        // Presents as previous day (00:00:00 - 7 hours)
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
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix) })),
            String::from("DTEND;TEST=VALUE;X-TEST=X_VALUE;VALUE=DATE;TZID=America/Phoenix:19960331"),
        );
    }
}
