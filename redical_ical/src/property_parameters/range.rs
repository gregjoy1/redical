use nom::error::context;
use nom::sequence::tuple;
use nom::combinator::map;
use nom::bytes::complete::tag;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// Recurrence Identifier Range
//
// Parameter Name:  RANGE
//
// Purpose:  To specify the effective range of recurrence instances from
//    the instance specified by the recurrence identifier specified by
//    the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     rangeparam = "RANGE" "=" "THISANDFUTURE"
//     ; To specify the instance specified by the recurrence identifier
//     ; and all subsequent recurrence instances.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RangeParam(pub ());

impl ICalendarEntity for RangeParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "RANGEPARAM",
            map(
                tuple((
                    tag("RANGE"),
                    tag("="),
                    tag("THISANDFUTURE"),
                )),
                |_| Self(())
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        String::from("RANGE=THISANDFUTURE")
    }
}

impl_icalendar_entity_traits!(RangeParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            RangeParam::parse_ical(r#"RANGE=THISANDFUTURE TESTING"#.into()),
            (
                " TESTING",
                RangeParam(()),
            ),
        );

        assert!(RangeParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            RangeParam(()).render_ical(),
            String::from("RANGE=THISANDFUTURE"),
        );
    }
}
