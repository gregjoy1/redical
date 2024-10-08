use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::duration::Duration;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct DurationPropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for DurationPropertyParams {
    define_property_params_ical_parser!(
        DurationPropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut DurationPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for DurationPropertyParams {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_params_with_context(&self, _context: Option<&RenderingContext>) -> ContentLineParams {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in self.other.clone().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        content_line_params
    }
}

impl From<DurationPropertyParams> for ContentLineParams {
    fn from(duration_params: DurationPropertyParams) -> Self {
        ContentLineParams::from(&duration_params)
    }
}

// Duration
//
// Property Name:  DURATION
//
// Purpose:  This property specifies a positive duration of time.
//
// Value Type:  DURATION
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property can be specified in "VEVENT", "VTODO", or
//    "VALARM" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     duration   = "DURATION" durparam ":" dur-value CRLF
//                  ;consisting of a positive duration of time.
//
//     durparam   = *(";" other-param)
//
// Example:  The following is an example of this property that specifies
//    an interval of time of one hour and zero minutes and zero seconds:
//
//     DURATION:PT1H0M0S
//
//    The following is an example of this property that specifies an
//    interval of time of 15 minutes.
//
//     DURATION:PT15M
//
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DurationProperty {
    pub params: DurationPropertyParams,
    pub duration: Duration,
}

impl DurationProperty {
    pub fn new_from_seconds(duration_in_seconds: &i64) -> Self {
        DurationProperty {
            params: DurationPropertyParams::default(),
            duration: Duration::from(duration_in_seconds.to_owned()),
        }
    }
}

impl ICalendarEntity for DurationProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DURATION",
            preceded(
                tag("DURATION"),
                cut(
                    map(
                        pair(
                            opt(DurationPropertyParams::parse_ical),
                            preceded(colon, Duration::parse_ical),
                        ),
                        |(params, duration)| {
                            DurationProperty {
                                params: params.unwrap_or(DurationPropertyParams::default()),
                                duration,
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
        self.duration.validate()?;

        Ok(())
    }
}

impl ICalendarProperty for DurationProperty {
    /// Build a `ContentLine` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "DURATION",
            (
                ContentLineParams::from(&self.params),
                self.duration.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for DurationProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(DurationProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DurationProperty::parse_ical("DURATION:PT1H0M0S DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DurationProperty {
                    params: DurationPropertyParams::default(),
                    duration: Duration {
                        positive_negative: None,
                        weeks: None,
                        days: None,
                        hours: Some(1),
                        minutes: Some(0),
                        seconds: Some(0),
                    },
                }
            )
        );

        assert_parser_output!(
            DurationProperty::parse_ical("DURATION;X-TEST=X_VALUE;TEST=VALUE:PT15M".into()),
            (
                "",
                DurationProperty {
                    params: DurationPropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    duration: Duration {
                        positive_negative: None,
                        weeks: None,
                        days: None,
                        hours: None,
                        minutes: Some(15),
                        seconds: None,
                    },
                },
            ),
        );

        assert!(DurationProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DurationProperty {
                params: DurationPropertyParams::default(),
                duration: Duration {
                    positive_negative: None,
                    weeks: None,
                    days: None,
                    hours: Some(1),
                    minutes: Some(0),
                    seconds: Some(0),
                },
            }.render_ical(),
            String::from("DURATION:PT1H0M0S"),
        );

        assert_eq!(
            DurationProperty {
                params: DurationPropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                duration: Duration {
                    positive_negative: None,
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: Some(15),
                    seconds: None,
                },
            }.render_ical(),
            String::from("DURATION;TEST=VALUE;X-TEST=X_VALUE:PT15M"),
        )
    }
}
