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
    UidParams,
    UidParam,
    "UIDPARAM",
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Unique Identifier
//
// Property Name:  UID
//
// Purpose:  This property defines the persistent, globally unique
//    identifier for the calendar component.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA and non-standard property parameters can
//    be specified on this property.
//
// Conformance:  The property MUST be specified in the "VEVENT",
//    "VTODO", "VJOURNAL", or "VFREEBUSY" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     uid        = "UID" uidparam ":" text CRLF
//
//     uidparam   = *(";" other-param)
//
// Example:  The following is an example of this property:
//
//     UID:19960401T080045Z-4000F192713-0052@example.com
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Uid {
    pub params: UidParams,
    pub value: Text,
}

impl ICalendarEntity for Uid {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "UID",
            preceded(
                tag("UID"),
                cut(
                    map(
                        pair(
                            opt(UidParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Uid {
                                params: params.unwrap_or(UidParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("UID{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Uid);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Uid::parse_ical(
                "UID:19960401T080045Z-4000F192713-0052@example.com".into()
            ),
            (
                "",
                Uid {
                    params: UidParams::default(),
                    value: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
                },
            ),
        );

        assert_parser_output!(
            Uid::parse_ical("UID;X-TEST=X_VALUE;TEST=VALUE:19960401T080045Z-4000F192713-0052@example.com".into()),
            (
                "",
                Uid {
                    params: UidParams {
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
                },
            ),
        );

        assert!(Uid::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Uid {
                params: UidParams::default(),
                value: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("UID:19960401T080045Z-4000F192713-0052@example.com"),
        );

        assert_eq!(
            Uid {
                params: UidParams {
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("UID;X-TEST=X_VALUE;TEST=VALUE:19960401T080045Z-4000F192713-0052@example.com"),
        );
    }
}
