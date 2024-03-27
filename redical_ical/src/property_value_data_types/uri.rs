use nom::sequence::{tuple, pair, preceded};
use nom::branch::alt;
use nom::error::context;
use nom::combinator::{recognize, opt, verify, map};
use nom::bytes::complete::{is_a, tag, take_while_m_n};
use nom::multi::{many0, many1, many_m_n, separated_list0};
use nom::character::{is_hex_digit, is_digit, is_alphabetic};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

/// URI         = scheme ":" hier-part [ "?" query ] [ "#" fragment ]
pub fn uri(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "URI",
        recognize(
            tuple(
                (
                    scheme,
                    tag(":"),
                    hier_part,
                    opt(pair(tag("?"), query)),
                    opt(pair(tag("#"), fragment)),
                )
            )
        )
    )(input)
}


/// scheme      = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )
pub fn scheme(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(
            alpha,
            many0(
                alt(
                    (
                        alpha,
                        digit,
                        tag("+"),
                        tag("-"),
                        tag("."),
                    )
                )
            )
        )
    )(input)
}

/// hier-part   = "//" authority path-abempty
///             / path-absolute
///             / path-rootless
///             / path-empty
///
/// path-empty    = 0<pchar>
pub fn hier_part(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        // opt covers path-empty
        opt(
            alt(
                (
                    recognize(
                        tuple(
                            (
                                tag("//"),
                                authority,
                                path_abempty,
                            )
                        )
                    ),
                    path_absolute,
                    path_rootless,
                )
            )
        )
    )(input)
}

/// authority   = [ userinfo "@" ] host [ ":" port ]
pub fn authority(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        tuple(
            (
                opt(pair(user_info, tag("@"))),
                host,
                opt(pair(tag(":"), port)),
            )
        )
    )(input)
}

/// userinfo    = *( unreserved / pct-encoded / sub-delims / ":" )
pub fn user_info(input: ParserInput) -> ParserResult<ParserInput> {
    alt(
        (
            unreserved,
            pct_encoded,
            sub_delims,
            tag(":"),
        )
    )(input)
}

/// host        = IP-literal / IPv4address / reg-name
pub fn host(input: ParserInput) -> ParserResult<ParserInput> {
    alt(
        (
            ip_literal,
            ip_v4_address,
            reg_name,
        )
    )(input)
}

/// port          = *DIGIT
pub fn port(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(many0(digit))(input)
}

/// IP-literal    = "[" ( IPv6address / IPvFuture  ) "]"
pub fn ip_literal(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        tuple(
            (
                tag("["),
                alt(
                    (
                        ip_v6_address, 
                        ip_v_future,
                    )
                ),
                tag("]"),
            )
        )
    )(input)
}

/// IPvFuture     = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )
pub fn ip_v_future(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        tuple(
            (
                tag("v"),
                hexdig,
                tag("."),
                alt(
                    (
                        unreserved,
                        sub_delims,
                        tag(":"),
                    )
                )
            )
        )
    )(input)
}

/// IPv6address   =                            6( h16 ":" ) ls32
///               /                       "::" 5( h16 ":" ) ls32
///               / [               h16 ] "::" 4( h16 ":" ) ls32
///               / [ *1( h16 ":" ) h16 ] "::" 3( h16 ":" ) ls32
///               / [ *2( h16 ":" ) h16 ] "::" 2( h16 ":" ) ls32
///               / [ *3( h16 ":" ) h16 ] "::"    h16 ":"   ls32
///               / [ *4( h16 ":" ) h16 ] "::"              ls32
///               / [ *5( h16 ":" ) h16 ] "::"              h16
///               / [ *6( h16 ":" ) h16 ] "::"
pub fn ip_v6_address(input: ParserInput) -> ParserResult<ParserInput> {
    fn many_h16_colon_m_n(min: usize, max: usize) -> impl FnMut(ParserInput) -> ParserResult<Vec<ParserInput>> {
        move |input: ParserInput| {
            verify(
                separated_list0(tag(":"), h16),
                |results: &Vec<ParserInput>| {
                    results.len() >= min && results.len() <= max
                }
            )(input)
        }
    }

    fn many_h16_colon_m(min: usize) -> impl FnMut(ParserInput) -> ParserResult<Vec<ParserInput>> {
        move |input: ParserInput| {
            verify(
                separated_list0(tag(":"), h16),
                |results: &Vec<ParserInput>| {
                    results.len() >= min
                }
            )(input)
        }
    }

    // Discard and use simpler "dumber" recognition approach as we do not care about the content of
    // the URI, just a vague recognition.
    //
    // alt(
    //     (
    //         recognize(                                            many_h16_colon_m_n(8, 8)),
    //         recognize(tuple((                          tag("::"), many_h16_colon_m_n(7, 7)))),
    //         recognize(tuple((opt(many_h16_colon_m(1)), tag("::"), many_h16_colon_m_n(6, 6)))),
    //         recognize(tuple((opt(many_h16_colon_m(2)), tag("::"), many_h16_colon_m_n(5, 5)))),
    //         recognize(tuple((opt(many_h16_colon_m(3)), tag("::"), many_h16_colon_m_n(4, 4)))),
    //         recognize(tuple((opt(many_h16_colon_m(4)), tag("::"), many_h16_colon_m_n(3, 3)))),
    //         recognize(tuple((opt(many_h16_colon_m(5)), tag("::"), many_h16_colon_m_n(2, 2)))),
    //         recognize(tuple((opt(many_h16_colon_m(6)), tag("::"), many_h16_colon_m_n(1, 1)))),
    //         recognize(tuple((opt(many_h16_colon_m(7)), tag("::"),                         ))),
    //     )
    // )(input)

    recognize(
        tuple((
            opt(many_h16_colon_m(0)),
            tag("::"),
            many_h16_colon_m_n(0, 7)
        ))
    )(input)
}

/// h16           = 1*4HEXDIG
pub fn h16(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many_m_n(1, 4, hexdig)
    )(input)
}

/// ls32          = ( h16 ":" h16 ) / IPv4address
pub fn ls32(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(h16, h16)
    )(input)
}

/// IPv4address   = dec-octet "." dec-octet "." dec-octet "." dec-octet
pub fn ip_v4_address(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        tuple(
            (
                dec_octet,
                tag("."),
                dec_octet,
                tag("."),
                dec_octet,
                tag("."),
                dec_octet,
            )
        )
    )(input)
}

/// dec-octet     = DIGIT                 ; 0-9
///               / %x31-39 DIGIT         ; 10-99
///               / "1" 2DIGIT            ; 100-199
///               / "2" %x30-34 DIGIT     ; 200-249
///               / "25" %x30-35          ; 250-255
pub fn dec_octet(input: ParserInput) -> ParserResult<ParserInput> {
    verify(
        recognize(many_m_n(1, 3, digit)),
        |value| {
            value.to_string().parse::<u8>().is_ok()
        }
    )(input)
}

/// reg-name      = *( unreserved / pct-encoded / sub-delims )
pub fn reg_name(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many0(
            alt(
                (
                    unreserved,
                    pct_encoded,
                    sub_delims,
                )
            )
        )
    )(input)
}

/// path          = path-abempty    ; begins with "/" or is empty
///               / path-absolute   ; begins with "/" but not "//"
///               / path-noscheme   ; begins with a non-colon segment
///               / path-rootless   ; begins with a segment
///               / path-empty      ; zero characters
///
/// path-empty    = 0<pchar>
pub fn path(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        // opt covers path-empty
        opt(
            alt(
                (
                    path_abempty,
                    path_absolute,
                    path_noscheme,
                    path_rootless,
                )
            )
        )
    )(input)
}

/// path-abempty  = *( "/" segment )
pub fn path_abempty(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many0(
            pair(
                tag("/"),
                segment,
            )
        )
    )(input)
}

/// path-absolute = "/" [ segment-nz *( "/" segment ) ]
pub fn path_absolute(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        preceded(
            tag("/"),
            pair(
                segment_nz,
                many0(
                    pair(
                        tag("/"),
                        segment,
                    )
                )
            )
        )
    )(input)
}

/// path-noscheme = segment-nz-nc *( "/" segment )
pub fn path_noscheme(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(
            segment_nz_nc,
            many0(
                pair(
                    tag("/"),
                    segment,
                )
            ),
        )
    )(input)
}

/// path-rootless = segment-nz *( "/" segment )
pub fn path_rootless(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(
            segment_nz,
            many0(
                pair(
                    tag("/"),
                    segment,
                )
            ),
        )
    )(input)
}

/// segment       = *pchar
pub fn segment(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(many0(pchar))(input)
}

/// segment-nz    = 1*pchar
pub fn segment_nz(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(many1(pchar))(input)
}

/// segment-nz-nc = 1*( unreserved / pct-encoded / sub-delims / "@" )
///               ; non-zero-length segment without any colon ":"
pub fn segment_nz_nc(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many1(
            alt(
                (
                    unreserved,
                    pct_encoded,
                    sub_delims,
                    tag("@"),
                )
            )
        )
    )(input)
}

/// pchar         = unreserved / pct-encoded / sub-delims / ":" / "@"
pub fn pchar(input: ParserInput) -> ParserResult<ParserInput> {
    alt(
        (
            unreserved,
            pct_encoded,
            sub_delims,
            tag(":"),
            tag("@"),
        )
    )(input)
}

/// query         = *( pchar / "/" / "?" )
pub fn query(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many0(
            alt((pchar, tag("/"), tag("?")))
        )
    )(input)
}

/// fragment      = *( pchar / "/" / "?" )
pub fn fragment(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many0(
            alt((pchar, tag("/"), tag("?")))
        )
    )(input)
}

/// pct-encoded   = "%" HEXDIG HEXDIG
pub fn pct_encoded(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        tuple((tag("%"), hexdig, hexdig))
    )(input) 
}

/// unreserved    = ALPHA / DIGIT / "-" / "." / "_" / "~"
pub fn unreserved(input: ParserInput) -> ParserResult<ParserInput> {
    alt((alpha, digit, is_a("-._~")))(input)
}

/// reserved      = gen-delims / sub-delims
pub fn reserved(input: ParserInput) -> ParserResult<ParserInput> {
    alt((gen_delims, sub_delims))(input)
}

/// gen-delims    = ":" / "/" / "?" / "#" / "[" / "]" / "@"
pub fn gen_delims(input: ParserInput) -> ParserResult<ParserInput> {
    is_a(":/?#[]@")(input)
}

/// sub-delims    = "!" / "$" / "&" / "'" / "(" / ")"
///               / "*" / "+" / "," / ";" / "="
pub fn sub_delims(input: ParserInput) -> ParserResult<ParserInput> {
    is_a("!$&'()*+,;=")(input)
}

/// Matches single hex digit character
pub fn hexdig(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |value| is_hex_digit(value as u8))(input)
}

/// Matches single alphabetic character
pub fn alpha(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |value| is_alphabetic(value as u8))(input)
}

/// Matches single digit character
pub fn digit(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |value| is_digit(value as u8))(input)
}

impl_icalendar_entity_traits!(Uri);

// Value Name:  URI
//
// Purpose:  This value type is used to identify values that contain a
//    uniform resource identifier (URI) type of reference to the
//    property value.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     uri = <As defined in Section 3 of [RFC3986]>
//     https://datatracker.ietf.org/doc/html/rfc3986#section-3
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Uri(pub String);

impl ICalendarEntity for Uri {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(uri, |value: ParserInput| Self(value.to_string()))(input)
    }

    fn render_ical(&self) -> String {
        self.0.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Uri::parse_ical("ftp://ftp.is.co.za/rfc/rfc1808.txt TESTING".into()),
            (
                " TESTING",
                Uri(String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("http://www.ietf.org/rfc/rfc2396.txt TESTING".into()),
            (
                " TESTING",
                Uri(String::from("http://www.ietf.org/rfc/rfc2396.txt")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("ldap://[2001:db8::7]/c=GB?objectClass?one TESTING".into()),
            (
                " TESTING",
                Uri(String::from("ldap://[2001:db8::7]/c=GB?objectClass?one")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("mailto:John.Doe@example.com TESTING".into()),
            (
                " TESTING",
                Uri(String::from("mailto:John.Doe@example.com")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("news:comp.infosystems.www.servers.unix TESTING".into()),
            (
                " TESTING",
                Uri(String::from("news:comp.infosystems.www.servers.unix")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("tel:+1-816-555-1212 TESTING".into()),
            (
                " TESTING",
                Uri(String::from("tel:+1-816-555-1212")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("telnet://192.0.2.16:80/ TESTING".into()),
            (
                " TESTING",
                Uri(String::from("telnet://192.0.2.16:80/")),
            ),
        );

        assert_parser_output!(
            Uri::parse_ical("urn:oasis:names:specification:docbook:dtd:xml:4.1.2 TESTING".into()),
            (
                " TESTING",
                Uri(String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2")),
            ),
        );

        assert!(Uri::parse_ical("Abc".into()).is_err());
        assert!(Uri::parse_ical("cB+/=".into()).is_err());
        assert!(Uri::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            Uri(String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt")).render_ical(),
            String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt"),
        );

        assert_eq!(
            Uri(String::from("http://www.ietf.org/rfc/rfc2396.txt")).render_ical(),
            String::from("http://www.ietf.org/rfc/rfc2396.txt"),
        );

        assert_eq!(
            Uri(String::from("ldap://[2001:db8::7]/c=GB?objectClass?one")).render_ical(),
            String::from("ldap://[2001:db8::7]/c=GB?objectClass?one"),
        );

        assert_eq!(
            Uri(String::from("mailto:John.Doe@example.com")).render_ical(),
            String::from("mailto:John.Doe@example.com"),
        );

        assert_eq!(
            Uri(String::from("news:comp.infosystems.www.servers.unix")).render_ical(),
            String::from("news:comp.infosystems.www.servers.unix"),
        );

        assert_eq!(
            Uri(String::from("tel:+1-816-555-1212")).render_ical(),
            String::from("tel:+1-816-555-1212"),
        );

        assert_eq!(
            Uri(String::from("telnet://192.0.2.16:80/")).render_ical(),
            String::from("telnet://192.0.2.16:80/"),
        );

        assert_eq!(
            Uri(String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2")).render_ical(),
            String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2"),
        );
    }
}
