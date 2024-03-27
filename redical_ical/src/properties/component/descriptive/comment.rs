use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};
use crate::property_value_data_types::text::Text;
use crate::property_parameters::{
    altrep::AltrepParam,
    language::LanguageParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

define_property_params!(
    CommParams,
    CommParam,
    "COMMPARAM",
    (Altrep, AltrepParam, altrep, Option<AltrepParam>),
    (Language, LanguageParam, language, Option<LanguageParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Comment
//
// Property Name:  COMMENT
//
// Purpose:  This property specifies non-processing information intended
//    to provide a comment to the calendar user.
//
// Value Type:  TEXT
//
// Property Parameters:  IANA, non-standard, alternate text
//    representation, and language property parameters can be specified
//    on this property.
//
// Conformance:  This property can be specified multiple times in
//    "VEVENT", "VTODO", "VJOURNAL", and "VFREEBUSY" calendar components
//    as well as in the "STANDARD" and "DAYLIGHT" sub-components.
//
// Format Definition:  This property is defined by the following
//    notation:
//
//     comment    = "COMMENT" commparam ":" text CRLF
//
//     commparam  = *(
//                ;
//                ; The following are OPTIONAL,
//                ; but MUST NOT occur more than once.
//                ;
//                (";" altrepparam) / (";" languageparam) /
//                ;
//                ; The following is OPTIONAL,
//                ; and MAY occur more than once.
//                ;
//                (";" other-param)
//                ;
//                )
//
// Example:  The following is an example of this property:
//
//     COMMENT:The meeting really needs to include both ourselves
//       and the customer. We can't hold this meeting without them.
//       As a matter of fact\, the venue for the meeting ought to be at
//       their site. - - John
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Comment {
    pub params: CommParams,
    pub value: Text,
}

impl ICalendarEntity for Comment {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "COMMENT",
            preceded(
                tag("COMMENT"),
                cut(
                    map(
                        pair(
                            opt(CommParams::parse_ical),
                            preceded(colon, Text::parse_ical),
                        ),
                        |(params, value)| {
                            Comment {
                                params: params.unwrap_or(CommParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("COMMENT{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Comment);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::language::Language;
    use crate::property_value_data_types::uri::Uri;

    use crate::grammar::Quoted;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Comment::parse_ical(
                r#"COMMENT:The meeting really needs to include both ourselves and the customer. As a matter of fact\, the venue for the meeting ought to be at their site. - - John"#.into()
            ),
            (
                "",
                Comment {
                    params: CommParams::default(),
                    value: Text(String::from(r#"The meeting really needs to include both ourselves and the customer. As a matter of fact\, the venue for the meeting ought to be at their site. - - John"#)),
                },
            ),
        );

        assert_parser_output!(
            Comment::parse_ical(r#"COMMENT;X-TEST=X_VALUE;TEST=VALUE;LANGUAGE=en-US;ALTREP="CID:part3.msg.970415T083000@example.com":Some comment text\, by Wilson in Maine"#.into()),
            (
                "",
                Comment {
                    params: CommParams {
                        altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                        language: Some(LanguageParam(Language(String::from("en-US")))),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: Text(String::from(r#"Some comment text\, by Wilson in Maine"#)),
                },
            ),
        );

        assert!(Comment::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Comment {
                params: CommParams::default(),
                value: Text(String::from(r#"The meeting really needs to include both ourselves and the customer. As a matter of fact\, the venue for the meeting ought to be at their site. - - John"#)),
            }.render_ical(),
            String::from(
                r#"COMMENT:The meeting really needs to include both ourselves and the customer. As a matter of fact\, the venue for the meeting ought to be at their site. - - John"#
            ),
        );

        assert_eq!(
            Comment {
                params: CommParams {
                    altrep: Some(AltrepParam(Quoted(Uri(String::from("CID:part3.msg.970415T083000@example.com"))))),
                    language: Some(LanguageParam(Language(String::from("en-US")))),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: Text(String::from(r#"Some comment text\, by Wilson in Maine"#)),
            }.render_ical(),
            String::from(r#"COMMENT;ALTREP="CID:part3.msg.970415T083000@example.com";LANGUAGE=en-US;X-TEST=X_VALUE;TEST=VALUE:Some comment text\, by Wilson in Maine"#),
        );
    }
}
