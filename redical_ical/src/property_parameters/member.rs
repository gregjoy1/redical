use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{List, Quoted};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::cal_address::CalAddress;

// Group or List Membership
//
// Parameter Name:  MEMBER
//
// Purpose:  To specify the group or list membership of the calendar
//    user specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     memberparam        = "MEMBER" "=" DQUOTE cal-address DQUOTE
//                          *("," DQUOTE cal-address DQUOTE)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MemberParam(pub List<Quoted<CalAddress>>);

impl ICalendarEntity for MemberParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "MEMBERPARAM",
            map(
                pair(
                    tag("MEMBER"),
                    preceded(tag("="), cut(List::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("MEMBER={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(MemberParam);

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::uri::Uri;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            MemberParam::parse_ical(r#"MEMBER="mailto:ietf-calsch@example.org","mailto:jsmith@example.com" TESTING"#.into()),
            (
                " TESTING",
                MemberParam(
                    List(
                        HashSet::from([
                            Quoted(
                                CalAddress(
                                    Uri(String::from("mailto:ietf-calsch@example.org"))
                                )
                            ),
                            Quoted(
                                CalAddress(
                                    Uri(String::from("mailto:jsmith@example.com"))
                                )
                            ),
                        ])
                    )
                ),
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            MemberParam(
                List(
                    HashSet::from([
                        Quoted(
                            CalAddress(
                                Uri(String::from("mailto:ietf-calsch@example.org"))
                            )
                        ),
                        Quoted(
                            CalAddress(
                                Uri(String::from("mailto:jsmith@example.com"))
                            )
                        ),
                    ])
                )
            ).render_ical(),
            String::from(r#"MEMBER="mailto:ietf-calsch@example.org","mailto:jsmith@example.com""#),
        );
    }
}
