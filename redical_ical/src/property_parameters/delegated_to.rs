use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::{List, Quoted};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::cal_address::CalAddress;

// Delegatees
//
// Parameter Name:  DELEGATED-TO
//
// Purpose:  To specify the calendar users to whom the calendar user
//    specified by the property has delegated participation.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     deltoparam = "DELEGATED-TO" "=" DQUOTE cal-address DQUOTE
//                  *("," DQUOTE cal-address DQUOTE)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeltoParam(pub List<Quoted<CalAddress>>);

impl ICalendarEntity for DeltoParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "DELTOPARAM",
            map(
                pair(
                    tag("DELEGATED-TO"),
                    preceded(tag("="), cut(List::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("DELEGATED-TO={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(DeltoParam);

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashSet;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::uri::Uri;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DeltoParam::parse_ical(r#"DELEGATED-TO="mailto:jsmith@example.com" TESTING"#.into()),
            (
                " TESTING",
                DeltoParam(
                    List(
                        HashSet::from([
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

        assert_parser_output!(
            DeltoParam::parse_ical(r#"DELEGATED-TO="mailto:jsmith@example.com","mailto:ajones@example.com" TESTING"#.into()),
            (
                " TESTING",
                DeltoParam(
                    List(
                        HashSet::from([
                            Quoted(
                                CalAddress(
                                    Uri(String::from("mailto:jsmith@example.com"))
                                )
                            ),
                            Quoted(
                                CalAddress(
                                    Uri(String::from("mailto:ajones@example.com"))
                                )
                            ),
                        ])
                    )
                ),
            ),
        );

        assert!(DeltoParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            DeltoParam(
                List(
                    HashSet::from([
                        Quoted(
                            CalAddress(
                                Uri(String::from("mailto:jsmith@example.com"))
                            )
                        ),
                    ])
                )
            ).render_ical(),
            String::from(r#"DELEGATED-TO="mailto:jsmith@example.com""#),
        );

        assert_eq!(
            DeltoParam(
                List(
                    HashSet::from([
                        Quoted(
                            CalAddress(
                                Uri(String::from("mailto:jsmith@example.com"))
                            )
                        ),
                        Quoted(
                            CalAddress(
                                Uri(String::from("mailto:ajones@example.com"))
                            )
                        ),
                    ])
                )
            ).render_ical(),
            String::from(r#"DELEGATED-TO="mailto:ajones@example.com","mailto:jsmith@example.com""#),
        )
    }
}
