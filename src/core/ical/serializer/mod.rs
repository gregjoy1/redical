use std::collections::BTreeSet;

use chrono::TimeZone;
use chrono_tz::Tz;

use crate::core::ical::properties::Property;

use crate::core::ical::parser::common::ParserResult;
use crate::core::utils::KeyValuePair;

mod serialized_value;

pub use serialized_value::SerializedValue;

/// Generates an iCalendar date-time string format with the prefix symbols.
/// Like: `:19970714T173000Z` or `19970714T133000`
/// ref: <https://tools.ietf.org/html/rfc5545#section-3.3.5>
pub fn serialize_timestamp_to_ical_datetime(utc_timestamp: &i64, timezone: &Tz) -> String {
    let mut timezone_postfix = String::new();

    let local_datetime = timezone.timestamp_opt(utc_timestamp.clone(), 0).unwrap();

    if timezone == &Tz::UTC {
        timezone_postfix = "Z".to_string();
    }

    let serialized_datetime = local_datetime.format("%Y%m%dT%H%M%S");

    format!("{}{}", serialized_datetime, timezone_postfix)
}

pub fn quote_string_if_needed<'a, F>(value: &'a String, mut no_quote_parser: F) -> String
where
    F: nom::Parser<&'a str, &'a str, nom::error::VerboseError<&'a str>>,
{
    // Wrap the FnMut parser inside a Fn closure that implements copy.
    let mut no_quote_parser = |value| no_quote_parser.parse(value);

    if let Ok((remaining, _parsed_value)) = no_quote_parser(value.as_str()) {
        if remaining.is_empty() {
            return value.clone();
        }
    }

    format!(r#""{value}""#)
}

pub trait SerializableICalComponent {
    // TODO: Wire up timezone...
    fn serialize_to_ical(&self, timezone: &Tz) -> Vec<String> {
        self.serialize_to_ical_set(timezone).into_iter().collect()
    }

    // TODO: Wire up timezone...
    fn serialize_to_ical_set(&self, timezone: &Tz) -> BTreeSet<String>;
}

pub trait SerializableICalProperty {
    fn to_key_value_pair(&self) -> KeyValuePair {
        let (name, params, value) = self.serialize_to_split_ical();

        let mut serialized_property = String::new();

        if let Some(params) = params {
            if params.len() > 0 {
                serialized_property.push(';');

                let key_value_pairs: Vec<String> = params
                    .iter()
                    .map(|(key, value)| format!("{}={}", key, value.to_string()))
                    .collect();

                serialized_property.push_str(key_value_pairs.join(";").as_str());
            }
        }

        serialized_property.push(':');
        serialized_property.push_str(value.to_string().as_str());

        KeyValuePair::new(name, serialized_property)
    }

    fn serialize_to_ical(&self) -> String {
        self.to_key_value_pair().to_string()
    }

    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue);
}

#[cfg(test)]
mod test {

    use super::*;
    use crate::core::ical::parser::common::param_text;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_quote_string_if_needed() {
        assert_eq!(
            quote_string_if_needed(&String::from("To be quoted; + 🎄 , STRING"), param_text,),
            String::from(r#""To be quoted; + 🎄 , STRING""#),
        );

        assert_eq!(
            quote_string_if_needed(&String::from("No need to be quoted"), param_text,),
            String::from("No need to be quoted"),
        );
    }
}
