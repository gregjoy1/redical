use std::collections::BTreeSet;

pub mod grammar;
pub mod content_line;
pub mod values;
pub mod properties;

use content_line::ContentLine;

#[derive(Clone, Debug, PartialEq)]
pub struct ParserError<'a> {
    span: ParserInput<'a>,
    message: Option<String>,
    context: Vec<String>,
}

impl <'a> std::fmt::Display for ParserError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", convert_error(self.span.into_fragment(), self.to_owned()))
    }
}

impl<'a> ParserError<'a> {
    pub fn new(message: String, span: ParserInput<'a>) -> Self {
        Self {
            span,
            message: Some(message),
            context: Vec::new(),
        }
    }

    pub fn span(&self) -> &ParserInput {
        &self.span
    }

    pub fn line(&self) -> u32 {
        self.span().location_line()
    }

    pub fn offset(&self) -> usize {
        self.span().location_offset()
    }
}

impl<'a> nom::error::ParseError<ParserInput<'a>> for ParserError<'a> {
    fn from_error_kind(input: ParserInput<'a>, kind: nom::error::ErrorKind) -> Self {
        Self::new(
            format!("parse error {:?}", kind),
            input,
        )
    }

    fn append(_input: ParserInput<'a>, _kind: nom::error::ErrorKind, other: Self) -> Self {
        other
    }

    fn from_char(input: ParserInput<'a>, unexpected: char) -> Self {
        Self::new(
            format!("unexpected character '{}'", unexpected),
            input,
        )
    }
}

impl<'a> nom::error::ContextError<ParserInput<'a>> for ParserError<'a> {
    fn add_context(_input: ParserInput, context: &'static str, mut other: Self) -> Self {
        other.context.push(String::from(context));
        other
    }
}

impl<'a, E> nom::error::FromExternalError<ParserInput<'a>, E> for ParserError<'a>
where
    E: ToString,
{
  /// Create a new error from an input position and an external error
  fn from_external_error(input: ParserInput<'a>, _kind: nom::error::ErrorKind, error: E) -> Self {
    Self::new(error.to_string(), input)
  }
}


/// Transforms a `VerboseError` into a trace with input position information
/// Copy, pasted, overridden from nom::error::convert_error to return single
/// line errors which are more redis friendly.
pub fn convert_error<I: core::ops::Deref<Target = str>>(_input: I, error: ParserError) -> std::string::String {
    // TODO: Implement this...
    if error.context.is_empty() {
        format!("Error - {} at {}", error.message.unwrap_or(String::from("no error")), error.span.to_string().trim())
    } else {
        format!("Error - context {} - {} at {}", error.context.join(" -> "), error.message.unwrap_or(String::from("no error")), error.span.to_string().trim())
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DistanceUnit {
    Kilometers,
    Miles,
}

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct RenderingContext {
    pub tz: Option<chrono_tz::Tz>,
    pub distance_unit: Option<DistanceUnit>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParserContext {
    None,
    Event,
    Query,
}

impl Copy for ParserContext {}

impl Default for ParserContext {
    fn default() -> Self {
        ParserContext::None
    }
}

impl ParserContext {
    fn terminating_property_lookahead(&self) -> impl FnMut(ParserInput) -> ParserResult<ParserInput> + '_ {
        move |mut input: ParserInput| {
            input.extra = ParserContext::None;

            use nom::error::context;
            use nom::combinator::{recognize, eof, opt, not};
            use nom::sequence::{tuple, preceded};
            use nom::multi::many1;
            use nom::branch::alt;
            use grammar::{wsp, tag, contentline};
            use values::where_operator::WhereOperator;
            use properties::query::{GroupedWhereProperty, QueryProperty};

            match self {
                ParserContext::Event => {
                    context(
                        "EVENT PARSER CONTEXT",
                        recognize(
                            preceded(
                                grammar::wsp,
                                properties::event::EventProperty::parse_ical,
                            )
                        ),
                    )(input)
                },

                ParserContext::Query => {
                    context(
                        "QUERY PARSER CONTEXT",
                        recognize(
                            preceded(
                                opt(wsp),
                                alt((
                                    // TODO: HACK HACK HACK HACK - tidy and consolidate
                                    recognize(tuple((WhereOperator::parse_ical, opt(wsp), tag("(")))),
                                    recognize(tuple((opt(wsp), tag("("), opt(wsp), GroupedWhereProperty::parse_ical))),
                                    recognize(tuple((not(contentline), many1(tag(")")), alt((wsp, eof))))),
                                    recognize(GroupedWhereProperty::parse_ical),
                                    recognize(QueryProperty::parse_ical),
                                )),
                            )
                        ),
                    )(input)
                },

                _ => {
                    context(
                        "UNDEFINED PARSER CONTEXT",
                        recognize(
                            preceded(
                                wsp,
                                contentline,
                            )
                        ),
                    )(input)
                },
            }
        }
    }
}

pub type ParserInput<'a> = nom_locate::LocatedSpan<&'a str, ParserContext>;
pub type ParserResult<'a, O> = nom::IResult<ParserInput<'a>, O, ParserError<'a>>;

// TODO: document this
pub trait UnicodeSegmentation {
    fn wrapped_grapheme_indices<'a>(
        &'a self,
        is_extended: bool,
    ) -> unicode_segmentation::GraphemeIndices<'a>;
}

impl<'a> UnicodeSegmentation for ParserInput<'a> {
    #[inline]
    fn wrapped_grapheme_indices(&self, is_extended: bool) -> unicode_segmentation::GraphemeIndices {
        unicode_segmentation::UnicodeSegmentation::grapheme_indices(self.into_fragment(), is_extended)
    }
}

impl UnicodeSegmentation for &str {
    #[inline]
    fn wrapped_grapheme_indices(&self, is_extended: bool) -> unicode_segmentation::GraphemeIndices {
        unicode_segmentation::UnicodeSegmentation::grapheme_indices(*self, is_extended)
    }
}

/// A parser that greedily matches from the primary parser, then finds the earliest match from the
/// lookahead parser and returns the shortest result.
///
/// # Arguments
/// * `first` The first parser to apply.
/// * `second` The lookahead parser to terminate at.
///
/// ```rust
/// use nom::{Err, error::ErrorKind, Needed};
/// use nom::bytes::complete::tag;
/// use nom::character::complete::alpha1;
/// use redical_ical::terminated_lookahead;
///
/// let mut parser = terminated_lookahead(alpha1, tag("END"));
///
/// assert_eq!(parser("abcdefgEND"), Ok(("END", "abcdefg")));
/// assert_eq!(parser("abcdefg END"), Ok((" END", "abcdefg")));
/// assert_eq!(parser(""), Err(Err::Error(("", ErrorKind::Alpha))));
/// assert_eq!(parser("123"), Err(Err::Error(("123", ErrorKind::Alpha))));
/// ```
pub fn terminated_lookahead<I, O, O2, E, F, F2>(
    mut parser: F,
    mut look_ahead_parser: F2,
) -> impl FnMut(I) -> nom::IResult<I, O, E>
where
    O: std::fmt::Debug,
    E: std::fmt::Debug,
    I: Clone
        + UnicodeSegmentation
        + nom::InputLength
        + nom::Slice<std::ops::Range<usize>>
        + nom::Slice<std::ops::RangeFrom<usize>>
        + std::fmt::Debug
        + Copy,
    F: nom::Parser<I, O, E>,
    F2: nom::Parser<I, O2, E>,
{
    move |input: I| {
        let (remaining, output) = parser.parse(input.clone())?;

        let parser_max_index = input.input_len() - remaining.input_len();
        let input_max_index = input.input_len();

        let max_index = std::cmp::max(input_max_index, parser_max_index);

        let mut look_ahead_max_index = max_index;

        for (index, _element) in input.wrapped_grapheme_indices(true) {
            if index >= parser_max_index {
                break;
            }

            let sliced_input = input.slice(index..max_index);

            if look_ahead_parser.parse(sliced_input).is_ok() {
                look_ahead_max_index = index;

                break;
            }
        }

        // Return early if the parser terminates before the lookahead parser does (or at the same point).
        if look_ahead_max_index >= max_index || look_ahead_max_index >= (input.input_len() - 1) {
            return Ok((remaining, output));
        }

        // If the lookahead parser finds a match before the parser terminates, then we terminate
        // the input to the point the lookahead parser matches, provide that to the parser so it
        // does not overrun, then return the result and the remaining input sliced from the point
        // of the lookahead parser match.
        let look_ahead_restricted_input = input.slice(0..look_ahead_max_index);

        let (_, refined_output) = parser.parse(look_ahead_restricted_input)?;
        let refined_remaining = input.slice(look_ahead_max_index..);

        Ok((refined_remaining, refined_output))
    }
}

pub trait ICalendarComponent {
    fn to_rendered_content_lines_with_context(&self, context: Option<&RenderingContext>) -> Vec<String> {
        Vec::from_iter(
            self.to_content_line_set_with_context(context)
                .into_iter()
                .map(|content_line| content_line.render_ical())
        )
    }

    fn to_rendered_content_lines(&self) -> Vec<String> {
        self.to_rendered_content_lines_with_context(None)
    }

    fn to_content_line_set(&self) -> BTreeSet<ContentLine> {
        self.to_content_line_set_with_context(None)
    }

    fn to_content_line_set_with_context(&self, context: Option<&RenderingContext>) -> BTreeSet<ContentLine>;
}

pub trait ICalendarEntity {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized;

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String;

    fn render_ical(&self) -> String {
        self.render_ical_with_context(None)
    }

    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

impl<T> ICalendarEntity for Option<T>
where
    T: ICalendarEntity,
{
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        T::parse_ical(input).and_then(|(remaining, parsed)| Ok((remaining, Some(parsed))))
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        if let Some(entity) = self {
            entity.render_ical()
        } else {
            String::new()
        }
    }
}

#[macro_export]
macro_rules! impl_icalendar_entity_traits {
    ($entity:ident) => {
        impl std::str::FromStr for $entity {
            type Err = String;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                let parser_result = nom::combinator::all_consuming(Self::parse_ical)(input.into());

                match parser_result {
                    Ok((_remaining, value)) => Ok(value),

                    Err(error) => {
                        if let nom::Err::Error(error) = error {
                            Err(crate::convert_error(input, error))
                        } else {
                            Err(error.to_string())
                        }
                    }
                }
            }
        }

        impl ToString for $entity {
            fn to_string(&self) -> String {
                self.render_ical()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[macro_export]
    macro_rules! assert_parser_output {
        ($subject:expr, ($remaining:expr, $expected:expr $(,)*) $(,)*) => {
            let result = $subject;

            let Ok((remaining, parsed_value)) = result else {
                panic!("Expected to be Ok, Actual: {:#?}", result);
            };

            pretty_assertions_sorted::assert_eq!(remaining.to_string(), String::from($remaining));
            pretty_assertions_sorted::assert_eq_sorted!(parsed_value, $expected);
        }
    }

    pub use assert_parser_output;

    #[macro_export]
    macro_rules! assert_parser_error {
        ($subject:expr, nom::Err::Error(span: $span:expr, message: $message:expr, context: [$($context:expr $(,)*)+ $(,)*] $(,)*) $(,)*) => {
            let result = $subject;

            let Err(nom::Err::Error(parser_error)) = result else {
                panic!("Expected to be nom::Err::Error Error, Actual: {:#?}", result);
            };

            pretty_assertions_sorted::assert_eq!(parser_error.span.to_string(), String::from($span));
            pretty_assertions_sorted::assert_eq!(parser_error.message, Some(String::from($message)));

            pretty_assertions_sorted::assert_eq!(
                parser_error.context,
                vec![
                    $(
                        String::from($context)
                    )+
                ],
            );
        }
    }

    pub use assert_parser_error;
}
