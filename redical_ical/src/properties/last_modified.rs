use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::integer::Integer;
use crate::values::tzid::Tzid;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

// TODO: Potentially accomodate RANGE param if required.
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct LastModifiedPropertyParams {
    pub millis: Option<Integer>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for LastModifiedPropertyParams {
    define_property_params_ical_parser!(
        LastModifiedPropertyParams,
        (
            pair(tag("X-MILLIS"), cut(preceded(tag("="), Integer::parse_ical))),
            |params: &mut LastModifiedPropertyParams, (_key, value): (ParserInput, Integer)| params.millis = Some(value),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut LastModifiedPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for LastModifiedPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(millis) = self.millis.as_ref() {
            content_line_params.insert(String::from("X-MILLIS"), millis.render_ical());
        }

        content_line_params
    }
}

impl LastModifiedPropertyParams {
    /// Sometimes we need to overide the timezone that date string within this property is rendered
    /// with. We do this via the optionally provided `RenderingContext`.
    ///
    /// We return the timezone contained within the `RenderingContext` (if present),
    ///   -> falling back to the one originally specified in the TZID param (if present)
    ///     -> falling back to None if nothing exists.
    fn get_context_tz(&self, context: Option<&RenderingContext>) -> Option<chrono_tz::Tz> {
        Some(chrono_tz::Tz::UTC)
    }
}

// Last Modified
//
// Property Name:  LAST-MODIFIED
//
// Purpose:  This property specifies the date and time that the
//    information associated with the calendar component was last
//    revised in the calendar store.
//
//       Note: This is analogous to the modification date and time for a
//       file in the file system.
//
// Value Type:  DATE-TIME
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in the "VEVENT",
//    "VTODO", "VJOURNAL", or "VTIMEZONE" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     last-mod   = "LAST-MODIFIED" lstparam ":" date-time CRLF
//
//     lstparam   = *(";" other-param)
//
// Example:  The following is an example of this property:
//
//     LAST-MODIFIED:19960817T133000Z
//
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LastModifiedProperty {
    pub params: LastModifiedPropertyParams,
    pub date_time: DateTime,
}

impl LastModifiedProperty {
    pub fn get_millis(&self) -> i64 {
        self.params.millis.clone().map_or(0_i64, |millis| *millis)
    }

    pub fn new_from_now(set_millis: bool) -> Self {
        let current_date_time = chrono::offset::Utc::now();

        let millis = current_date_time.timestamp_subsec_millis() as i64;

        let date_time = DateTime::UtcDateTime(current_date_time.naive_utc());

        let mut params = LastModifiedPropertyParams::default();

        if set_millis {
            params.millis = Some(Integer::from(millis));
        }

        LastModifiedProperty {
            params,
            date_time,
        }
    }
}

impl ICalendarDateTimeProperty for LastModifiedProperty {
    fn new(_value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let params = LastModifiedPropertyParams::default();

        let current_tz = tzid.map_or(None, |tzid| Some(&tzid.0));

        // This property can only be UTC.
        let date_time = date_time.with_timezone(current_tz, &chrono_tz::Tz::UTC);

        let last_modified_property = LastModifiedProperty {
            params,
            date_time: date_time.to_owned(),
        };

        last_modified_property
    }

    fn get_tzid(&self) -> Option<&Tzid> {
        Some(&Tzid(chrono_tz::Tz::UTC))
    }

    fn get_value_type(&self) -> Option<&ValueType> {
        Some(&ValueType::DateTime)
    }

    fn get_date_time(&self) -> &DateTime {
        &self.date_time
    }
}

impl ICalendarEntity for LastModifiedProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "LAST-MODIFIED",
            preceded(
                tag("LAST-MODIFIED"),
                cut(
                    map(
                        pair(
                            opt(LastModifiedPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, date_time)| {
                            // This property can only be UTC.
                            let date_time = date_time.with_timezone(Some(&chrono_tz::Tz::UTC), &chrono_tz::Tz::UTC);

                            let last_modified_property =
                                LastModifiedProperty {
                                    params: params.unwrap_or(LastModifiedPropertyParams::default()),
                                    date_time,
                                };

                            last_modified_property
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

        Ok(())
    }
}

impl ICalendarProperty for LastModifiedProperty {
    /// Build a `ContentLine` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine {
        let last_modified_property = self.clone();

        // Force `LocalDate` into full `DateTime`.
        let date_time =
            match last_modified_property.date_time {
                DateTime::LocalDate(naive_date) => {
                    DateTime::LocalDateTime(
                        chrono::NaiveDateTime::new(
                            naive_date,
                            chrono::NaiveTime::from_hms_opt(0_u32, 0_u32, 0_u32).unwrap(),
                        )
                    )
                },

                _ => last_modified_property.date_time,
            };

        ContentLine::from((
            "LAST-MODIFIED",
            (
                last_modified_property.params.to_content_line_params_with_context(context),
                date_time.render_formatted_date_time(None),
            )
        ))
    }
}

impl std::hash::Hash for LastModifiedProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl Ord for LastModifiedProperty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let ordering =
            self.date_time
                .get_utc_timestamp(None)
                .cmp(
                    &other.date_time
                          .get_utc_timestamp(None)
                );

        if ordering.is_eq() {
            self.get_millis().cmp(&other.get_millis())
        } else {
            ordering
        }
    }
}

impl PartialOrd for LastModifiedProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let partial_ordering =
            self.date_time
                .get_utc_timestamp(None)
                .partial_cmp(
                    &other.date_time
                          .get_utc_timestamp(None)
                );

        if partial_ordering.is_some_and(|partial_ordering| partial_ordering.is_eq()) {
            self.get_millis().partial_cmp(&other.get_millis())
        } else {
            partial_ordering
        }
    }
}

impl_icalendar_entity_traits!(LastModifiedProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            LastModifiedProperty::parse_ical("LAST-MODIFIED:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                LastModifiedProperty {
                    params: LastModifiedPropertyParams {
                        millis: None,
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
            LastModifiedProperty::parse_ical("LAST-MODIFIED;X-MILLIS=1234:19960401T150000".into()),
            (
                "",
                LastModifiedProperty {
                    params: LastModifiedPropertyParams {
                        millis: Some(Integer(1234_i64)),
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
            LastModifiedProperty::parse_ical("LAST-MODIFIED;X-TEST=X_VALUE;TEST=VALUE;VALUE=DATE:19960401".into()),
            (
                "",
                LastModifiedProperty {
                    params: LastModifiedPropertyParams {
                        millis: None,
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("VALUE"), String::from("DATE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                },
            ),
        );

        assert!(LastModifiedProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            LastModifiedProperty {
                    params: LastModifiedPropertyParams {
                        millis: None,
                        other: HashMap::new(),
                    },
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("LAST-MODIFIED:19960401T150000Z"),
        );

        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams {
                    millis: Some(Integer(1234_i64)),
                    other: HashMap::new(),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("LAST-MODIFIED;X-MILLIS=1234:19960401T150000Z"),
        );

        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams {
                    millis: None,
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("VALUE"), String::from("DATE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("LAST-MODIFIED;TEST=VALUE;VALUE=DATE;X-TEST=X_VALUE:19960401T000000Z"),
        );
    }

    #[test]
    fn render_ical_with_context_tz_override() {
        // UTC -> Europe/Warsaw (UTC +02:00 DST) -- IGNORED
        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::Europe__Warsaw), distance_unit: None })),
            String::from("LAST-MODIFIED:19960401T150000Z"),
        );

        // Europe/London (UTC +01:00 BST) -> America/Phoenix (UTC -07:00 MST) -- IGNORED
        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams {
                    millis: None,
                    other: HashMap::new(),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("LAST-MODIFIED:19960401T150000Z"),
        );

        // Europe/London (UTC +01:00 BST) -> UTC -- IGNORED
        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams {
                    millis: None,
                    other: HashMap::new(),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::UTC), distance_unit: None })),
            String::from("LAST-MODIFIED:19960401T150000Z"),
        );

        // UTC (implied) -> America/Phoenix (UTC -07:00 MST) -- IGNORED
        // Presents as previous day (00:00:00 - 7 hours)
        assert_eq!(
            LastModifiedProperty {
                params: LastModifiedPropertyParams {
                    millis: None,
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical_with_context(Some(&RenderingContext { tz: Some(Tz::America__Phoenix), distance_unit: None })),
            String::from("LAST-MODIFIED;TEST=VALUE;X-TEST=X_VALUE:19960401T000000Z"),
        );
    }

    #[test]
    fn new_from_now_with_millis() {
        let last_modified_property = LastModifiedProperty::new_from_now(true);

        let current_date_time = chrono::offset::Utc::now();

        assert_eq!(last_modified_property.get_utc_timestamp(), current_date_time.timestamp());

        let current_millis = current_date_time.timestamp_subsec_millis() as i64;

        assert!(current_millis >= last_modified_property.get_millis());
    }

    #[test]
    fn new_from_now_without_millis() {
        let last_modified_property = LastModifiedProperty::new_from_now(false);

        let current_date_time = chrono::offset::Utc::now();

        assert_eq!(last_modified_property.get_utc_timestamp(), current_date_time.timestamp());

        assert_eq!(0, last_modified_property.get_millis());
    }
}
