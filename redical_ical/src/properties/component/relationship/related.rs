use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    reltype::ReltypeParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    RelParams,
    RelParam,
    "RELPARAM",
    (Reltype, ReltypeParam, reltype, Option<ReltypeParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Related To
//
// Property Name:  RELATED-TO
//
// Purpose:  This property is used to represent a relationship or
//    reference between one calendar component and another.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, and relationship type
//    property parameters can be specified on this property.
//
// Conformance:  This property can be specified in the "VEVENT",
//    "VTODO", and "VJOURNAL" calendar components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     related    = "RELATED-TO" relparam ":" text CRLF
//
//     relparam   = *(
//                ;
//                ; The following is OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" reltypeparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
//    The following is an example of this property:
//
//     RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com
//
//     RELATED-TO:19960401-080045-4000F192713-0052@example.com
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Related {
    pub params: RelParams,
    pub value: Text,
}

impl ICalendarEntity for Related {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RELATED-TO",
            preceded(
                tag("RELATED-TO"),
                cut(
                    map(
                        pair(
                            opt(RelParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Related {
                                params: params.unwrap_or(RelParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("RELATED-TO{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Related);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::reltype::Reltype;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Related::parse_ical(
                "RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com".into()
            ),
            (
                "",
                Related {
                    params: RelParams::default(),
                    value: Text(String::from("jsmith.part7.19960817T083000.xyzMail@example.com")),
                },
            ),
        );

        assert_parser_output!(
            Related::parse_ical("RELATED-TO;RELTYPE=CHILD;X-TEST=X_VALUE;TEST=VALUE:19960401-080045-4000F192713-0052@example.com".into()),
            (
                "",
                Related {
                    params: RelParams {
                        reltype: Some(ReltypeParam(Reltype::Child)),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from("19960401-080045-4000F192713-0052@example.com")),
                },
            ),
        );

        assert!(Related::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Related {
                params: RelParams::default(),
                value: Text(String::from("jsmith.part7.19960817T083000.xyzMail@example.com")),
            }.render_ical(),
            String::from("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com"),
        );

        assert_eq!(
            Related {
                params: RelParams {
                    reltype: Some(ReltypeParam(Reltype::Child)),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from("19960401-080045-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("RELATED-TO;RELTYPE=CHILD;X-TEST=X_VALUE;TEST=VALUE:19960401-080045-4000F192713-0052@example.com"),
        );
    }
}
