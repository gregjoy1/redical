use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::duration::Duration as DurValue;

use crate::properties::define_property_params;

define_property_params!(
    DurParams,
    DurParam,
    "DURPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

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
pub struct Duration {
    pub params: DurParams,
    pub value: DurValue,
}

impl ICalendarEntity for Duration {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DURATION",
            preceded(
                tag("DURATION"),
                cut(
                    map(
                        pair(
                            opt(DurParams::parse_ical),
                            preceded(colon, DurValue::parse_ical),
                        ),
                        |(params, value)| {
                            Duration {
                                params: params.unwrap_or(DurParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DURATION{}:{}", self.params.render_ical(), self.value.render_ical())
    }

    fn validate(&self) -> Result<(), String> {
        self.value.validate()?;

        Ok(())
    }
}

impl_icalendar_entity_traits!(Duration);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Duration::parse_ical("DURATION:PT1H0M0S".into()),
            (
                "",
                Duration {
                    params: DurParams::default(),
                    value: DurValue {
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
            Duration::parse_ical("DURATION;X-TEST=X_VALUE;TEST=VALUE:PT15M".into()),
            (
                "",
                Duration {
                    params: DurParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: DurValue {
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

        assert!(Duration::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Duration {
                params: DurParams::default(),
                value: DurValue {
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
            Duration {
                params: DurParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: DurValue {
                    positive_negative: None,
                    weeks: None,
                    days: None,
                    hours: None,
                    minutes: Some(15),
                    seconds: None,
                },
            }.render_ical(),
            String::from("DURATION;X-TEST=X_VALUE;TEST=VALUE:PT15M"),
        )
    }
}
