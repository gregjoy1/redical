use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};
use nom::bytes::complete::tag;

use crate::property_value_data_types::text::Text;
use crate::property_parameters::reltype::{Reltype, ReltypeParam};

use crate::grammar::{semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::define_property_params_ical_parser;

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RelatedToPropertyParams {
    pub reltype: Option<Reltype>,
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for RelatedToPropertyParams {
    define_property_params_ical_parser!(
        RelatedToPropertyParams,
        (
            ReltypeParam::parse_ical,
            |params: &mut RelatedToPropertyParams, reltype_param: ReltypeParam| params.reltype = Some(reltype_param.0),
        ),
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut RelatedToPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical(&self) -> String {
        ContentLineParams::from(self).render_ical()
    }
}

impl From<&RelatedToPropertyParams> for ContentLineParams {
    fn from(related_to_params: &RelatedToPropertyParams) -> Self {
        let mut content_line_params = ContentLineParams::default();

        for (key, value) in related_to_params.other.to_owned().into_iter().sorted() {
            content_line_params.insert(key.to_owned(), value.to_owned());
        }

        if let Some(reltype) = related_to_params.reltype.as_ref() {
            content_line_params.insert(String::from("RELTYPE"), reltype.render_ical());
        }

        content_line_params
    }
}

impl From<RelatedToPropertyParams> for ContentLineParams {
    fn from(related_to_params: RelatedToPropertyParams) -> Self {
        ContentLineParams::from(&related_to_params)
    }
}

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
pub struct RelatedToProperty {
    pub params: RelatedToPropertyParams,
    pub value: Text,
}

impl ICalendarEntity for RelatedToProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RELATED-TO",
            preceded(
                tag("RELATED-TO"),
                cut(
                    map(
                        pair(
                            opt(RelatedToPropertyParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            RelatedToProperty {
                                params: params.unwrap_or(RelatedToPropertyParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }


    fn render_ical(&self) -> String {
        ContentLine::from(self).render_ical()
    }
}

impl From<&RelatedToProperty> for ContentLine {
    fn from(related_to_property: &RelatedToProperty) -> Self {
        ContentLine::from((
            "RELATED-TO",
            (
                ContentLineParams::from(&related_to_property.params),
                related_to_property.value.to_string(),
            )
        ))
    }
}

impl_icalendar_entity_traits!(RelatedToProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RelatedToProperty::parse_ical(
                "RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com DESCRIPTION:Description text".into()
            ),
            (
                " DESCRIPTION:Description text",
                RelatedToProperty {
                    params: RelatedToPropertyParams::default(),
                    value: Text(String::from("jsmith.part7.19960817T083000.xyzMail@example.com")),
                },
            ),
        );

        assert_parser_output!(
            RelatedToProperty::parse_ical("RELATED-TO;RELTYPE=CHILD;X-TEST=X_VALUE;TEST=VALUE:19960401-080045-4000F192713-0052@example.com".into()),
            (
                "",
                RelatedToProperty {
                    params: RelatedToPropertyParams {
                        reltype: Some(Reltype::Child),
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    value: Text(String::from("19960401-080045-4000F192713-0052@example.com")),
                },
            ),
        );

        assert!(RelatedToProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RelatedToProperty {
                params: RelatedToPropertyParams::default(),
                value: Text(String::from("jsmith.part7.19960817T083000.xyzMail@example.com")),
            }.render_ical(),
            String::from("RELATED-TO:jsmith.part7.19960817T083000.xyzMail@example.com"),
        );

        assert_eq!(
            RelatedToProperty {
                params: RelatedToPropertyParams {
                    reltype: Some(Reltype::Child),
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                value: Text(String::from("19960401-080045-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("RELATED-TO;TEST=VALUE;X-TEST=X_VALUE;RELTYPE=CHILD:19960401-080045-4000F192713-0052@example.com"),
        );
    }
}
