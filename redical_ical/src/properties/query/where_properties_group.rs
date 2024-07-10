use nom::error::context;
use nom::branch::alt;
use nom::multi::separated_list0;
use nom::sequence::{pair, preceded, terminated, delimited};
use nom::combinator::{map, cut, opt};

use crate::grammar::{tag, wsp};

use crate::properties::ICalendarProperty;

use crate::values::where_operator::WhereOperator;

use crate::content_line::ContentLine;

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

use crate::properties::query::{
    x_uid::XUIDProperty,
    x_geo::XGeoProperty,
    x_class::XClassProperty,
    x_related_to::XRelatedToProperty,
    x_categories::XCategoriesProperty,
    x_location_type::XLocationTypeProperty,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum GroupedWhereProperty {
    XUID(Option<WhereOperator>, XUIDProperty),
    XGeo(Option<WhereOperator>, XGeoProperty),
    XClass(Option<WhereOperator>, XClassProperty),
    XRelatedTo(Option<WhereOperator>, XRelatedToProperty),
    XCategories(Option<WhereOperator>, XCategoriesProperty),
    XLocationType(Option<WhereOperator>, XLocationTypeProperty),
    WherePropertiesGroup(Option<WhereOperator>, WherePropertiesGroup),
}

impl GroupedWhereProperty {
    fn get_external_operator(&self) -> &Option<WhereOperator> {
        match self {
            Self::XUID(external_operator, _) => external_operator,
            Self::XGeo(external_operator, _) => external_operator,
            Self::XClass(external_operator, _) => external_operator,
            Self::XRelatedTo(external_operator, _) => external_operator,
            Self::XCategories(external_operator, _) => external_operator,
            Self::XLocationType(external_operator, _) => external_operator,
            Self::WherePropertiesGroup(external_operator, _) => external_operator,
        }
    }

    fn get_property_content_line(&self, context: Option<&RenderingContext>) -> ContentLine {
        match self {
            Self::XUID(_, property) => property.to_content_line_with_context(context),
            Self::XGeo(_, property) => property.to_content_line_with_context(context),
            Self::XClass(_, property) => property.to_content_line_with_context(context),
            Self::XRelatedTo(_, property) => property.to_content_line_with_context(context),
            Self::XCategories(_, property) => property.to_content_line_with_context(context),
            Self::XLocationType(_, property) => property.to_content_line_with_context(context),
            Self::WherePropertiesGroup(_, property) => property.to_content_line_with_context(context),
        }
    }
}

impl ICalendarEntity for GroupedWhereProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "PROPERTY",
            preceded(
                opt(wsp),
                alt((
                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), WherePropertiesGroup::parse_ical),
                        |(external_operator, where_properties_group)| GroupedWhereProperty::WherePropertiesGroup(external_operator, where_properties_group),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XUIDProperty::parse_ical),
                        |(external_operator, x_uid_property)| GroupedWhereProperty::XUID(external_operator, x_uid_property),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XGeoProperty::parse_ical),
                        |(external_operator, x_geo_property)| GroupedWhereProperty::XGeo(external_operator, x_geo_property),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XClassProperty::parse_ical),
                        |(external_operator, x_class_property)| GroupedWhereProperty::XClass(external_operator, x_class_property),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XRelatedToProperty::parse_ical),
                        |(external_operator, x_related_to_property)| GroupedWhereProperty::XRelatedTo(external_operator, x_related_to_property),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XCategoriesProperty::parse_ical),
                        |(external_operator, x_categories_property)| GroupedWhereProperty::XCategories(external_operator, x_categories_property),
                    ),

                    map(
                        pair(opt(terminated(WhereOperator::parse_ical, wsp)), XLocationTypeProperty::parse_ical),
                        |(external_operator, x_location_type_property)| GroupedWhereProperty::XLocationType(external_operator, x_location_type_property),
                    ),
                )),
            )
        )(input)
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        self.to_content_line_with_context(context).render_ical()
    }
}

impl ICalendarProperty for GroupedWhereProperty {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine {
        let mut property_content_line = self.get_property_content_line(context);

        if let Some(external_operator) = self.get_external_operator() {
            if property_content_line.is_unstructured() {
                property_content_line.2 = format!("{} {}", external_operator.render_ical(), property_content_line.2);
            } else {
                property_content_line.0 = format!("{} {}", external_operator.render_ical(), property_content_line.0);
            }
        }

        property_content_line
    }
}

impl_icalendar_entity_traits!(GroupedWhereProperty);

/// Query CLASS where condition property.
///
/// Example:
///
/// (X-CLASS:PUBLIC)
/// (X-CLASS:PUBLIC AND X-RELATED-TO;RELTYPE=PARENT:parent.uid OR X-GEO;DIST=1.5KM:48.85299;2.36885)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WherePropertiesGroup {
    pub properties: Vec<GroupedWhereProperty>,
}

static MAX_NESTING_DEPTH: usize = 32;

// Thread specific static storage of the current parsed nested query group depth.
// We use this to keep track of and enforce a nesting limit to prevent a stack overflow.
thread_local! {
    pub static CURRENT_NESTED_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

impl ICalendarEntity for WherePropertiesGroup {
    // TODO: Document better...
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        // Increment the current nested depth count.
        CURRENT_NESTED_DEPTH.set(CURRENT_NESTED_DEPTH.get() + 1);

        // If the nested depth count exceeds the defined nesting limit, return an error to prevent
        // it nesting indefinitely resulting in a stack overflow.
        if CURRENT_NESTED_DEPTH.get() > MAX_NESTING_DEPTH {
            // Decrement the current nested depth count.
            CURRENT_NESTED_DEPTH.set(CURRENT_NESTED_DEPTH.get() - 1);

            return Err(
                nom::Err::Error(
                    ParserError::new(String::from("Nested depth exceeded limit"), input)
                )
            );
        }

        // Store the parser result to delay returning before decrementing the current nested depth
        // count.
        let parser_result =
            context(
                "GROUP",
                map(
                    delimited(
                        delimited(opt(wsp), tag("("), opt(wsp)),
                        cut(
                            separated_list0(
                                wsp,
                                GroupedWhereProperty::parse_ical,
                            ),
                        ),
                        preceded(opt(wsp), tag(")")),
                    ),
                    |properties| {
                        WherePropertiesGroup { properties }
                    },
                ),
            )(input);

        // Decrement the current nested depth count.
        CURRENT_NESTED_DEPTH.set(CURRENT_NESTED_DEPTH.get() - 1);

        // Return the deferred parser result.
        parser_result
    }

    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String {
        let properties: Vec<String> = self.properties.clone().into_iter().map(|where_property| where_property.render_ical_with_context(context)).collect();

        format!("({})", properties.join(" "))
    }
}

impl ICalendarProperty for WherePropertiesGroup {
    /// Build a `ContentLineParams` instance with consideration to the optionally provided
    /// `RenderingContext`.
    fn to_content_line_with_context(&self, context: Option<&RenderingContext>) -> ContentLine {
        let joined_properties =
            self.properties.iter()
                           .map(|property| property.render_ical_with_context(context))
                           .collect::<Vec<String>>()
                           .join(" ");

        ContentLine::new_unstructured(
            format!("({})", joined_properties)
        )
    }
}

impl std::hash::Hash for WherePropertiesGroup {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.render_ical().hash(state)
    }
}

impl_icalendar_entity_traits!(WherePropertiesGroup);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::ParserContext;

    use crate::properties::query::{
        x_class::XClassPropertyParams,
        x_categories::XCategoriesPropertyParams,
        x_location_type::XLocationTypePropertyParams,
    };

    use crate::values::list::List;
    use crate::values::text::Text;
    use crate::values::class::ClassValue;

    #[test]
    fn parse_ical_stack_overflow_fuzzing_failure_test() {
        assert!(
            WherePropertiesGroup::parse_ical(
                "(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((".into()
            ).is_err()
        );
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            WherePropertiesGroup::parse_ical(ParserInput::new_extra("() X-CATEGORIES:Categories text", ParserContext::Query)),
            (
                " X-CATEGORIES:Categories text",
                WherePropertiesGroup { properties: vec![] },
            ),
        );

        assert_parser_output!(
            WherePropertiesGroup::parse_ical(ParserInput::new_extra("(X-CLASS:PUBLIC,PRIVATE) X-CATEGORIES:Categories text", ParserContext::Query)),
            (
                " X-CATEGORIES:Categories text",
                WherePropertiesGroup {
                    properties: vec![
                        GroupedWhereProperty::XClass(
                            None,
                            XClassProperty {
                                params: XClassPropertyParams::default(),
                                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                            },
                        ),
                    ]
                },
            ),
        );

        assert_parser_output!(
            WherePropertiesGroup::parse_ical(ParserInput::new_extra("(X-CLASS:PUBLIC,PRIVATE OR X-CATEGORIES:APPOINTMENT,EDUCATION) X-CATEGORIES:Categories text", ParserContext::Query)),
            (
                " X-CATEGORIES:Categories text",
                WherePropertiesGroup {
                    properties: vec![
                        GroupedWhereProperty::XClass(
                            None,
                            XClassProperty {
                                params: XClassPropertyParams::default(),
                                classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                            },
                        ),
                        GroupedWhereProperty::XCategories(
                            Some(WhereOperator::Or),
                            XCategoriesProperty {
                                params: XCategoriesPropertyParams::default(),
                                categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                            },
                        ),
                    ]
                },
            ),
        );

        assert_parser_output!(
            WherePropertiesGroup::parse_ical(ParserInput::new_extra("(X-CLASS:PUBLIC X-CATEGORIES:APPOINTMENT (X-CLASS:PRIVATE X-CATEGORIES:EDUCATION)) X-CATEGORIES:Categories text", ParserContext::Query)),
            (
                " X-CATEGORIES:Categories text",
                WherePropertiesGroup {
                    properties: vec![
                        GroupedWhereProperty::XClass(
                            None,
                            XClassProperty {
                                params: XClassPropertyParams::default(),
                                classes: List::from(vec![ClassValue::Public]),
                            },
                        ),
                        GroupedWhereProperty::XCategories(
                            None,
                            XCategoriesProperty {
                                params: XCategoriesPropertyParams::default(),
                                categories: List::from(vec![Text(String::from("APPOINTMENT"))]),
                            },
                        ),
                        GroupedWhereProperty::WherePropertiesGroup(
                            None,
                            WherePropertiesGroup {
                                properties: vec![
                                    GroupedWhereProperty::XClass(
                                        None,
                                        XClassProperty {
                                            params: XClassPropertyParams::default(),
                                            classes: List::from(vec![ClassValue::Private]),
                                        },
                                    ),
                                    GroupedWhereProperty::XCategories(
                                        None,
                                        XCategoriesProperty {
                                            params: XCategoriesPropertyParams::default(),
                                            categories: List::from(vec![Text(String::from("EDUCATION"))]),
                                        },
                                    ),
                                ]
                            },
                        ),
                    ]
                },
            ),
        );

        assert_parser_output!(
            WherePropertiesGroup::parse_ical(ParserInput::new_extra("(X-CLASS:PUBLIC OR X-CATEGORIES:APPOINTMENT AND (X-CLASS:PRIVATE OR X-LOCATION-TYPE:ONLINE,ZOOM)) X-CATEGORIES:Categories text", ParserContext::Query)),
            (
                " X-CATEGORIES:Categories text",
                WherePropertiesGroup {
                    properties: vec![
                        GroupedWhereProperty::XClass(
                            None,
                            XClassProperty {
                                params: XClassPropertyParams::default(),
                                classes: List::from(vec![ClassValue::Public]),
                            },
                        ),
                        GroupedWhereProperty::XCategories(
                            Some(WhereOperator::Or),
                            XCategoriesProperty {
                                params: XCategoriesPropertyParams::default(),
                                categories: List::from(vec![Text(String::from("APPOINTMENT"))]),
                            },
                        ),
                        GroupedWhereProperty::WherePropertiesGroup(
                            Some(WhereOperator::And),
                            WherePropertiesGroup {
                                properties: vec![
                                    GroupedWhereProperty::XClass(
                                        None,
                                        XClassProperty {
                                            params: XClassPropertyParams::default(),
                                            classes: List::from(vec![ClassValue::Private]),
                                        },
                                    ),
                                    GroupedWhereProperty::XLocationType(
                                        Some(WhereOperator::Or),
                                        XLocationTypeProperty {
                                            params: XLocationTypePropertyParams::default(),
                                            types: List::from(vec![Text(String::from("ONLINE")), Text(String::from("ZOOM"))]),
                                        },
                                    ),
                                ]
                            },
                        ),
                    ]
                },
            ),
        );

        assert!(WherePropertiesGroup::parse_ical(":".into()).is_err());
        assert!(WherePropertiesGroup::parse_ical("X-CLASS;OP=WRONG:PUBLIC".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            WherePropertiesGroup { properties: vec![] }.render_ical(),
            String::from("()"),
        );

        assert_eq!(
            WherePropertiesGroup {
                properties: vec![
                    GroupedWhereProperty::XClass(
                        None,
                        XClassProperty {
                            params: XClassPropertyParams::default(),
                            classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                        },
                    ),
                ]
            }.render_ical(),
            String::from("(X-CLASS;OP=AND:PRIVATE,PUBLIC)"),
        );

        assert_eq!(
            WherePropertiesGroup {
                properties: vec![
                    GroupedWhereProperty::XClass(
                        None,
                        XClassProperty {
                            params: XClassPropertyParams::default(),
                            classes: List::from(vec![ClassValue::Public, ClassValue::Private]),
                        },
                    ),
                    GroupedWhereProperty::XCategories(
                        Some(WhereOperator::Or),
                        XCategoriesProperty {
                            params: XCategoriesPropertyParams::default(),
                            categories: List::from(vec![Text(String::from("APPOINTMENT")), Text(String::from("EDUCATION"))]),
                        },
                    ),
                ]
            }.render_ical(),
            String::from("(X-CLASS;OP=AND:PRIVATE,PUBLIC OR X-CATEGORIES;OP=AND:APPOINTMENT,EDUCATION)"),
        );

        assert_eq!(
            WherePropertiesGroup {
                properties: vec![
                    GroupedWhereProperty::XClass(
                        None,
                        XClassProperty {
                            params: XClassPropertyParams::default(),
                            classes: List::from(vec![ClassValue::Public]),
                        },
                    ),
                    GroupedWhereProperty::XCategories(
                        None,
                        XCategoriesProperty {
                            params: XCategoriesPropertyParams::default(),
                            categories: List::from(vec![Text(String::from("APPOINTMENT"))]),
                        },
                    ),
                    GroupedWhereProperty::WherePropertiesGroup(
                        None,
                        WherePropertiesGroup {
                            properties: vec![
                                GroupedWhereProperty::XClass(
                                    None,
                                    XClassProperty {
                                        params: XClassPropertyParams::default(),
                                        classes: List::from(vec![ClassValue::Private]),
                                    },
                                ),
                                GroupedWhereProperty::XCategories(
                                    None,
                                    XCategoriesProperty {
                                        params: XCategoriesPropertyParams::default(),
                                        categories: List::from(vec![Text(String::from("EDUCATION"))]),
                                    },
                                ),
                            ]
                        },
                    ),
                ]
            }.render_ical(),
            String::from("(X-CLASS;OP=AND:PUBLIC X-CATEGORIES;OP=AND:APPOINTMENT (X-CLASS;OP=AND:PRIVATE X-CATEGORIES;OP=AND:EDUCATION))"),
        );

        assert_eq!(
            WherePropertiesGroup {
                properties: vec![
                    GroupedWhereProperty::XClass(
                        None,
                        XClassProperty {
                            params: XClassPropertyParams::default(),
                            classes: List::from(vec![ClassValue::Public]),
                        },
                    ),
                    GroupedWhereProperty::XCategories(
                        Some(WhereOperator::Or),
                        XCategoriesProperty {
                            params: XCategoriesPropertyParams::default(),
                            categories: List::from(vec![Text(String::from("APPOINTMENT"))]),
                        },
                    ),
                    GroupedWhereProperty::WherePropertiesGroup(
                        Some(WhereOperator::And),
                        WherePropertiesGroup {
                            properties: vec![
                                GroupedWhereProperty::XClass(
                                    None,
                                    XClassProperty {
                                        params: XClassPropertyParams::default(),
                                        classes: List::from(vec![ClassValue::Private]),
                                    },
                                ),
                                GroupedWhereProperty::XUID(
                                    Some(WhereOperator::Or),
                                    XUIDProperty {
                                        uids: List::from(vec![Text(String::from("UID_ONE")), Text(String::from("UID_TWO"))]),
                                    },
                                ),
                            ]
                        },
                    ),
                ]
            }.render_ical(),
            String::from("(X-CLASS;OP=AND:PUBLIC OR X-CATEGORIES;OP=AND:APPOINTMENT AND (X-CLASS;OP=AND:PRIVATE OR X-UID:UID_ONE,UID_TWO))"),
        );
    }
}
