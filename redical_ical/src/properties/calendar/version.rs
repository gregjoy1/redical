use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    VerParams,
    VerParam,
    "VERSION",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Version
//
// Property Name:  VERSION
//
// Purpose:  This property specifies the identifier corresponding to the
//    highest version number or the minimum and maximum range of the
//    iCalendar specification that is required in order to interpret the
//    iCalendar object.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  This property MUST be specified once in an iCalendar
//    object.
//
// Description:  A value of "2.0" corresponds to this memo.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     version    = "VERSION" verparam ":" vervalue CRLF
//
//     verparam   = *(";" other-param)
//
//     vervalue   = "2.0"         ;This memo
//                / maxver
//                / (minver ";" maxver)
//
//     minver     = <A IANA-registered iCalendar version identifier>
//     ;Minimum iCalendar version needed to parse the iCalendar object.
//
//     maxver     = <A IANA-registered iCalendar version identifier>
//     ;Maximum iCalendar version needed to parse the iCalendar object.
//
// Example:  The following is an example of this property:
//
//     VERSION:2.0
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Version {
    pub params: VerParams,
    pub value: Text,
}

impl ICalendarEntity for Version {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "VERSION",
            preceded(
                tag("VERSION"),
                cut(
                    map(
                        pair(
                            opt(VerParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Version {
                                params: params.unwrap_or(VerParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("VERSION{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Version);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Version::parse_ical("VERSION:2.0".into()),
            (
                "",
                Version {
                    params: VerParams::default(),
                    value: Text(String::from("2.0")),
                },
            ),
        );

        assert_parser_output!(
            Version::parse_ical("VERSION;X-TEST=X_VALUE;TEST=VALUE:2.0".into()),
            (
                "",
                Version {
                    params: VerParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("2.0")),
                },
            ),
        );

        assert!(Version::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Version {
                params: VerParams::default(),
                value: Text(String::from("2.0")),
            }.render_ical(),
            String::from("VERSION:2.0"),
        );

        assert_eq!(
            Version {
                params: VerParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("2.0")),
            }.render_ical(),
            String::from("VERSION;X-TEST=X_VALUE;TEST=VALUE:2.0"),
        );
    }
}
