use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut, opt};

use crate::values::date_time::{DateTime, ValueType};
use crate::values::tzid::Tzid;
use crate::values::where_range_property::WhereRangeProperty;
use crate::values::where_range_operator::WhereUntilRangeOperator;

use crate::grammar::{tag, semicolon, colon};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, ICalendarDateTimeProperty, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XUntilPropertyParams {
    pub prop: WhereRangeProperty,
    pub op: WhereUntilRangeOperator,
    pub tzid: Option<Tzid>,
}

impl ICalendarEntity for XUntilPropertyParams {
    define_property_params_ical_parser!(
        XUntilPropertyParams,
        (
            pair(tag("PROP"), cut(preceded(tag("="), WhereRangeProperty::parse_ical))),
            |params: &mut XUntilPropertyParams, (_key, value): (ParserInput, WhereRangeProperty)| params.prop = value,
        ),
        (
            pair(tag("OP"), cut(preceded(tag("="), WhereUntilRangeOperator::parse_ical))),
            |params: &mut XUntilPropertyParams, (_key, value): (ParserInput, WhereUntilRangeOperator)| params.op = value,
        ),
        (
            pair(tag("TZID"), cut(preceded(tag("="), Tzid::parse_ical))),
            |params: &mut XUntilPropertyParams, (_key, value): (ParserInput, Tzid)| params.tzid = Some(value),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for XUntilPropertyParams {
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

impl XUntilPropertyParams {
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

impl Default for XUntilPropertyParams {
    fn default() -> Self {
        XUntilPropertyParams {
            prop: WhereRangeProperty::DTStart,
            op: WhereUntilRangeOperator::LessThan,
            tzid: None,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct XUntilProperty {
    pub params: XUntilPropertyParams,
    pub date_time: DateTime,
}

impl XUntilProperty {
    pub fn get_prop(&self) -> WhereRangeProperty {
        self.params.prop.to_owned()
    }

    pub fn get_op(&self) -> WhereUntilRangeOperator {
        self.params.op.to_owned()
    }
}

impl ICalendarDateTimeProperty for XUntilProperty {
    fn new(_value_type: Option<&ValueType>, tzid: Option<&Tzid>, date_time: &DateTime) -> Self {
        let mut params = XUntilPropertyParams::default();

        params.tzid = tzid.cloned();

        XUntilProperty {
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

impl ICalendarEntity for XUntilProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "X-UNTIL",
            preceded(
                tag("X-UNTIL"),
                cut(
                    map(
                        pair(
                            opt(XUntilPropertyParams::parse_ical),
                            preceded(colon, DateTime::parse_ical),
                        ),
                        |(params, date_time)| {
                            XUntilProperty {
                                params: params.unwrap_or(XUntilPropertyParams::default()),
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

impl ICalendarProperty for XUntilProperty {
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
            "X-UNTIL",
            (
                self.params.to_content_line_params_with_context(context),
                context_adjusted_date_time.render_formatted_date_time(Some(context_tz)),
            )
        ))
    }
}

impl std::hash::Hash for XUntilProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(XUntilProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use chrono_tz::Tz;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            XUntilProperty::parse_ical("X-UNTIL:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                XUntilProperty {
                    params: XUntilPropertyParams::default(),
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
            XUntilProperty::parse_ical("X-UNTIL;TZID=Europe/London:19960401T150000".into()),
            (
                "",
                XUntilProperty {
                    params: XUntilPropertyParams {
                        prop: WhereRangeProperty::DTStart,
                        op: WhereUntilRangeOperator::LessThan,
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
            XUntilProperty::parse_ical("X-UNTIL;PROP=DTEND;OP=LTE:19960401".into()),
            (
                "",
                XUntilProperty {
                    params: XUntilPropertyParams {
                        prop: WhereRangeProperty::DTEnd,
                        op: WhereUntilRangeOperator::LessEqualThan,
                        tzid: None,
                    },
                    date_time: DateTime::LocalDate(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                    ),
                },
            ),
        );

        assert!(XUntilProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            XUntilProperty {
                params: XUntilPropertyParams::default(),
                date_time: DateTime::UtcDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("X-UNTIL;PROP=DTSTART;OP=LT:19960401T150000Z"),
        );

        assert_eq!(
            XUntilProperty {
                params: XUntilPropertyParams {
                    prop: WhereRangeProperty::DTStart,
                    op: WhereUntilRangeOperator::LessThan,
                    tzid: Some(Tzid(Tz::Europe__London)),
                },
                date_time: DateTime::LocalDateTime(
                    NaiveDateTime::new(
                        NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap(),
                        NaiveTime::from_hms_opt(15_u32, 0_u32, 0_u32).unwrap(),
                    )
                ),
            }.render_ical(),
            String::from("X-UNTIL;PROP=DTSTART;OP=LT;TZID=Europe/London:19960401T150000"),
        );

        assert_eq!(
            XUntilProperty {
                params: XUntilPropertyParams {
                    prop: WhereRangeProperty::DTEnd,
                    op: WhereUntilRangeOperator::LessEqualThan,
                    tzid: None,
                },
                date_time: DateTime::LocalDate(
                    NaiveDate::from_ymd_opt(1996_i32, 4_u32, 1_u32).unwrap()
                ),
            }.render_ical(),
            String::from("X-UNTIL;PROP=DTEND;OP=LTE:19960401"),
        );
    }
}
