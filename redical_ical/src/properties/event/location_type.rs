use itertools::Itertools;

use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::separated_list1;
use nom::combinator::{recognize, map, cut, opt};

use crate::grammar::{tag, semicolon, colon, comma, x_name, iana_token, param_value};

use crate::values::text::Text;
use crate::values::list::List;

use crate::properties::{ICalendarProperty, ICalendarPropertyParams, define_property_params_ical_parser};

use crate::content_line::{ContentLineParams, ContentLine};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct LocationTypePropertyParams {
    pub other: HashMap<String, String>,
}

impl ICalendarEntity for LocationTypePropertyParams {
    define_property_params_ical_parser!(
        LocationTypePropertyParams,
        (
            pair(alt((x_name, iana_token)), cut(preceded(tag("="), recognize(separated_list1(comma, param_value))))),
            |params: &mut LocationTypePropertyParams, (key, value): (ParserInput, ParserInput)| params.other.insert(key.to_string(), value.to_string()),
        ),
    );

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_params_with_context(context).render_ical()
    }
}

impl ICalendarPropertyParams for LocationTypePropertyParams {
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

impl From<LocationTypePropertyParams> for ContentLineParams {
    fn from(types_params: LocationTypePropertyParams) -> Self {
        ContentLineParams::from(&types_params)
    }
}

// Location Type
//
// Property Name: LOCATION-TYPE
//
// Purpose: This property specifies the type(s) of a location.
//
// Value Type: The value type for this property is TEXT. The allowable values are defined below.
//
// Format Definition: This property is defined by the following notation:
//
//     loctype      = "LOCATION-TYPE" loctypeparam ":"
//                     text *("," text)
//                     CRLF
//
//     loctypeparam   = *(";" other-param)
//
// Multiple values may be used if the location has multiple purposes, for example, a hotel and a
// restaurant.
//
// Values for this parameter are taken from the values defined in Section 3 of [RFC4589]. New
// location types SHOULD be registered in the manner laid down in Section 5 of [RFC4589].
//
// Example:  The following are examples of this property:
//
//     LOCATION-TYPE:ONLINE,ZOOM
//
//     LOCATION-TYPE:HOTEL
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LocationTypeProperty {
    pub params: LocationTypePropertyParams,
    pub types: List<Text>,
}

impl ICalendarEntity for LocationTypeProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "LOCATION-TYPE",
            preceded(
                tag("LOCATION-TYPE"),
                cut(
                    map(
                        pair(
                            opt(LocationTypePropertyParams::parse_ical),
                            preceded(colon, List::parse_ical),
                        ),
                        |(params, types)| {
                            LocationTypeProperty {
                                params: params.unwrap_or(LocationTypePropertyParams::default()),
                                types,
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

impl ICalendarProperty for LocationTypeProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, _context: Option<&RenderingContext>) -> ContentLine {
        ContentLine::from((
            "LOCATION-TYPE",
            (
                ContentLineParams::from(&self.params),
                self.types.to_string(),
            )
        ))
    }
}

impl std::hash::Hash for LocationTypeProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(LocationTypeProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            LocationTypeProperty::parse_ical("LOCATION-TYPE:HOTEL,RESTAURANT DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                LocationTypeProperty {
                    params: LocationTypePropertyParams::default(),
                    types: List::from(vec![Text(String::from("HOTEL")), Text(String::from("RESTAURANT"))]),
                },
            ),
        );

        assert_parser_output!(
            LocationTypeProperty::parse_ical("LOCATION-TYPE;X-TEST=X_VALUE;TEST=VALUE:RESTAURANT".into()),
            (
                "",
                LocationTypeProperty {
                    params: LocationTypePropertyParams {
                        other: HashMap::from([
                            (String::from("X-TEST"), String::from("X_VALUE")),
                            (String::from("TEST"), String::from("VALUE")),
                        ]),
                    },
                    types: List::from(vec![Text(String::from("RESTAURANT"))]),
                },
            ),
        );

        assert!(LocationTypeProperty::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            LocationTypeProperty {
                params: LocationTypePropertyParams::default(),
                types: List::from(vec![Text(String::from("HOTEL")), Text(String::from("RESTAURANT"))]),
            }.render_ical(),
            String::from("LOCATION-TYPE:HOTEL,RESTAURANT"),
        );

        assert_eq!(
            LocationTypeProperty {
                params: LocationTypePropertyParams {
                    other: HashMap::from([
                        (String::from("X-TEST"), String::from("X_VALUE")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]),
                },
                types: List::from(vec![Text(String::from("RESTAURANT"))]),
            }.render_ical(),
            String::from("LOCATION-TYPE;TEST=VALUE;X-TEST=X_VALUE:RESTAURANT"),
        );
    }
}
