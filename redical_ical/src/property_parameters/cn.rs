use nom::error::context;
use nom::sequence::{pair, preceded};
use nom::combinator::{map, cut};
use nom::bytes::complete::tag;

use crate::grammar::param_value;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// Common Name
//
// Parameter Name:  CN
//
// Purpose:  To specify the common name to be associated with the
//    calendar user specified by the property.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//   cnparam    = "CN" "=" param-value
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CnParam(pub String);

impl ICalendarEntity for CnParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CNPARAM",
            map(
                pair(
                    tag("CN"),
                    preceded(tag("="), cut(param_value)),
                ),
                |(_key, value)| Self(value.to_string())
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("CN={}", self.0)
    }
}

impl_icalendar_entity_traits!(CnParam);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            CnParam::parse_ical(r#"CN="John Smith" TESTING"#.into()),
            (
                " TESTING",
                CnParam(String::from(r#""John Smith""#)),
            ),
        );

        assert!(CnParam::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            CnParam(String::from(r#""John Smith""#)).render_ical(),
            String::from(r#"CN="John Smith""#),
        );
    }
}
