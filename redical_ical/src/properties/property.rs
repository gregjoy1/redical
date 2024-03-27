use nom::combinator::map;

use crate::content_line::ContentLine;

use crate::{ICalendarEntity, ParserInput, ParserResult};

use std::collections::HashMap;

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct Property {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}

impl ICalendarEntity for Property {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(ContentLine::parse_ical, Property::from)(input)
    }

    fn render_ical(&self) -> String {
        let mut output = self.name.clone();

        for (key, value) in &self.params {
            output.push_str(format!("{key}={value}").as_str());
        }

        output.push_str(self.value.as_str());

        output
    }
}

impl Property {
    fn parse_ical_for_property(property_name: &'static str) -> impl FnMut(ParserInput) -> ParserResult<Property> {
        move |input: ParserInput| {
            map(ContentLine::parse_ical_for_property(property_name), Property::from)(input)
        }
    }
}

impl<'a> From<ContentLine> for Property {
    fn from(content_line: ContentLine) -> Self {
        let name: String = content_line.0.to_string();

        let params: HashMap<String, String> =
            content_line.1
                        .0
                        .into_iter()
                        .map(|param| (param.0, param.1))
                        .collect();

        let value: String = content_line.2.to_string();

        Property { name, params, value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical_for_property() {
        assert_parser_output!(
            Property::parse_ical_for_property("DESCRIPTION")(r#"DESCRIPTION;ALTREP="cid:part1.0001@example.org":The Fall'98 WildWizards Conference - - Las Vegas\, NV\, USA"#.into()),
            (
                "",
                Property {
                    name: String::from("DESCRIPTION"),
                    params: HashMap::from([(String::from("ALTREP"), String::from(r#""cid:part1.0001@example.org""#))]), 
                    value: String::from(r#"The Fall'98 WildWizards Conference - - Las Vegas\, NV\, USA"#),
                },
            ),
        );

        assert!(Property::parse_ical_for_property("DESCRIPTION")("TEST;PARAM-KEY=PARAM_VALUE:VALUE".into()).is_err());
        assert!(Property::parse_ical_for_property("DESCRIPTION")("".into()).is_err());
    }

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            Property::parse_ical("TEST;X-TEST=X_VALUE;TEST=VALUE:VALUE TEXT".into()),
            (
                "",
                Property {
                    name: String::from("TEST"),
                    params: HashMap::from([(String::from("X-TEST"), String::from("X_VALUE")), (String::from("TEST"), String::from("VALUE"))]), 
                    value: String::from("VALUE TEXT"),
                },
            ),
        );

        assert_parser_output!(
            Property::parse_ical("TEST;X-TEST-ONE=X_VALUE_ONE;TEST=VALUE;X-TEST-TWO=X_VALUE_TWO;X-TEST-ONE=X_VALUE_ONE_UPDATED:VALUE TEXT".into()),
            (
                "",
                Property {
                    name: String::from("TEST"),
                    params: HashMap::from([
                        (String::from("X-TEST-ONE"), String::from("X_VALUE_ONE_UPDATED")),
                        (String::from("X-TEST-TWO"), String::from("X_VALUE_TWO")),
                        (String::from("TEST"), String::from("VALUE")),
                    ]), 
                    value: String::from("VALUE TEXT"),
                },
            ),
        );

        assert!(Property::parse_ical("".into()).is_err());
    }
}
