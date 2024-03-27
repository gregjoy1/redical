pub mod grammar;
pub mod content_line;
pub mod property_value_data_types;
pub mod property_parameters;
pub mod properties;

#[derive(Clone, Debug, PartialEq)]
pub struct ParserError<'a> {
    span: ParserInput<'a>,
    message: Option<String>,
    context: Vec<String>,
}

impl <'a> std::fmt::Display for ParserError<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = self.message.clone().unwrap_or(String::from("-"));
        let context = self.context.join(" <- ");

        write!(f, "Error: {message} in {context}")
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
    format!("Error - {:#?}", error)
}

pub type ParserInput<'a> = nom_locate::LocatedSpan<&'a str>;
pub type ParserResult<'a, O> = nom::IResult<ParserInput<'a>, O, ParserError<'a>>;

pub trait ICalendarEntity {
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized;

    fn render_ical(&self) -> String;

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

    fn render_ical(&self) -> String {
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
    // use super::*;

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

    /*
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
    */
}
