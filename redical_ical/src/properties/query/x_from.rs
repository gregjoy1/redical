use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut, opt};

use crate::value_data_types::date_time::{DateTime, ValueType};
use crate::value_data_types::tzid::Tzid;

use crate::grammar::{tag, semicolon, colon};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// prop = "DTSTART" / "DTEND"
//
// ;Default is DTSTART
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PropValue {
    DTStart,
    DTEnd,
}

impl ICalendarEntity for PropValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PROP",
            alt((
                map(tag("DTSTART"), |_| PropValue::DTStart),
                map(tag("DTEND"), |_| PropValue::DTEnd),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::DTStart => String::from("DTSTART"),
           Self::DTEnd => String::from("DTEND"),
        }
    }
}

impl_icalendar_entity_traits!(PropValue);

// OP = "GT" / "GTE"
//
// ;Default is GT
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum FromRangeOperator {
    GreaterThan,
    GreaterEqualThan,
}

impl ICalendarEntity for FromRangeOperator {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "OP",
            alt((
                map(tag("GTE"), |_| FromRangeOperator::GreaterEqualThan),
                map(tag("GT"), |_| FromRangeOperator::GreaterThan),
            )),
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
           Self::GreaterThan => String::from("GT"),
           Self::GreaterEqualThan => String::from("GTE"),
        }
    }
}

impl_icalendar_entity_traits!(FromRangeOperator);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XFromPropertyParams {
    pub prop: PropValue,
    pub op: FromRangeOperator,
    pub tzid: Option<Tzid>,
}

impl ICalendarEntity for XFromPropertyParams {
    define_property_params_ical_parser!(
        XFromPropertyParams,
        (
            pair(tag("PROP"), cut(preceded(tag("="), PropValue::parse_ical))),
            |params: &mut XFromPropertyParams, (_key, value): (ParserInput, PropValue)| params.prop = value,
        ),
        (
            pair(tag("OP"), cut(preceded(tag("="), FromRangeOperator::parse_ical))),
            |params: &mut XFromPropertyParams, (_key, value): (ParserInput, FromRangeOperator)| params.op = value,
        ),
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut XFromPropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XFromPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        content_line_params.insert(String::from("PROP"), self.prop.render_ical());
        content_line_params.insert(String::from("OP"), self.op.render_ical());

        if let Some(tz) = self.get_context_tz(context) {
            content_line_params.insert(String::from("TZID"), tz.to_string());
        }

        content_line_params
    }
}

impl XFromPropertyParams {
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

impl Default for XFromPropertyParams {
    fn default() -> Self {
        XFromPropertyParams {
            prop: PropValue::DTStart,
            op: FromRangeOperator::GreaterThan,
            tzid: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XFromProperty {
    pub params: XFromPropertyParams,
    pub date_time: DateTime,
}

impl XFromProperty {
    pub fn get_prop(&self) -> PropValue {
        self.params.prop.to_owned()
    }

    pub fn get_op(&self) -> FromRangeOperator {
        self.params.op.to_owned()
    }
}

impl ICalendarDateTimeProperty for XFromProperty {
    fn new(_value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let mut params = XFromPropertyParams::default();

        params.tzid = tzid.cloned();

        XFromProperty {
            params,
            date_time: date_time.to_owned(),
        }
    }

    fn get_tzid(&self) -> Option<&Tzid> {
        self.params.tzid.as_ref()
    }

    fn get_value_type(&self) -> Option<&ValueType> {
        None
    }

    fn get_date_time(&self) -> &DateTime {
        &self.date_time
    }
}

impl ICalendarEntity for XFromProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-FROM",
            preceded(
                tag("X-FROM"),
                cut(
                    map(
                        pair(
                            opt(XFromPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, date_time)| {
                            XFromProperty {
                                params: params.unwrap_or(XFromPropertyParams::default()),
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

        Ok(())
    }
}

impl ICalendarProperty for XFromProperty {
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
            "X-FROM",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_date_time.render_formatted_date_time(Some(context_tz)),
            )
        ))
    }
}

impl std::hash::Hash for XFromProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XFromProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XFromProperty::parse_ical("X-FROM:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XFromProperty {
                    params: XFromPropertyParams::default(),
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
            XFromProperty::parse_ical("X-FROM;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                XFromProperty {
                    params: XFromPropertyParams {
                        prop: PropValue::DTStart,
                        op: FromRangeOperator::GreaterThan,
                        tzid: Some(Tzid(Tz::Europe__London)),
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
            XFromProperty::parse_ical("X-FROM;PROP=DTEND;OP=GTE:19960401".into()),
            (
                "",
                XFromProperty {
                    params: XFromPropertyParams {
                        prop: PropValue::DTEnd,
                        op: FromRangeOperator::GreaterEqualThan,
                        tzid: None,
                    },
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                },
            ),
        );

        assert!(XFromProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XFromProperty {
                params: XFromPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("X-FROM;PROP=DTSTART;OP=GT:19960401T150000Z"),
        );

        assert_eq!(
            XFromProperty {
                params: XFromPropertyParams {
                    prop: PropValue::DTStart,
                    op: FromRangeOperator::GreaterThan,
                    tzid: Some(Tzid(Tz::Europe__London)),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("X-FROM;PROP=DTSTART;OP=GT;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            XFromProperty {
                params: XFromPropertyParams {
                    prop: PropValue::DTEnd,
                    op: FromRangeOperator::GreaterEqualThan,
                    tzid: None,
                },
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("X-FROM;PROP=DTEND;OP=GTE:19960401"),
        );
    }
}
