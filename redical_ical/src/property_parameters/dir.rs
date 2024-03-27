use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{List, Quoted};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::uri::Uri;

// Directory Entry Reference
//
// Parameter Name:  DIR
//
// Purpose:  To specify reference to a directory entry associated with
//    the calendar user specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     dirparam   = "DIR" "=" DQUOTE uri DQUOTE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DirParam(List<Quoted<Uri>>);

impl ICalendarEntity for DirParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DIRPARAM",
            map(
                pair(
                    tag("DIR"),
                    preceded(tag("="), cut(List::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DIR={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(DirParam);

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DirParam::parse_ical(r#"DIR="mailto:jsmith@example.com" TESTING"#.into()),
            (
                " TESTING",
                DirParam(
                    List(
                        HashSet::from([
                            Quoted(
                                Uri(String::from("mailto:jsmith@example.com"))
                            ),
                        ])
                    )
                ),
            ),
        );

        assert_parser_output!(
            DirParam::parse_ical(r#"DIR="mailto:jsmith@example.com","mailto:ajones@example.com" TESTING"#.into()),
            (
                " TESTING",
                DirParam(
                    List(
                        HashSet::from([
                            Quoted(
                                Uri(String::from("mailto:jsmith@example.com"))
                            ),
                            Quoted(
                                Uri(String::from("mailto:ajones@example.com"))
                            ),
                        ])
                    )
                ),
            ),
        );

        assert!(DirParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DirParam(
                List(
                    HashSet::from([
                        Quoted(
                            Uri(String::from("mailto:jsmith@example.com"))
                        ),
                    ])
                )
            ).render_ical(),
            String::from(r#"DIR="mailto:jsmith@example.com""#),
        );

        assert_eq!(
            DirParam(
                List(
                    HashSet::from([
                        Quoted(
                            Uri(String::from("mailto:jsmith@example.com"))
                        ),
                        Quoted(
                            Uri(String::from("mailto:ajones@example.com"))
                        ),
                    ])
                )
            ).render_ical(),
            String::from(r#"DIR="mailto:ajones@example.com","mailto:jsmith@example.com""#),
        )
    }
}
