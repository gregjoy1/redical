use nom::error::context;
use nom::branch::alt;
use nom::multi::{many0, many1, many_m_n};
use nom::sequence::{tuple, pair, preceded, terminated};
use nom::combinator::{recognize, opt, map, cut, not};
use nom::bytes::complete::{tag, take_while_m_n};
use nom::character::{is_alphabetic, is_digit};

use crate::grammar::{latin_capital_letter_x, hyphen_minus};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

// Language-Tag  = langtag             ; normal language tags
//               / privateuse          ; private use tag
//               / grandfathered       ; grandfathered tags
fn language_tag(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        langtag,
        privateuse,
        grandfathered,
    ))(input)
}

// langtag       = language
//                 ["-" script]
//                 ["-" region]
//                 *("-" variant)
//                 *("-" extension)
//                 ["-" privateuse]
fn langtag(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        preceded(
            language,
            cut(
                tuple((
                    opt(pair(hyphen_minus, script)),
                    opt(pair(hyphen_minus, region)),
                    many0(pair(hyphen_minus, variant)),
                    many0(pair(hyphen_minus, extension)),
                    opt(pair(hyphen_minus, privateuse)),
                ))
            )
        )
    )(input)
}

// language      = 2*3ALPHA            ; shortest ISO 639 code
//                 ["-" extlang]       ; sometimes followed by
//                                     ; extended language subtags
//               / 4ALPHA              ; or reserved for future use
//               / 5*8ALPHA            ; or registered language subtag
fn language(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        alt((
            recognize(pair(many_m_n(2, 3, alpha), opt(pair(hyphen_minus, extlang)))),
            recognize(many_m_n(4, 4, alpha)),
            recognize(many_m_n(5, 8, alpha)),
        ))
    )(input)
}

// extlang       = 3ALPHA              ; selected ISO 639 codes
//                 *2("-" 3ALPHA)      ; permanently reserved
fn extlang(input: ParserInput) -> ParserResult<ParserInput> {
    // Use negative lookaead to ensure that it does not greedily eat into a series of alphabetic
    // characters with a length greater than 3.
    fn parse_3alpha(input: ParserInput) -> ParserResult<ParserInput> {
        recognize(
            terminated(
                many_m_n(3, 3, alpha),
                not(alpha),
            )
        )(input)
    }

    recognize(
        tuple((
            parse_3alpha,
            many_m_n(0, 2, pair(hyphen_minus, parse_3alpha)),
        )),
    )(input)
}

// script        = 4ALPHA              ; ISO 15924 code
fn script(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        many_m_n(4, 4, alpha),
    )(input)
}

// region        = 2ALPHA              ; ISO 3166-1 code
//               / 3DIGIT              ; UN M.49 code
fn region(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        recognize(
            many_m_n(2, 2, alpha),
        ),
        recognize(
            many_m_n(3, 3, digit),
        ),
    ))(input)
}

// variant       = 5*8alphanum         ; registered variants
//               / (DIGIT 3alphanum)
fn variant(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        recognize(
            many_m_n(5, 8, alphanum),
        ),
        recognize(
            pair(
                digit,
                many_m_n(3, 3, alphanum),
            )
        ),
    ))(input)
}

// extension     = singleton 1*("-" (2*8alphanum))
fn extension(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(
            singleton,
            many1(
                pair(
                    hyphen_minus,
                    many_m_n(2, 8, alphanum),
                )
            ),
        )
    )(input)
}

//                                     ; Single alphanumerics
//                                     ; "x" reserved for private use
// singleton     = DIGIT               ; 0 - 9
//               / %x41-57             ; A - W
//               / %x59-5A             ; Y - Z
//               / %x61-77             ; a - w
//               / %x79-7A             ; y - z
fn singleton(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |chr: char| {
        is_digit(chr as u8) ||
        (chr >= '\x41' && chr <= '\x57') ||
        (chr >= '\x59' && chr <= '\x5A') ||
        (chr >= '\x61' && chr <= '\x77') ||
        (chr >= '\x79' && chr <= '\x7A')
    })(input)
}

// privateuse    = "x" 1*("-" (1*8alphanum))
fn privateuse(input: ParserInput) -> ParserResult<ParserInput> {
    recognize(
        pair(
            latin_capital_letter_x,
            many1(
                pair(
                    hyphen_minus,
                    many_m_n(1, 8, alphanum),
                )
            ),
        )
    )(input)
}

// grandfathered = irregular           ; non-redundant tags registered
//               / regular             ; during the RFC 3066 era
fn grandfathered(input: ParserInput) -> ParserResult<ParserInput> {
    alt((irregular, regular))(input)
}

// irregular     = "en-GB-oed"         ; irregular tags do not match
//               / "i-ami"             ; the 'langtag' production and
//               / "i-bnn"             ; would not otherwise be
//               / "i-default"         ; considered 'well-formed'
//               / "i-enochian"        ; These tags are all valid,
//               / "i-hak"             ; but most are deprecated
//               / "i-klingon"         ; in favor of more modern
//               / "i-lux"             ; subtags or subtag
//               / "i-mingo"           ; combination
//               / "i-navajo"
//               / "i-pwn"
//               / "i-tao"
//               / "i-tay"
//               / "i-tsu"
//               / "sgn-BE-FR"
//               / "sgn-BE-NL"
//               / "sgn-CH-DE"
fn irregular(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        tag("en-GB-oed"),
        tag("i-ami"),
        tag("i-bnn"),
        tag("i-default"),
        tag("i-enochian"),
        tag("i-hak"),
        tag("i-klingon"),
        tag("i-lux"),
        tag("i-mingo"),
        tag("i-navajo"),
        tag("i-pwn"),
        tag("i-tao"),
        tag("i-tay"),
        tag("i-tsu"),
        tag("sgn-BE-FR"),
        tag("sgn-BE-NL"),
        tag("sgn-CH-DE"),
    ))(input)
}

// regular       = "art-lojban"        ; these tags match the 'langtag'
//               / "cel-gaulish"       ; production, but their subtags
//               / "no-bok"            ; are not extended language
//               / "no-nyn"            ; or variant subtags: their meaning
//               / "zh-guoyu"          ; is defined by their registration
//               / "zh-hakka"          ; and all of these are deprecated
//               / "zh-min"            ; in favor of a more modern
//               / "zh-min-nan"        ; subtag or sequence of subtags
//               / "zh-xiang"
fn regular(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        tag("art-lojban"),
        tag("cel-gaulish"),
        tag("no-bok"),
        tag("no-nyn"),
        tag("zh-guoyu"),
        tag("zh-hakka"),
        tag("zh-min-nan"),
        tag("zh-min"),
        tag("zh-xiang"),
    ))(input)
}

// alphanum      = (ALPHA / DIGIT)     ; letters and numbers
fn alphanum(input: ParserInput) -> ParserResult<ParserInput> {
    alt((alpha, digit))(input)
}

fn alpha(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |chr| is_alphabetic(chr as u8))(input)
}

fn digit(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, |chr| is_digit(chr as u8))(input)
}

// language = Language-Tag
//            ; As defined in [RFC5646].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Language(pub String);

impl ICalendarEntity for Language {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(language_tag, |value| Self(value.to_string()))(input)
    }

    fn render_ical(&self) -> String {
        self.0.to_string()
    }
}

// Language
//
// Parameter Name:  LANGUAGE
//
// Purpose:  To specify the language for text values in a property or
//    property parameter.
//
// Format Definition:  This property parameter is defined by the
//    following notation:
//
//     languageparam = "LANGUAGE" "=" language
//
//     language = Language-Tag
//                ; As defined in [RFC5646].
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct LanguageParam(pub Language);

impl ICalendarEntity for LanguageParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "LANGUAGEPARAM",
            map(
                pair(
                    tag("LANGUAGE"),
                    preceded(tag("="), cut(Language::parse_ical)),
                ),
                |(_key, value)| Self(value)
            ),
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("LANGUAGE={}", self.0.render_ical())
    }
}

impl_icalendar_entity_traits!(LanguageParam);

#[cfg(test)]
mod tests {
    use super::*;

    use nom::combinator::all_consuming;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=en-US TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("en-US"))),
            ),
        );

        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=en TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("en"))),
            ),
        );

        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=i-enochian TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("i-enochian"))),
            ),
        );

        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=zh-Hant TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("zh-Hant"))),
            ),
        );

        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=zh-cmn-Hans-CN TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("zh-cmn-Hans-CN"))),
            ),
        );

        assert_parser_output!(
            LanguageParam::parse_ical("LANGUAGE=de-CH-1901 TESTING".into()),
            (
                " TESTING",
                LanguageParam(Language(String::from("de-CH-1901"))),
            ),
        );

        assert!(LanguageParam::parse_ical(":".into()).is_err());

        // (two region tags)
        assert!(all_consuming(LanguageParam::parse_ical)("LANGUAGE=de-419-DE".into()).is_err());
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            LanguageParam(Language(String::from("zh-cmn-Hans-CN"))).render_ical(),
            String::from("LANGUAGE=zh-cmn-Hans-CN"),
        );
    }
}
