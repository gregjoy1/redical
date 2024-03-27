use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::Quoted;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::cal_address::CalAddress;

// Sent By
//
// Parameter Name:  SENT-BY
//
// Purpose:  To specify the calendar user that is acting on behalf of
//    the calendar user specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     sentbyparam        = "SENT-BY" "=" DQUOTE cal-address DQUOTE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SentByParam(pub Quoted<CalAddress>);

impl ICalendarEntity for SentByParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "SENTBYPARAM",
            map(
                pair(
                    tag("SENT-BY"),
                    preceded(tag("="), cut(Quoted::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("SENT-BY={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(SentByParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_value_data_types::uri::Uri;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            SentByParam::parse_ical(r#"SENT-BY="mailto:ietf-calsch@example.org" TESTING"#.into()),
            (
                " TESTING",
                SentByParam(
                    Quoted(
                        CalAddress(
                            Uri(String::from("mailto:ietf-calsch@example.org"))
                        )
                    ),
                ),
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            SentByParam(
                Quoted(
                    CalAddress(
                        Uri(String::from("mailto:ietf-calsch@example.org"))
                    )
                ),
            ).render_ical(),
            String::from(r#"SENT-BY="mailto:ietf-calsch@example.org""#),
        );
    }
}
