use std::ops::Deref;
use std::collections::HashSet;

use nom::multi::separated_list0;
use nom::combinator::map;

use crate::grammar::comma;

use crate::{ICalendarEntity, ParserInput, ParserResult};

/// Parses and serializes a list of values
///
/// # Examples
///
/// ```rust
///
/// use std::collections::HashSet;
///
/// use redical_ical::grammar::List;
/// use redical_ical::value_data_types::integer::Integer;
/// use redical_ical::ICalendarEntity;
///
/// // Testing zero values
/// let result = List::<Integer>::parse_ical(" TESTING".into());
///
/// let Ok((remaining, parsed_list)) = result else {
///     panic!("Expected to be Ok, Actual: {:#?}", result);
/// };
///
/// assert_eq!(remaining.to_string(), String::from(" TESTING"));
/// assert_eq!(parsed_list, List(HashSet::from([])));
///
/// // Testing single value
/// let result = List::<Integer>::parse_ical("10 TESTING".into());
///
/// let Ok((remaining, parsed_list)) = result else {
///     panic!("Expected to be Ok, Actual: {:#?}", result);
/// };
///
/// assert_eq!(remaining.to_string(), String::from(" TESTING"));
/// assert_eq!(parsed_list, List(HashSet::from([Integer(10)])));
///
/// // Testing multiple values
/// let result = List::<Integer>::parse_ical("10,20,30 TESTING".into());
///
/// let Ok((remaining, parsed_list)) = result else {
///     panic!("Expected to be Ok, Actual: {:#?}", result);
/// };
///
/// assert_eq!(remaining.to_string(), String::from(" TESTING"));
/// assert_eq!(parsed_list, List(HashSet::from([Integer(10), Integer(20), Integer(30)])));
/// ```
/// [plus / minus] 1*digit
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct List<T>(pub HashSet<T>)
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
;

impl<T> Deref for List<T>
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
{
    type Target = HashSet<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ICalendarEntity for List<T>
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
{
    fn parse_ical(input: ParserInput) -> ParserResult<Self>
    where
        Self: Sized
    {
        map(
            separated_list0(comma, T::parse_ical),
            |parsed_list| {
                List(HashSet::from_iter(parsed_list.into_iter()))
            },
        )(input)
    }

    fn render_ical(&self) -> String {
        let mut list_elements =
            self.0
                .iter()
                .map(|value| value.render_ical())
                .collect::<Vec<String>>();

        list_elements.sort();

        list_elements.join(",")
    }
}

impl<T> std::str::FromStr for List<T>
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
{
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

impl<T> ToString for List<T>
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
{
    fn to_string(&self) -> String {
        self.render_ical()
    }
}

impl<T> From<Vec<T>> for List<T>
where
    T: std::fmt::Debug + Clone + ICalendarEntity + Eq + PartialEq + std::hash::Hash,
{
    fn from(value: Vec<T>) -> Self {
        List(HashSet::from_iter(value.into_iter()))
    }
}
