use nom::error::context;
use nom::branch::alt;
use nom::sequence::{pair, preceded};
use nom::multi::fold_many0;
use nom::combinator::{map, cut, opt};
use nom::bytes::complete::tag;

use crate::grammar::{semicolon, colon};

use crate::property_value_data_types::{
    uri::Uri,
    binary::Binary,
};

use crate::property_parameters::{
    encoding::EncodingParam,
    fmttype::FmttypeParam,
    iana::{IanaParam, IanaParams},
    experimental::{XParam, XParams},
};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

use crate::properties::define_property_params;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ValueParam {
    Binary,
}

impl ICalendarEntity for ValueParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "VALUEPARAM",
            map(
                pair(
                    tag("VALUE"),
                    preceded(tag("="), cut(tag("BINARY"))),
                ),
                |(_key, _value)| Self::Binary
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        String::from("VALUE=BINARY")
    }
}

impl_icalendar_entity_traits!(ValueParam);

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AttachValue {
    Uri(Uri),
    Binary(Binary),
}

impl ICalendarEntity for AttachValue {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ATTACHVALUE",
            alt((
                map(Uri::parse_ical, |uri| AttachValue::Uri(uri)),
                map(Binary::parse_ical, |binary| AttachValue::Binary(binary)),
            )),
        )(input)
    }

    fn render_ical(&self) -> String {
        match self {
            Self::Uri(uri) => uri.render_ical(),
            Self::Binary(binary) => binary.render_ical(),
        }
    }
}

impl_icalendar_entity_traits!(AttachValue);

define_property_params!(
    AttachParams,
    AttachParam,
    "ATTACHPARAM",
    (Encoding, EncodingParam, encoding, Option<EncodingParam>),
    (Value, ValueParam, value, Option<ValueParam>),
    (Fmttype, FmttypeParam, fmttype, Option<FmttypeParam>),
    (X, XParam, x, XParams),
    (Iana, IanaParam, iana, IanaParams),
);

// Attachment
//
// Property Name:  ATTACH
//
// Purpose:  This property provides the capability to associate a
//    document object with a calendar component.
//
// Value Type:  The default value type for this property is URI.  The
//    value type can also be set to BINARY to indicate inline binary
//    encoded content information.
//
// Property Parameters:  IANA, non-standard, inline encoding, and value
//    data type property parameters can be specified on this property.
//    The format type parameter can be specified on this property and is
//    RECOMMENDED for inline binary encoded content information.
//
// Conformance:  This property can be specified multiple times in a
//    "VEVENT", "VTODO", "VJOURNAL", or "VALARM" calendar component with
//    the exception of AUDIO alarm that only allows this property to
//    occur once.
//
//     attach     = "ATTACH" attachparam ( ":" uri ) /
//                  (
//                    ";" "ENCODING" "=" "BASE64"
//                    ";" "VALUE" "=" "BINARY"
//                    ":" binary
//                  )
//                  CRLF
//
//     attachparam = *(
//                 ;
//                 ; The following is OPTIONAL for a URI value,
//                 ; RECOMMENDED for a BINARY value,
//                 ; and MUST NOT occur more than once.
//                 ;
//                 (";" fmttypeparam) /
//                 ;
//                 ; The following is OPTIONAL,
//                 ; and MAY occur more than once.
//                 ;
//                 (";" other-param)
//                 ;
//                 )
//
// Example:  The following are examples of this property:
//
//     ATTACH:CID:jsmith.part3.960817T083000.xyzMail@example.com
//
//     ATTACH;FMTTYPE=application/postscript:ftp://example.com/pub/
//      reports/r-960812.ps
//
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Attach {
    pub params: AttachParams,
    pub value: AttachValue,
}

impl ICalendarEntity for Attach {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "ATTACH",
            preceded(
                tag("ATTACH"),
                cut(
                    map(
                        pair(
                            opt(AttachParams::parse_ical),
                            preceded(
                                colon,
                                AttachValue::parse_ical,
                            ),
                        ),
                        |(params, value)| {
                            // TODO: Validate URI vs Binary with params constraint.

                            Attach {
                                params: params.unwrap_or(AttachParams::default()),
                                value,
                            }
                        }
                    )
                )
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("ATTACH{}:{}", self.params.render_ical(), self.value.render_ical())
    }
}

impl_icalendar_entity_traits!(Attach);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    use crate::property_parameters::fmttype::Fmttype;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Attach::parse_ical("ATTACH:CID:jsmith.part3.960817T083000.xyzMail@example.com".into()),
            (
                "",
                Attach {
                    params: AttachParams::default(),
                    value: AttachValue::Uri(Uri(String::from("CID:jsmith.part3.960817T083000.xyzMail@example.com"))),
                },
            ),
        );

        assert_parser_output!(
            Attach::parse_ical("ATTACH;X-TEST=X_VALUE;TEST=VALUE;FMTTYPE=application/postscript:ftp://example.com/pub/reports/r-960812.ps".into()),
            (
                "",
                Attach {
                    params: AttachParams {
                        encoding: None,
                        value: None,
                        fmttype: Some(FmttypeParam(Fmttype { type_name: String::from("application"), subtype_name: String::from("postscript") })),
                        iana: IanaParams::from(vec![("TEST", "VALUE")]),
                        x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                    },
                    value: AttachValue::Uri(Uri(String::from("ftp://example.com/pub/reports/r-960812.ps"))),
                },
            ),
        );

        assert!(Attach::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Attach {
                params: AttachParams::default(),
                value: AttachValue::Uri(Uri(String::from("CID:jsmith.part3.960817T083000.xyzMail@example.com"))),
            }.render_ical(),
            String::from("ATTACH:CID:jsmith.part3.960817T083000.xyzMail@example.com"),
        );

        assert_eq!(
            Attach {
                params: AttachParams {
                    encoding: None,
                    value: None,
                    fmttype: Some(FmttypeParam(Fmttype { type_name: String::from("application"), subtype_name: String::from("postscript") })),
                    iana: IanaParams::from(vec![("TEST", "VALUE")]),
                    x: XParams::from(vec![("X-TEST", "X_VALUE")]),
                },
                value: AttachValue::Uri(Uri(String::from("ftp://example.com/pub/reports/r-960812.ps"))),
            }.render_ical(),
            String::from("ATTACH;FMTTYPE=application/postscript;X-TEST=X_VALUE;TEST=VALUE:ftp://example.com/pub/reports/r-960812.ps"),
        );
    }
}
