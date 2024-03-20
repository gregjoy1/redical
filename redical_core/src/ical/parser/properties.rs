use nom::{
    branch::alt,
    bytes::complete::{escaped, tag, tag_no_case, take, take_while, take_while1},
    character::complete::{alphanumeric1, char, digit1, one_of, space1},
    combinator::{cut, map, opt, recognize},
    error::{context, ContextError, ErrorKind, ParseError, VerboseError},
    multi::{separated_list0, separated_list1},
    number::complete::double,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};

use crate::core::ical::parser::common;
use crate::core::ical::parser::common::ParserResult;

// text       = *(TSAFE-CHAR / ":" / DQUOTE / ESCAPED-CHAR)
//
// ESCAPED-CHAR = "\\" / "\;" / "\," / "\N" / "\n")
//    ; \\ encodes \, \N or \n encodes newline
//    ; \; encodes ;, \, encodes ,
// TSAFE-CHAR = %x20-21 / %x23-2B / %x2D-39 / %x3C-5B %x5D-7E / NON-US-ASCII
//    ; Any character except CTLs not needed by the current
pub fn value_text(input: &str) -> ParserResult<&str, &str> {
    common::parse_with_look_ahead_parser(common::value_text, look_ahead_property_parser)(input)
}

// paramtext     = *SAFE-CHAR
fn param_text(input: &str) -> ParserResult<&str, &str> {
    common::parse_with_look_ahead_parser(common::param_text, look_ahead_property_parser)(input)
}

// value         = *VALUE-CHAR
fn value(input: &str) -> ParserResult<&str, &str> {
    common::parse_with_look_ahead_parser(common::value, look_ahead_property_parser)(input)
}

// All iana registered icalendar properties
// https://www.iana.org/assignments/icalendar/icalendar.xhtml
//
// Use this over iana_token as it is too permissive and permits invalid non-vendor specific
// property names.
pub fn known_iana_properties(input: &str) -> ParserResult<&str, &str> {
    context(
        "IANA property",
        // Tuples are restricted to 21 elements, to accomodate 67 tags, nested
        // alt achieves the same with an insignificant impact on performance.
        alt((
            alt((
                tag("CALSCALE"),
                tag("METHOD"),
                tag("PRODID"),
                tag("VERSION"),
                tag("ATTACH"),
                tag("CATEGORIES"),
                tag("CLASS"),
                tag("COMMENT"),
                tag("DESCRIPTION"),
                tag("GEO"),
                tag("LOCATION"),
                tag("PERCENT-COMPLETE"),
                tag("PRIORITY"),
                tag("RESOURCES"),
                tag("STATUS"),
                tag("SUMMARY"),
            )),
            alt((
                tag("COMPLETED"),
                tag("DTEND"),
                tag("DUE"),
                tag("DTSTART"),
                tag("DURATION"),
                tag("FREEBUSY"),
                tag("TRANSP"),
                tag("TZID"),
                tag("TZNAME"),
                tag("TZOFFSETFROM"),
                tag("TZOFFSETTO"),
                tag("TZURL"),
                tag("ATTENDEE"),
                tag("CONTACT"),
                tag("ORGANIZER"),
                tag("RECURRENCE-ID"),
            )),
            alt((
                tag("RELATED-TO"),
                tag("URL"),
                tag("UID"),
                tag("EXDATE"),
                tag("EXRULE"),
                tag("RDATE"),
                tag("RRULE"),
                tag("ACTION"),
                tag("REPEAT"),
                tag("TRIGGER"),
                tag("CREATED"),
                tag("DTSTAMP"),
                tag("LAST-MODIFIED"),
                tag("SEQUENCE"),
                tag("REQUEST-STATUS"),
                tag("XML"),
            )),
            alt((
                tag("TZUNTIL"),
                tag("TZID-ALIAS-OF"),
                tag("BUSYTYPE"),
                tag("NAME"),
                tag("REFRESH-INTERVAL"),
                tag("SOURCE"),
                tag("COLOR"),
                tag("IMAGE"),
                tag("CONFERENCE"),
                tag("CALENDAR-ADDRESS"),
                tag("LOCATION-TYPE"),
                tag("PARTICIPANT-TYPE"),
                tag("RESOURCE-TYPE"),
                tag("STRUCTURED-DATA"),
                tag("STYLED-DESCRIPTION"),
                tag("ACKNOWLEDGED"),
                tag("PROXIMITY"),
                tag("CONCEPT"),
                tag("LINK"),
                tag("REFID"),
            )),
        )),
    )(input)
}

// name          = iana-token / x-name
pub fn name(input: &str) -> ParserResult<&str, &str> {
    context(
        "name",
        preceded(
            take_while(common::is_white_space_char),
            alt((known_iana_properties, common::x_name)),
        ),
    )(input)
}

pub fn look_ahead_property_parser(input: &str) -> ParserResult<&str, &str> {
    recognize(tuple((
        common::white_space1,
        name,
        alt((common::semicolon_delimeter, common::colon_delimeter)),
    )))(input)
}
