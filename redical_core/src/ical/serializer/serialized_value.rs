#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub enum SerializedValue {
    Single(String),
    List(Vec<String>),
    Params(Vec<(String, SerializedValue)>),
}

impl ToString for SerializedValue {
    fn to_string(&self) -> String {
        match self {
            Self::Single(value) => value.clone(),
            Self::List(values) => values.join(","),
            Self::Params(values) => {
                let key_value_pairs: Vec<String> = values
                    .iter()
                    .map(|(key, value)| format!("{}={}", key, value.to_string()))
                    .collect();

                key_value_pairs.join(";")
            }
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;
    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_serialized_value_to_string() {
        assert_eq!(
            SerializedValue::Single(String::from("Single Value")).to_string(),
            String::from("Single Value"),
        );

        assert_eq!(
            SerializedValue::List(vec![String::from("List Value")]).to_string(),
            String::from("List Value"),
        );

        assert_eq!(
            SerializedValue::List(vec![
                String::from("List Value One"),
                String::from("List Value Two")
            ])
            .to_string(),
            String::from("List Value One,List Value Two"),
        );

        assert_eq!(
            SerializedValue::Params(vec![
                (
                    String::from("SINGLE_VALUE_KEY"),
                    SerializedValue::Single(String::from("SINGLE_VALUE"))
                ),
                (
                    String::from("LIST_VALUE_KEY"),
                    SerializedValue::List(vec![
                        String::from("LIST_VALUE_ONE"),
                        String::from("LIST_VALUE_TWO")
                    ])
                ),
            ])
            .to_string(),
            String::from(
                "SINGLE_VALUE_KEY=SINGLE_VALUE;LIST_VALUE_KEY=LIST_VALUE_ONE,LIST_VALUE_TWO"
            ),
        );
    }
}
