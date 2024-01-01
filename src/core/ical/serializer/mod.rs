use crate::core::ical::parser::common::ParserResult;

mod serialized_value;

pub use serialized_value::SerializedValue;

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

pub trait SerializableICalProperty {
    fn serialize_to_ical(&self) -> String {
        let (name, params, value) = self.serialize_to_split_ical();

        let mut serialized_property = name.clone();

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

        serialized_property
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
