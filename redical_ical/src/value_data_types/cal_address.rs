use nom::error::context;
use nom::combinator::map;

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};
use crate::value_data_types::uri::{uri, Uri};

/// Parse cal-address chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::value_data_types::cal_address::cal_address;
///
/// assert!(cal_address("ftp://ftp.is.co.za/rfc/rfc1808.txt".into()).is_ok());
/// assert!(cal_address("http://www.ietf.org/rfc/rfc2396.txt".into()).is_ok());
/// assert!(cal_address("ldap://[2001:db8::7]/c=GB?objectClass?one".into()).is_ok());
/// assert!(cal_address("mailto:John.Doe@example.com".into()).is_ok());
/// assert!(cal_address("news:comp.infosystems.www.servers.unix".into()).is_ok());
/// assert!(cal_address("tel:+1-816-555-1212".into()).is_ok());
/// assert!(cal_address("telnet://192.0.2.16:80/".into()).is_ok());
/// assert!(cal_address("urn:oasis:names:specification:docbook:dtd:xml:4.1.2".into()).is_ok());
///
/// assert!(cal_address("Abc".into()).is_err());
/// assert!(cal_address("cB+/=".into()).is_err());
/// assert!(cal_address(":".into()).is_err());
/// ```
///
/// cal-address        = uri
pub fn cal_address(input: ParserInput) -> ParserResult<ParserInput> {
    context("CAL-ADDRESS", uri)(input)
}

// Value Name:  CAL-ADDRESS
//
// Purpose:  This value type is used to identify properties that contain
//    a calendar user address.
//
// Format Definition:  This value type is defined by the following
//    notation:
//
//     cal-address        = uri
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct CalAddress(pub Uri);

impl ICalendarEntity for CalAddress {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(Uri::parse_ical, Self)(input)
    }

    fn render_ical(&self) -> String {
        self.0.render_ical()
    }
}

impl_icalendar_entity_traits!(CalAddress);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            CalAddress::parse_ical("ftp://ftp.is.co.za/rfc/rfc1808.txt TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("http://www.ietf.org/rfc/rfc2396.txt TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("http://www.ietf.org/rfc/rfc2396.txt"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("ldap://[2001:db8::7]/c=GB?objectClass?one TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("ldap://[2001:db8::7]/c=GB?objectClass?one"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("mailto:John.Doe@example.com TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("mailto:John.Doe@example.com"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("news:comp.infosystems.www.servers.unix TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("news:comp.infosystems.www.servers.unix"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("tel:+1-816-555-1212 TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("tel:+1-816-555-1212"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("telnet://192.0.2.16:80/ TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("telnet://192.0.2.16:80/"))),
            ),
        );

        assert_parser_output!(
            CalAddress::parse_ical("urn:oasis:names:specification:docbook:dtd:xml:4.1.2 TESTING".into()),
            (
                " TESTING",
                CalAddress(Uri(String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2"))),
            ),
        );

        assert!(CalAddress::parse_ical("Abc".into()).is_err());
        assert!(CalAddress::parse_ical("cB+/=".into()).is_err());
        assert!(CalAddress::parse_ical(":".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            CalAddress(Uri(String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt"))).render_ical(),
            String::from("ftp://ftp.is.co.za/rfc/rfc1808.txt"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("http://www.ietf.org/rfc/rfc2396.txt"))).render_ical(),
            String::from("http://www.ietf.org/rfc/rfc2396.txt"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("ldap://[2001:db8::7]/c=GB?objectClass?one"))).render_ical(),
            String::from("ldap://[2001:db8::7]/c=GB?objectClass?one"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("mailto:John.Doe@example.com"))).render_ical(),
            String::from("mailto:John.Doe@example.com"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("news:comp.infosystems.www.servers.unix"))).render_ical(),
            String::from("news:comp.infosystems.www.servers.unix"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("tel:+1-816-555-1212"))).render_ical(),
            String::from("tel:+1-816-555-1212"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("telnet://192.0.2.16:80/"))).render_ical(),
            String::from("telnet://192.0.2.16:80/"),
        );

        assert_eq!(
            CalAddress(Uri(String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2"))).render_ical(),
            String::from("urn:oasis:names:specification:docbook:dtd:xml:4.1.2"),
        );
    }
}
