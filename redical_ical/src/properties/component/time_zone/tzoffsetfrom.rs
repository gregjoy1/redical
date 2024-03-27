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
    FrmParams,
    FrmParam,
    "TZOFFSETFROM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Time Zone Offset From
//
// Property Name:  TZOFFSETFROM
//
// Purpose:  This property specifies the offset that is in use prior to
//    this time zone observance.
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
//     tzoffsetfrom       = "TZOFFSETFROM" frmparam ":" utc-offset
//                          CRLF
//
//     frmparam   = *(";" other-param)
//
// Example:  The following are examples of this property:
//
//     TZOFFSETFROM:-0500
//
//     TZOFFSETFROM:+1345
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Tzoffsetfrom {
    pub params: FrmParams,
    pub value: UtcOffset,
}

impl ICalendarEntity for Tzoffsetfrom {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "TZOFFSETFROM",
            preceded(
                tag("TZOFFSETFROM"),
                cut(
                    map(
                        pair(
                            opt(FrmParams::parse_ical),
                            preceded(
                                colon,
                                UtcOffset::parse_ical,
                            )
                        ),
                        |(params, value)| {
                            Tzoffsetfrom {
                                params: params.unwrap_or(FrmParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("TZOFFSETFROM{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Tzoffsetfrom);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::integer::Integer;
    use crate::grammar::PositiveNegative;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Tzoffsetfrom::parse_ical("TZOFFSETFROM:-0500 TESTING".into()),
            (
                " TESTING",
                Tzoffsetfrom {
                    params: FrmParams::default(),
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
            Tzoffsetfrom::parse_ical("TZOFFSETFROM;X-TEST=X_VALUE;TEST=VALUE:+1345 TESTING".into()),
            (
                " TESTING",
                Tzoffsetfrom {
                    params: FrmParams {
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
            Tzoffsetfrom {
                params: FrmParams::default(),
                value: UtcOffset {
                    positive_negative: PositiveNegative::Negative,
                    time_hour: Integer(5),
                    time_minute: Integer(00),
                    time_second: None,
                },
            }.render_ical(),
            String::from("TZOFFSETFROM:-0500"),
        );

        assert_eq!(
            Tzoffsetfrom {
                params: FrmParams {
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
            String::from("TZOFFSETFROM;X-TEST=X_VALUE;TEST=VALUE:+1345"),
        );
    }
}
