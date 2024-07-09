use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::values::text::Text;

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct UIDPropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for UIDPropertyParams {
    define_property_params_ical_parser!(
        UIDPropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut UIDPropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for UIDPropertyParams {
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

impl From<UIDPropertyParams> for ContentLineParams {
    fn from(uid_params: UIDPropertyParams) -> Self {
        uid_params.to_content_line_params()
    }
}

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
pub struct UIDProperty {
    pub params: UIDPropertyParams,
    pub uid: Text,
}

impl PartialOrd for UIDProperty {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for UIDProperty {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.uid.to_string().cmp(&other.uid.to_string())
    }
}

impl ICalendarEntity for UIDProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "UID",
            preceded(
                tag("UID"),
                cut(
                    map(
                        pair(
                            opt(UIDPropertyParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, uid)| {
                            UIDProperty {
                                params: params.unwrap_or(UIDPropertyParams::default()),
                                uid,
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
}

impl ICalendarProperty for UIDProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "UID",
            (
                ContentLineParams::from(&self.params),
                self.uid.to_string(),
            )
        ))
    }
}

impl From<String> for UIDProperty {
    fn from(uid: String) -> Self {
        UIDProperty {
            params: UIDPropertyParams::default(),
            uid: uid.into(),
        }
    }
}

impl std::hash::Hash for UIDProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(UIDProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            UIDProperty::parse_ical(
                "UID:19960401T080045Z-4000F192713-0052@example.com DESCRIPTION:Description text".into()
            ),
            (
                " DESCRIPTION:Description text",
                UIDProperty {
                    params: UIDPropertyParams::default(),
                    uid: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
                },
            ),
        );

        assert_parser_output!(
            UIDProperty::parse_ical("UID;X-TEST=X_VALUE;TEST=VALUE:19960401T080045Z-4000F192713-0052@example.com".into()),
            (
                "",
                UIDProperty {
                    params: UIDPropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    uid: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
                },
            ),
        );

        assert!(UIDProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            UIDProperty {
                params: UIDPropertyParams::default(),
                uid: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("UID:19960401T080045Z-4000F192713-0052@example.com"),
        );

        assert_eq!(
            UIDProperty {
                params: UIDPropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                uid: Text(String::from("19960401T080045Z-4000F192713-0052@example.com")),
            }.render_ical(),
            String::from("UID;TEST=VALUE;X-TEST=X_VALUE:19960401T080045Z-4000F192713-0052@example.com"),
        );
    }
}
