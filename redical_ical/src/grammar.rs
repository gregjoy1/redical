use nom::sequence::{tuple, preceded, delimited, terminated};
use nom::error::context;
use nom::branch::alt;
use nom::multi::{many0, separated_list1};
use nom::combinator::{recognize, opt, verify, map};
use nom::bytes::complete::{take_while, take_while1, take_while_m_n};
use nom::character::{is_alphabetic, is_digit};
use nom::character::complete::{alphanumeric1, char};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, ParserError, impl_icalendar_entity_traits};

/// Recognizes a pattern
///
/// Wrapped nom::bytes::complete::tag to provide better error messages e.g. "expected '<tag text>'"
/// over the unhelpful nom base tag parser.
///
/// The input data will be compared to the tag combinator's argument and will return the part of
/// the input that matches the argument
///
/// It will return `Err(Err::Error((ParserError)))` if the input doesn't match the pattern
/// # Example
/// ```rust
/// # use nom::{Err, error::{Error, ErrorKind}, Needed, IResult};
/// use redical_ical::{ParserError, ParserInput, ParserResult};
/// use redical_ical::grammar::tag;
///
/// fn parser(input: ParserInput) -> ParserResult<ParserInput> {
///   tag("Hello")(input)
/// }
///
/// assert!(parser("Hello, World!".into()).is_ok());
///
/// let input: ParserInput = "Something".into();
///
/// assert_eq!(
///     parser(input),
///     Err(
///         nom::Err::Error(
///             ParserError::new(String::from("expected 'Hello'"), input)
///         )
///     ),
/// );
/// ```
pub fn tag<'a>(tag: &'a str) -> impl Fn(ParserInput) -> ParserResult<ParserInput> + 'a {
    move |input: ParserInput| {
        match nom::bytes::complete::tag::<&'a str, ParserInput, ParserError>(tag)(input) {
            Ok(result) => Ok(result),

            Err(nom::Err::Error(_error)) => {
                Err(
                    nom::Err::Error(
                        ParserError::new(format!("expected '{}'", tag), input)
                    )
                )
            },

            Err(nom::Err::Failure(_error)) => {
                Err(
                    nom::Err::Failure(
                        ParserError::new(format!("expected '{}'", tag), input)
                    )
                )
            },

            Err(nom::Err::Incomplete(error)) => {
                Err(
                    nom::Err::Incomplete(error)
                )
            },
        }
    }
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | HTAB                   | 9                 |
// +------------------------+-------------------+

/// Returns if horizontal tab char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_htab_char;
///
/// assert!(is_htab_char('\t'));
///
/// assert_eq!(is_htab_char('_'), false);
/// assert_eq!(is_htab_char(' '), false);
/// ```
pub fn is_htab_char(input: char) -> bool {
    input as u8 == b'\t'
}

/// Parses horizontal tab char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::htab;
///
/// assert!(htab("\ttest".into()).is_ok());
/// assert!(htab("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = htab("\t\t".into()).unwrap();
///
/// assert_eq!(*remaining, "\t");
/// assert_eq!(*parsed, "\t");
/// ```
pub fn htab(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_htab_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LF                     | 10                |
// +------------------------+-------------------+

/// Returns if line feed char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_lf_char;
///
/// assert!(is_lf_char(10_u8 as char));
/// assert!(is_lf_char('\n'));
///
/// assert_eq!(is_lf_char('_'), false);
/// assert_eq!(is_lf_char(' '), false);
/// ```
pub fn is_lf_char(input: char) -> bool {
    input as u8 == b'\n'
}

/// Parses line feed char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::lf;
///
/// assert!(lf("\ntest".into()).is_ok());
/// assert!(lf("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = lf("\n\n".into()).unwrap();
///
/// assert_eq!(*remaining, "\n");
/// assert_eq!(*parsed, "\n");
/// ```
pub fn lf(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_lf_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | CR                     | 13                |
// +------------------------+-------------------+

/// Returns if carriage return char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_cr_char;
///
/// assert!(is_cr_char(13_u8 as char));
/// assert!(is_cr_char('\r'));
///
/// assert_eq!(is_cr_char('_'), false);
/// assert_eq!(is_cr_char(' '), false);
/// ```
pub fn is_cr_char(input: char) -> bool {
    input as u8 == b'\r'
}

/// Parses carriage return char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::cr;
///
/// assert!(cr("\rtest".into()).is_ok());
/// assert!(cr("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = cr("\r\r".into()).unwrap();
///
/// assert_eq!(*remaining, "\r");
/// assert_eq!(*parsed, "\r");
/// ```
pub fn cr(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_cr_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | DQUOTE                 | 22                |
// +------------------------+-------------------+

/// Returns if double quote char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_dquote_char;
///
/// assert!(is_dquote_char('"'));
///
/// assert_eq!(is_dquote_char('_'), false);
/// assert_eq!(is_dquote_char(' '), false);
/// ```
pub fn is_dquote_char(input: char) -> bool {
    input == '"'
}

/// Parses double quote char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::dquote;
///
/// assert!(dquote("\"test".into()).is_ok());
/// assert!(dquote("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = dquote("\"\"".into()).unwrap();
///
/// assert_eq!(*remaining, "\"");
/// assert_eq!(*parsed, "\"");
/// ```
pub fn dquote(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_dquote_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | SPACE                  | 32                |
// +------------------------+-------------------+

/// Returns if space char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_space_char;
///
/// assert!(is_space_char(' '));
///
/// assert_eq!(is_space_char('_'), false);
/// assert_eq!(is_space_char('-'), false);
/// ```
pub fn is_space_char(input: char) -> bool {
    input == ' '
}

/// Parses space char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::space;
///
/// assert!(space(" test".into()).is_ok());
/// assert!(space("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = space("  ".into()).unwrap();
///
/// assert_eq!(*remaining, " ");
/// assert_eq!(*parsed, " ");
/// ```
pub fn space(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_space_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | PLUS SIGN              | 43                |
// +------------------------+-------------------+

/// Returns if plus sign char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_plus_sign_char;
///
/// assert!(is_plus_sign_char('+'));
///
/// assert_eq!(is_plus_sign_char('_'), false);
/// assert_eq!(is_plus_sign_char(' '), false);
/// ```
pub fn is_plus_sign_char(input: char) -> bool {
    input == '+'
}

/// Parses plus sign char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::plus_sign;
///
/// assert!(plus_sign("+test".into()).is_ok());
/// assert!(plus_sign("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = plus_sign("++".into()).unwrap();
///
/// assert_eq!(*remaining, "+");
/// assert_eq!(*parsed, "+");
/// ```
pub fn plus_sign(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_plus_sign_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | COMMA                  | 44                |
// +------------------------+-------------------+

/// Returns if comma char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_comma_char;
///
/// assert!(is_comma_char(','));
///
/// assert_eq!(is_comma_char('_'), false);
/// assert_eq!(is_comma_char(' '), false);
/// ```
pub fn is_comma_char(input: char) -> bool {
    input == ','
}

/// Parses comma char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::comma;
///
/// assert!(comma(",test".into()).is_ok());
/// assert!(comma("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = comma(",,".into()).unwrap();
///
/// assert_eq!(*remaining, ",");
/// assert_eq!(*parsed, ",");
/// ```
pub fn comma(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_comma_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | HYPHEN-MINUS           | 45                |
// +------------------------+-------------------+

/// Returns if hyphen-minus char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_hyphen_minus_char;
///
/// assert!(is_hyphen_minus_char('-'));
///
/// assert_eq!(is_hyphen_minus_char('_'), false);
/// assert_eq!(is_hyphen_minus_char(' '), false);
/// ```
pub fn is_hyphen_minus_char(input: char) -> bool {
    input == '-'
}

/// Parses hyphen-minus char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::hyphen_minus;
///
/// assert!(hyphen_minus("-test".into()).is_ok());
/// assert!(hyphen_minus("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = hyphen_minus("--".into()).unwrap();
///
/// assert_eq!(*remaining, "-");
/// assert_eq!(*parsed, "-");
/// ```
pub fn hyphen_minus(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_hyphen_minus_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | PERIOD                 | 46                |
// +------------------------+-------------------+

/// Returns if period char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_period_char;
///
/// assert!(is_period_char('.'));
///
/// assert_eq!(is_period_char('_'), false);
/// assert_eq!(is_period_char(' '), false);
/// ```
pub fn is_period_char(input: char) -> bool {
    input == '.'
}

/// Parses period char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::period;
///
/// assert!(period(".test".into()).is_ok());
/// assert!(period("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = period("..".into()).unwrap();
///
/// assert_eq!(*remaining, ".");
/// assert_eq!(*parsed, ".");
/// ```
pub fn period(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_period_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | SOLIDUS                | 47                |
// +------------------------+-------------------+

/// Returns if solidus char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_solidus_char;
///
/// assert!(is_solidus_char('/'));
///
/// assert_eq!(is_solidus_char('_'), false);
/// assert_eq!(is_solidus_char(' '), false);
/// ```
pub fn is_solidus_char(input: char) -> bool {
    input == '/'
}

/// Parses solidus char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::solidus;
///
/// assert!(solidus("/test".into()).is_ok());
/// assert!(solidus("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = solidus("//".into()).unwrap();
///
/// assert_eq!(*remaining, "/");
/// assert_eq!(*parsed, "/");
/// ```
pub fn solidus(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_solidus_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | COLON                  | 58                |
// +------------------------+-------------------+

/// Returns if colon char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_colon_char;
///
/// assert!(is_colon_char(':'));
///
/// assert_eq!(is_colon_char('_'), false);
/// assert_eq!(is_colon_char(' '), false);
/// ```
pub fn is_colon_char(input: char) -> bool {
    input == ':'
}

/// Parses colon char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::colon;
///
/// assert!(colon(":test".into()).is_ok());
/// assert!(colon("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = colon("::".into()).unwrap();
///
/// assert_eq!(*remaining, ":");
/// assert_eq!(*parsed, ":");
/// ```
pub fn colon(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_colon_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | SEMICOLON              | 59                |
// +------------------------+-------------------+

/// Returns if semicolon char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_semicolon_char;
///
/// assert!(is_semicolon_char(';'));
///
/// assert_eq!(is_semicolon_char('_'), false);
/// assert_eq!(is_semicolon_char(' '), false);
/// ```
pub fn is_semicolon_char(input: char) -> bool {
    input == ';'
}

/// Parses semicolon char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::semicolon;
///
/// assert!(semicolon(";test".into()).is_ok());
/// assert!(semicolon("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = semicolon(";;".into()).unwrap();
///
/// assert_eq!(*remaining, ";");
/// assert_eq!(*parsed, ";");
/// ```
pub fn semicolon(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_semicolon_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LATIN CAPITAL LETTER N | 78                |
// +------------------------+-------------------+

/// Returns if horizontal tab char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_latin_capital_letter_n_char;
///
/// assert!(is_latin_capital_letter_n_char('N'));
///
/// assert_eq!(is_latin_capital_letter_n_char('_'), false);
/// assert_eq!(is_latin_capital_letter_n_char(' '), false);
/// ```
pub fn is_latin_capital_letter_n_char(input: char) -> bool {
    input == 'N'
}

/// Parses latin capital letter N char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::latin_capital_letter_n;
///
/// assert!(latin_capital_letter_n("Ntest".into()).is_ok());
/// assert!(latin_capital_letter_n("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = latin_capital_letter_n("NN".into()).unwrap();
///
/// assert_eq!(*remaining, "N");
/// assert_eq!(*parsed, "N");
/// ```
pub fn latin_capital_letter_n(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_latin_capital_letter_n_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LATIN CAPITAL LETTER T | 84                |
// +------------------------+-------------------+

/// Returns if latin capital letter T char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_latin_capital_letter_t_char;
///
/// assert!(is_latin_capital_letter_t_char('T'));
///
/// assert_eq!(is_latin_capital_letter_t_char('_'), false);
/// assert_eq!(is_latin_capital_letter_t_char(' '), false);
/// ```
pub fn is_latin_capital_letter_t_char(input: char) -> bool {
    input == 'T'
}

/// Parses latin capital letter T char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::latin_capital_letter_t;
///
/// assert!(latin_capital_letter_t("Ttest".into()).is_ok());
/// assert!(latin_capital_letter_t("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = latin_capital_letter_t("TT".into()).unwrap();
///
/// assert_eq!(*remaining, "T");
/// assert_eq!(*parsed, "T");
/// ```
pub fn latin_capital_letter_t(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_latin_capital_letter_t_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LATIN CAPITAL LETTER X | 88                |
// +------------------------+-------------------+

/// Returns if latin capital letter X char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_latin_capital_letter_x_char;
///
/// assert!(is_latin_capital_letter_x_char('X'));
///
/// assert_eq!(is_latin_capital_letter_x_char('_'), false);
/// assert_eq!(is_latin_capital_letter_x_char(' '), false);
/// ```
pub fn is_latin_capital_letter_x_char(input: char) -> bool {
    input == 'X'
}

/// Parses latin capital letter X char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::latin_capital_letter_x;
///
/// assert!(latin_capital_letter_x("Xtest".into()).is_ok());
/// assert!(latin_capital_letter_x("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = latin_capital_letter_x("XX".into()).unwrap();
///
/// assert_eq!(*remaining, "X");
/// assert_eq!(*parsed, "X");
/// ```
pub fn latin_capital_letter_x(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_latin_capital_letter_x_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LATIN CAPITAL LETTER Z | 90                |
// +------------------------+-------------------+

/// Returns if letter capital letter Z char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_latin_capital_letter_z_char;
///
/// assert!(is_latin_capital_letter_z_char('Z'));
///
/// assert_eq!(is_latin_capital_letter_z_char('_'), false);
/// assert_eq!(is_latin_capital_letter_z_char(' '), false);
/// ```
pub fn is_latin_capital_letter_z_char(input: char) -> bool {
    input == 'Z'
}

/// Parses letter capital letter Z char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::latin_capital_letter_z;
///
/// assert!(latin_capital_letter_z("Ztest".into()).is_ok());
/// assert!(latin_capital_letter_z("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = latin_capital_letter_z("ZZ".into()).unwrap();
///
/// assert_eq!(*remaining, "Z");
/// assert_eq!(*parsed, "Z");
/// ```
pub fn latin_capital_letter_z(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_latin_capital_letter_z_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | BACKSLASH              | 92                |
// +------------------------+-------------------+

/// Returns if backslash char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_backslash_char;
///
/// assert!(is_backslash_char('\\'));
///
/// assert_eq!(is_backslash_char('_'), false);
/// assert_eq!(is_backslash_char(' '), false);
/// ```
pub fn is_backslash_char(input: char) -> bool {
    input == '\\'
}

/// Parses backslash char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::backslash;
///
/// assert!(backslash("\\test".into()).is_ok());
/// assert!(backslash("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = backslash("\\\\".into()).unwrap();
///
/// assert_eq!(*remaining, "\\");
/// assert_eq!(*parsed, "\\");
/// ```
pub fn backslash(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_backslash_char)(input)
}

// +------------------------+-------------------+
// | Character name         | Decimal codepoint |
// +------------------------+-------------------+
// | LATIN SMALL LETTER N   | 110               |
// +------------------------+-------------------+

/// Returns if latin small letter n char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_latin_small_letter_n_char;
///
/// assert!(is_latin_small_letter_n_char('n'));
///
/// assert_eq!(is_latin_small_letter_n_char('_'), false);
/// assert_eq!(is_latin_small_letter_n_char(' '), false);
/// ```
pub fn is_latin_small_letter_n_char(input: char) -> bool {
    input == 'n'
}

/// Parses latin small letter n char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::latin_small_letter_n;
///
/// assert!(latin_small_letter_n("ntest".into()).is_ok());
/// assert!(latin_small_letter_n("test".into()).is_err());
///
/// // It only takes one char at a time.
/// let (remaining, parsed) = latin_small_letter_n("nn".into()).unwrap();
///
/// assert_eq!(*remaining, "n");
/// assert_eq!(*parsed, "n");
/// ```
pub fn latin_small_letter_n(input: ParserInput) -> ParserResult<ParserInput> {
    take_while_m_n(1, 1, is_latin_small_letter_n_char)(input)
}

/// Parse a CRLF line-break char sequence (CR followed by LF).
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::crlf;
///
/// assert!(crlf("\n\rTest".into()).is_ok());
///
/// assert!(crlf("\rTest".into()).is_err());
/// assert!(crlf("\r\nTest".into()).is_err());
/// assert!(crlf("\rTest".into()).is_err());
/// assert!(crlf("\nTest".into()).is_err());
/// assert!(crlf("Test".into()).is_err());
/// assert!(crlf(" ".into()).is_err());
/// ```
pub fn crlf(input: ParserInput) -> ParserResult<ParserInput> {
    tag("\n\r")(input)
}

/// ; This ABNF is just a general definition for an initial parsing
/// ; of the content line into its property name, parameter list,
/// ; and value string
/// 
/// contentline   = name *(";" param ) ":" value CRLF
pub fn contentline(input: ParserInput) -> ParserResult<(ParserInput, Vec<(ParserInput, ParserInput)>, ParserInput)> {
    context(
        "CONTENTLINE",
        tuple(
            (
                name,
                many0(preceded(semicolon, param)),
                terminated(
                    preceded(colon, value),
                    opt(crlf),
                )
            )
        )
    )(input)
}

/// name          = iana-token / x-name
pub fn name(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "NAME",
        alt(
            (
                iana_token,
                x_name,
            )
        )
    )(input)
}

/// Returns if IANA token char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_iana_token_char;
///
/// assert!(is_iana_token_char('-'));
/// assert!(is_iana_token_char('2'));
/// assert!(is_iana_token_char('b'));
/// assert!(is_iana_token_char('Z'));
///
/// assert_eq!(is_iana_token_char('!'), false);
/// assert_eq!(is_iana_token_char('_'), false);
/// ```
///
/// iana-token    = 1*(ALPHA / DIGIT / "-")
/// ; iCalendar identifier registered with IANA
pub fn is_iana_token_char(input: char) -> bool {
    is_alphabetic(input as u8) || is_digit(input as u8) || is_hyphen_minus_char(input)
}

/// Parses IANA token chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::iana_token;
///
/// assert!(iana_token("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".into()).is_ok());
/// assert!(iana_token("!test_".into()).is_err());
/// ```
///
/// iana-token    = 1*(ALPHA / DIGIT / "-")
/// ; iCalendar identifier registered with IANA
pub fn iana_token(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "IANA-TOKEN",
        take_while1(is_iana_token_char),
    )(input)
}

/// Parses X-NAME token chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::x_name;
///
/// assert!(x_name("X-abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".into()).is_ok());
/// assert!(x_name("X-TEST".into()).is_ok());
///
/// assert!(x_name("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890".into()).is_err());
/// assert!(x_name("X-".into()).is_err());
/// assert!(x_name("!test_".into()).is_err());
/// ```
///
/// x-name        = "X-" [vendorid "-"] 1*(ALPHA / DIGIT / "-")
/// ; Reserved for experimental use.
pub fn x_name(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "X-NAME",
        recognize(
            tuple(
                (
                    tag("X-"),
                    opt(terminated(vendorid, char('-'))),
                    iana_token,
                )
            )
        )
    )(input)
}

/// Parse vendorid chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::vendorid;
///
/// assert!(vendorid("2Zb".into()).is_ok());
/// assert!(vendorid("2Zb0".into()).is_ok());
/// assert!(vendorid("2Zb0P".into()).is_ok());
///
/// assert!(vendorid("2".into()).is_err());
/// assert!(vendorid("2Z".into()).is_err());
/// assert!(vendorid("-".into()).is_err());
/// assert!(vendorid("!".into()).is_err());
/// assert!(vendorid("_".into()).is_err());
/// ```
///
/// vendorid      = 3*(ALPHA / DIGIT)
/// ; Vendor identification
pub fn vendorid(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "VENDORID",
        verify(alphanumeric1, |parsed_input: &ParserInput| parsed_input.len() >= 3)
    )(input)
}

/// param         = param-name "=" param-value *("," param-value)
/// ; Each property defines the specific ABNF for the parameters
/// ; allowed on the property.  Refer to specific properties for
/// ; precise parameter ABNF.
pub fn param(input: ParserInput) -> ParserResult<(ParserInput, ParserInput)> {
    context(
        "PARAM",
        tuple(
            (
                terminated(param_name, char('=')),
                recognize(separated_list1(comma, param_value)),
            )
        )
    )(input)
}

/// param-name    = iana-token / x-name
pub fn param_name(input: ParserInput) -> ParserResult<ParserInput> {
    context("PARAM-NAME", alt((iana_token, x_name)))(input)
}

/// param-value   = paramtext / quoted-string
pub fn param_value(input: ParserInput) -> ParserResult<ParserInput> {
    context("PARAM-VALUE", alt((quoted_string, paramtext)))(input)
}

/// paramtext     = *SAFE-CHAR
pub fn paramtext(input: ParserInput) -> ParserResult<ParserInput> {
    context("PARAMTEXT", take_while(is_safe_char))(input)
}

/// value         = *VALUE-CHAR
pub fn value(input: ParserInput) -> ParserResult<ParserInput> {
    context("VALUE", take_while(is_value_char))(input)
}

/// quoted-string = DQUOTE *QSAFE-CHAR DQUOTE
pub fn quoted_string(input: ParserInput) -> ParserResult<ParserInput> {
    context(
        "QUOTED-STRING",
        recognize(
            delimited(
                dquote,
                take_while(is_qsafe_char),
                dquote,
            )
        )
    )(input)
}

/// Returns if quote safe char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_qsafe_char;
///
/// assert!(is_qsafe_char(' '));
/// assert!(is_qsafe_char('A'));
/// assert!(is_qsafe_char('\x21'));
/// assert!(is_qsafe_char('\x35'));
/// assert!(is_qsafe_char('\x2E'));
/// assert!(is_qsafe_char('\x3D'));
/// assert!(is_qsafe_char('の'));
///
/// assert_eq!(is_qsafe_char('"'), false);
/// assert_eq!(is_qsafe_char('\x02'), false);
/// ```
///
/// QSAFE-CHAR    = WSP / %x21 / %x23-7E / NON-US-ASCII
/// ; Any character except CONTROL and DQUOTE
pub fn is_qsafe_char(input: char) -> bool {
    is_wsp_char(input)     ||
    is_non_us_ascii_char(input) ||
    input == '\x21'        ||
    (input >= '\x23' && input <= '\x7E')
}

/// Parse quote safe char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::qsafe_char;
///
/// assert!(qsafe_char("A \x21\x35\x2E\x3Dの\r".into()).is_ok());
/// assert!(qsafe_char("\"test".into()).is_err());
/// assert!(qsafe_char("\x02test".into()).is_err())
/// ```
///
/// QSAFE-CHAR    = WSP / %x21 / %x23-7E / NON-US-ASCII
/// ; Any character except CONTROL and DQUOTE
pub fn qsafe_char(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_qsafe_char)(input)
}

/// Returns if safe char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_safe_char;
///
/// assert!(is_safe_char(' '));
/// assert!(is_safe_char('A'));
/// assert!(is_safe_char('\x21'));
/// assert!(is_safe_char('\x35'));
/// assert!(is_safe_char('\x2E'));
/// assert!(is_safe_char('\x3D'));
/// assert!(is_safe_char('の'));
///
/// assert_eq!(is_safe_char(';'), false);
/// assert_eq!(is_safe_char(':'), false);
/// assert_eq!(is_safe_char(','), false);
/// ```
///
/// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E
///               / NON-US-ASCII
/// ; Any character except CONTROL, DQUOTE, ";", ":", ","
pub fn is_safe_char(input: char) -> bool {
    is_wsp_char(input)     ||
    is_non_us_ascii_char(input) ||
    input == '\x21'        ||
    (input >= '\x23' && input <= '\x2B') ||
    (input >= '\x2D' && input <= '\x39') ||
    (input >= '\x3C' && input <= '\x7E')
}

/// Parse safe char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::safe_char;
///
/// assert!(safe_char("A \x21\x35\x2E\x3Dの\r".into()).is_ok());
/// assert!(safe_char(":test".into()).is_err());
/// ```
///
/// SAFE-CHAR     = WSP / %x21 / %x23-2B / %x2D-39 / %x3C-7E
///               / NON-US-ASCII
/// ; Any character except CONTROL, DQUOTE, ";", ":", ","
pub fn safe_char(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_safe_char)(input)
}

/// Returns if value char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_value_char;
///
/// assert!(is_value_char(' '));
/// assert!(is_value_char('\x23'));
/// assert!(is_value_char('の'));
///
/// assert_eq!(is_value_char('\x08'), false);
/// ```
///
/// VALUE-CHAR    = WSP / %x21-7E / NON-US-ASCII
/// ; Any textual character
pub fn is_value_char(input: char) -> bool {
    is_wsp_char(input)     ||
    is_non_us_ascii_char(input) ||
    (input >= '\x21' && input <= '\x7E')
}

/// Parse value char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::value;
///
/// assert!(value(" の\x23".into()).is_ok());
/// ```
///
/// VALUE-CHAR    = WSP / %x21-7E / NON-US-ASCII
/// ; Any textual character
pub fn value_char(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_value_char)(input)
}

/// Parses one or more whitespace chars.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::wsp;
///
/// assert!(wsp("\t\r\n\x0C ".into()).is_ok());
/// assert!(wsp("A".into()).is_err());
/// ```
///
/// WSP  = HTAB | CR | LF | SPACE | FF
pub fn wsp(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_wsp_char)(input)
}

/// Parses single whitespace char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::wsp_1_1;
///
/// assert!(wsp_1_1("\tTEST".into()).is_ok());
/// assert!(wsp_1_1("\rTEST".into()).is_ok());
/// assert!(wsp_1_1("\nTEST".into()).is_ok());
/// assert!(wsp_1_1("\x0CTEST".into()).is_ok());
/// assert!(wsp_1_1(" TEST".into()).is_ok());
///
/// assert!(wsp_1_1("A".into()).is_err());
/// ```
///
/// WSP  = HTAB | CR | LF | SPACE | FF
pub fn wsp_1_1(input: ParserInput) -> ParserResult<ParserInput> {
    alt((
        tag("\t"),
        tag("\r"),
        tag("\n"),
        tag("\x0C"),
        tag(" ")
    ))(input)
}

/// Returns if whitespace char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_wsp_char;
///
/// assert!(is_wsp_char('\t'));
/// assert!(is_wsp_char('\r'));
/// assert!(is_wsp_char('\n'));
/// assert!(is_wsp_char('\x0C'));
/// assert!(is_wsp_char(' '));
///
/// assert_eq!(is_wsp_char('A'), false);
/// assert_eq!(is_wsp_char('_'), false);
/// ```
///
/// WSP  = HTAB | CR | LF | SPACE | FF
pub fn is_wsp_char(input: char) -> bool {
    is_htab_char(input)  ||
    is_cr_char(input)    ||
    is_lf_char(input)    ||
    is_space_char(input) ||
    input == '\x0C' // FF
}
/// Returns if non US ASCII char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_non_us_ascii_char;
///
/// assert!(is_non_us_ascii_char('の'));
/// assert!(is_non_us_ascii_char('조'));
///
/// assert_eq!(is_non_us_ascii_char('A'), false);
/// assert_eq!(is_non_us_ascii_char('_'), false);
/// ```
///
/// NON-US-ASCII  = UTF8-2 / UTF8-3 / UTF8-4
/// ; UTF8-2, UTF8-3, and UTF8-4 are defined in [RFC3629]
pub fn is_non_us_ascii_char(input: char) -> bool {
    input > '\x7F'
}

/// Parse non US ASCII char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::non_us_ascii;
///
/// assert!(non_us_ascii("조test".into()).is_ok());
/// assert!(non_us_ascii("!test".into()).is_err());
/// ```
///
/// NON-US-ASCII  = UTF8-2 / UTF8-3 / UTF8-4
/// ; UTF8-2, UTF8-3, and UTF8-4 are defined in [RFC3629]
pub fn non_us_ascii(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_non_us_ascii_char)(input)
}

/// Returns if control char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::is_control_char;
///
/// assert!(is_control_char('\x02'));
/// assert!(is_control_char('\x0B'));
/// assert!(is_control_char('\x7F'));
///
/// assert_eq!(is_control_char('!'), false);
/// assert_eq!(is_control_char('_'), false);
/// ```
///
/// CONTROL       = %x00-08 / %x0A-1F / %x7F
/// ; All the controls except HTAB
pub fn is_control_char(input: char) -> bool {
    (input >= '\x00' && input <= '\x09') ||
    (input >= '\x0A' && input <= '\x1F') ||
    input >= '\x7F'
}

/// Parse control char.
///
/// # Examples
///
/// ```rust
/// use redical_ical::grammar::control;
///
/// assert!(control("\rtest".into()).is_ok());
/// assert!(control("!test".into()).is_err());
/// ```
///
/// CONTROL       = %x00-08 / %x0A-1F / %x7F
/// ; All the controls except HTAB
pub fn control(input: ParserInput) -> ParserResult<ParserInput> {
    take_while1(is_control_char)(input)
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PositiveNegative {
    Positive,
    Negative,
}

impl ICalendarEntity for PositiveNegative {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        alt((
            map(plus_sign, |_| Self::Positive),
            map(hyphen_minus, |_| Self::Negative),
        ))(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        match self {
            Self::Positive => String::from("+"),
            Self::Negative => String::from("-"),
        }
    }
}

impl PositiveNegative {
    /// Parses a potentially positive or negative digits
    ///
    /// # Examples
    ///
    /// ```rust
    ///
    /// use redical_ical::grammar::PositiveNegative;
    ///
    /// // Create a parser that:
    /// //   1. Expects a digit 2-3 characters in length
    /// //   2. Expects it to be at greater than or equal to 15
    /// //   3. Expects it to be less than or equal to 500
    /// let mut parser = PositiveNegative::parse_i32_m_n(2, 3, 15, 500);
    ///
    /// // Testing "+22"
    /// let result = parser("+22 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, 22_i32);
    ///
    /// // Testing "-250"
    /// let result = parser("-250 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, -250_i32);
    ///
    /// // Testing "500"
    /// let result = parser("500 TESTING".into());
    ///
    /// let Ok((remaining, parsed_value)) = result else {
    ///     panic!("Expected to be Ok, Actual: {:#?}", result);
    /// };
    ///
    /// assert_eq!(remaining.to_string(), String::from(" TESTING"));
    /// assert_eq!(parsed_value, 500_i32);
    ///
    /// assert!(parser("1".into()).is_err());
    /// assert!(parser("501".into()).is_err());
    /// assert!(parser("10".into()).is_err());
    /// ```
    /// [plus / minus] 1*digit
    pub fn parse_i32_m_n<'a>(min_chars: usize, max_chars: usize, min_value: i32, max_value: i32) -> impl FnMut(ParserInput) -> ParserResult<i32> {
        move |input: ParserInput| {
            let (remaining, parsed_positive_negative) = opt(Self::parse_ical)(input)?;
            let (remaining, parsed_value) = take_while_m_n(min_chars, max_chars, |value| is_digit(value as u8))(remaining)?;

            let Ok(mut value) = parsed_value.to_string().parse::<i32>() else {
                return Err(
                    nom::Err::Error(
                        ParserError::new(String::from("Invalid number"), input)
                    )
                );
            };

            if value < min_value || value > max_value {
                return Err(
                    nom::Err::Error(
                        ParserError::new(format!("Expected number between {min_value}-{max_value}"), input)
                    )
                );
            }

            if let Some(Self::Negative) = parsed_positive_negative {
                value = -value;
            }

            Ok((remaining, value))
        }
    }
}

impl_icalendar_entity_traits!(PositiveNegative);
