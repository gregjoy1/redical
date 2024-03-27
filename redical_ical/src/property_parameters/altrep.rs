use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::Quoted;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::property_value_data_types::uri::Uri;

// Alternate Text Representation
//
// Parameter Name:  ALTREP
//
// Purpose:  To specify an alternate text representation for the
//   property value.
//
// Format Definition:  This property parameter is defined by the
//   following notation:
//
//  altrepparam = "ALTREP" "=" DQUOTE uri DQUOTE
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AltrepParam(pub Quoted<Uri>);

impl ICalendarEntity for AltrepParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ALTREPPARAM",
            map(
                pair(
                    tag("ALTREP"),
                    preceded(tag("="), cut(Quoted::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ALTREP={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(AltrepParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            AltrepParam::parse_ical(r#"ALTREP="CID:part3.msg.970415T083000@example.com" TESTING"#.into()),
            (
                " TESTING",
                AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com")))),
            ),
        );

        assert!(AltrepParam::parse_ical("ALTREP=CID:part3.msg.970415T083000@example.com".into()).is_err());
        assert!(AltrepParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com")))).render_ical(),
            String::from(r#"ALTREP="CID:part3.msg.970415T083000@example.com""#),
        );
    }
}
