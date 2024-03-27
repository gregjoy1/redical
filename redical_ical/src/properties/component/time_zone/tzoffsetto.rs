use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::bytes::complete::tag;
use nom::multi::fold_many0;
use nom::combinator::{opt, map, cut};

use crate::grammar::{colon, semicolon};
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::property_value_data_types::utc_offset::UtcOffset;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    ToParams,
    ToParam,
    "TOPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Zone Offset To
//
// Property Name:  TZOFFSETTO
//
// Purpose:  This property specifies the offset that is in use in this
//    time zone observance.
//
// Value Type:  UTC-OFFSET
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property MUST be specified in "STANDARD" and
//    "DAYLIGHT" sub-components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     tzoffsetto = "TZOFFSETTO" toparam ":" utc-offset CRLF
//
//     toparam    = *(";" other-param)
//
// Example:  The following are examples of this property:
//
//     TZOFFSETTO:-0400
//
//     TZOFFSETTO:+1245
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzoffsetto {
    pub params: ToParams,
    pub value: UtcOffset,
}

impl ICalendarEntity for Tzoffsetto {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZOFFSETTO",
            preceded(
                tag("TZOFFSETTO"),
                cut(
                    map(
                        pair(
                            opt(ToParams::parse_ical),
                            preceded(
                                colon,
                                UtcOffset::parse_ical,
                            )
                        ),
                        |(params, value)| {
                            Tzoffsetto {
                                params: params.unwrap_or(ToParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZOFFSETTO{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Tzoffsetto);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::integer::Integer;
    use crate::grammar::PositiveNegative;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzoffsetto::parse_ical("TZOFFSETTO:-0500 TESTING".into()),
            (
                " TESTING",
                Tzoffsetto {
                    params: ToParams::default(),
                    value: UtcOffset {
                        positive_negative: PositiveNegative::Negative,
                        time_hour: Integer(5),
                        time_minute: Integer(00),
                        time_second: None,
                    },
                },
            )
        );

        assert_parser_output!(
            Tzoffsetto::parse_ical("TZOFFSETTO;X-TEST=X_VALUE;TEST=VALUE:+1345 TESTING".into()),
            (
                " TESTING",
                Tzoffsetto {
                    params: ToParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: UtcOffset {
                        positive_negative: PositiveNegative::Positive,
                        time_hour: Integer(13),
                        time_minute: Integer(45),
                        time_second: None,
                    },
                },
            )
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Tzoffsetto {
                params: ToParams::default(),
                value: UtcOffset {
                    positive_negative: PositiveNegative::Negative,
                    time_hour: Integer(5),
                    time_minute: Integer(00),
                    time_second: None,
                },
            }.render_ical(),
            String::from("TZOFFSETTO:-0500"),
        );

        assert_eq!(
            Tzoffsetto {
                params: ToParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: UtcOffset {
                    positive_negative: PositiveNegative::Positive,
                    time_hour: Integer(13),
                    time_minute: Integer(45),
                    time_second: None,
                },
            }.render_ical(),
            String::from("TZOFFSETTO;X-TEST=X_VALUE;TEST=VALUE:+1345"),
        );
    }
}
