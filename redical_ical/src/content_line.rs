use nom::error::context;
use nom::sequence::{preceded, tuple};
use nom::multi::many0;
use nom::combinator::{cut, map};

use crate::grammar::{tag, colon, semicolon, x_name, name, param, value};

use crate::{RenderingContext, ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits, terminated_lookahead};

#[derive(Debug, Clone, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct ContentLineParam(pub String, pub String);

impl<'a> From<(ParserInput<'a>, ParserInput<'a>)> for ContentLineParam {
    fn from(param: (ParserInput<'a>, ParserInput<'a>)) -> Self {
        ContentLineParam(
            param.0.to_string(),
            param.1.to_string(),
        )
    }
}

impl From<(String, String)> for ContentLineParam {
    fn from(param: (String, String)) -> Self {
        ContentLineParam(
            param.0,
            param.1,
        )
    }
}

impl From<(&str, &str)> for ContentLineParam {
    fn from(param: (&str, &str)) -> Self {
        ContentLineParam(
            param.0.to_string(),
            param.1.to_string(),
        )
    }
}

impl ICalendarEntity for ContentLineParam {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(param, ContentLineParam::from)(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        format!("{}={}", self.0, self.1)
    }
}

impl_icalendar_entity_traits!(ContentLineParam);

#[derive(Debug, Clone, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct ContentLineParams(pub Vec<ContentLineParam>);

impl From<Vec<ContentLineParam>> for ContentLineParams {
    fn from(params: Vec<ContentLineParam>) -> Self {
        ContentLineParams(params)
    }
}

impl From<Vec<(String, String)>> for ContentLineParams {
    fn from(params: Vec<(String, String)>) -> Self {
        ContentLineParams(
            params.into_iter()
                  .map(ContentLineParam::from)
                  .collect()
        )
    }
}

impl From<Vec<(&str, &str)>> for ContentLineParams {
    fn from(params: Vec<(&str, &str)>) -> Self {
        ContentLineParams(
            params.into_iter()
                  .map(ContentLineParam::from)
                  .collect()
        )
    }
}

impl ICalendarEntity for ContentLineParams {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        map(
            many0(
                preceded(
                    semicolon,
                    cut(ContentLineParam::parse_ical),
                )
            ),
            ContentLineParams::from,
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        let mut output = String::new();

        for param in &self.0 {
            output.push_str(format!(";{}", param.render_ical()).as_str());
        }

        output
    }
}

impl ContentLineParams {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn insert(&mut self, key: String, value: String) {
        self.0.push(
            ContentLineParam(key, value)
        );
    }
}

impl_icalendar_entity_traits!(ContentLineParams);

#[derive(Debug, Clone, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct ContentLine(pub String, pub ContentLineParams, pub String);

impl<'a> From<(ParserInput<'a>, ContentLineParams, ParserInput<'a>)> for ContentLine {
    fn from((name, params, value): (ParserInput, ContentLineParams, ParserInput)) -> Self {
        ContentLine(
            name.to_string(),
            params,
            value.to_string(),
        )
    }
}

impl From<(&str, (ContentLineParams, String))> for ContentLine {
    fn from((name, (params, value)): (&str, (ContentLineParams, String))) -> Self {
        ContentLine(name.to_string(), params, value)
    }
}

impl From<(&str, (&ContentLineParams, &String))> for ContentLine {
    fn from((name, (params, value)): (&str, (&ContentLineParams, &String))) -> Self {
        ContentLine(name.to_string(), params.to_owned(), value.to_owned())
    }
}

impl From<(String, Vec<(String, String)>, String)> for ContentLine {
    fn from((name, params, value): (String, Vec<(String, String)>, String)) -> Self {
        ContentLine(name, params.into(), value)
    }
}

impl From<(&str, Vec<(&str, &str)>, &str)> for ContentLine {
    fn from((name, params, value): (&str, Vec<(&str, &str)>, &str)) -> Self {
        ContentLine(
            name.to_string(),
            params.into(),
            value.to_string(),
        )
    }
}

impl ICalendarEntity for ContentLine {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        context(
            "CONTENTLINE",
            map(
                tuple(
                    (
                        name,
                        ContentLineParams::parse_ical,
                        preceded(
                            colon,
                            terminated_lookahead(
                                value,
                                input.extra.terminating_property_lookahead(),
                            ),
                        )
                    )
                ),
                ContentLine::from,
            )
        )(input)
    }

    fn render_ical_with_context(&self, _context: Option<&RenderingContext>) -> String {
        if self.is_unstructured() {
            return self.2.to_owned();
        }

        format!("{}{}:{}", self.0, self.1.render_ical(), self.2)
    }
}

impl ContentLine {
    pub fn parse_ical_for_x_property() -> impl FnMut(ParserInput) -> ParserResult<Self> {
        move |input: ParserInput| {
            let (remaining, value) =
                context(
                    "X",
                    context(
                        "CONTENTLINE",
                        map(
                            tuple(
                                (
                                    x_name,
                                    ContentLineParams::parse_ical,
                                    preceded(
                                        colon,
                                        terminated_lookahead(
                                            value,
                                            input.extra.terminating_property_lookahead(),
                                        ),
                                    )
                                )
                            ),
                            ContentLine::from,
                        )
                    )
                )(input)?;

            Ok((remaining, value))
        }
    }

    pub fn parse_ical_for_property(property_name: &'static str) -> impl FnMut(ParserInput) -> ParserResult<Self> {
        move |input: ParserInput| {
            let (remaining, value) =
                context(
                    property_name,
                    context(
                        "CONTENTLINE",
                        map(
                            tuple(
                                (
                                    tag(property_name),
                                    ContentLineParams::parse_ical,
                                    preceded(
                                        colon,
                                        terminated_lookahead(
                                            value,
                                            input.extra.terminating_property_lookahead(),
                                        ),
                                    )
                                )
                            ),
                            ContentLine::from,
                        )
                    )
                )(input)?;

            Ok((remaining, value))
        }
    }
}

impl ContentLine {
    pub fn is_unstructured(&self) -> bool {
        self.0.is_empty() && self.1.is_empty()
    }

    pub fn new_unstructured(value: String) -> ContentLine {
        ContentLine(String::new(), ContentLineParams::default(), value)
    }
}

impl_icalendar_entity_traits!(ContentLine);

#[cfg(test)]
mod tests {
    use super::*;

    use nom::multi::separated_list1;
    use crate::grammar::wsp;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            ContentLine::parse_ical(r#"CATEGORIES;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "CATEGORIES",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("LANGUAGE", "ENGLISH"),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        r#"  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"RELATED-TO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";RELTYPE=X-CUSTOM-RELTYPE;X-TEST-KEY-TWO="KEY -🎄- TWO":  UID "#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "RELATED-TO",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("RELTYPE", "X-CUSTOM-RELTYPE"),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "  UID ",
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"X-PROPERTY;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":Experimental property text."#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "X-PROPERTY",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("LANGUAGE", "ENGLISH"),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "Experimental property text.",
                    )
                )
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"RESOURCES;ALTREP="http://xyzcorp.com/conf-rooms/f123.vcf";X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";LANGUAGE=ENGLISH;X-TEST-KEY-TWO="KEY -🎄- TWO":  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "RESOURCES",
                        vec![
                            ("ALTREP", r#""http://xyzcorp.com/conf-rooms/f123.vcf""#),
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("LANGUAGE", "ENGLISH"),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        r#"  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"CLASS;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":PUBLIC"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "CLASS",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "PUBLIC",
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"DTSTART;TZID=Europe/London;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";VALUE=DATE-TIME;X-TEST-KEY-TWO="KEY -🎄- TWO":20201231T183000"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "DTSTART",
                        vec![
                            ("TZID", "Europe/London"),
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("VALUE", "DATE-TIME"),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        r#"20201231T183000"#,
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"RRULE;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":FREQ=MONTHLY;INTERVAL=2;COUNT=10;WKST=SU;UNTIL=19971007T000000Z;BYSECOND=1,30;BYMINUTE=1,30;BYHOUR=1,6;BYDAY=-1SU,2WE;BYWEEKNO=20;BYMONTH=3,6;BYMONTHDAY=7,10;BYYEARDAY=1,30,60;BYEASTER=-1,3;BYSETPOS=3"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "RRULE",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "FREQ=MONTHLY;INTERVAL=2;COUNT=10;WKST=SU;UNTIL=19971007T000000Z;BYSECOND=1,30;BYMINUTE=1,30;BYHOUR=1,6;BYDAY=-1SU,2WE;BYWEEKNO=20;BYMONTH=3,6;BYMONTHDAY=7,10;BYYEARDAY=1,30,60;BYEASTER=-1,3;BYSETPOS=3",
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"GEO;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":37.386013;-122.082932"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "GEO",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "37.386013;-122.082932",
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"UID;X-TEST-KEY-ONE=VALUE_ONE,"VALUE_TWO";X-TEST-KEY-TWO="KEY -🎄- TWO":UID text."#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "UID",
                        vec![
                            ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                            ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                        ],
                        "UID text.",
                    )
                ),
            )
        );

        assert_parser_output!(
            ContentLine::parse_ical(r#"DURATION;X-TEST-KEY-ONE=VALUE_ONE,VALUE_TWO;X-TEST-KEY-TWO=KEY -🎄- TWO:PT25S"#.into()),
            (
                "",
                ContentLine::from(
                    (
                        "DURATION",
                        vec![
                            ("X-TEST-KEY-ONE", "VALUE_ONE,VALUE_TWO"),
                            ("X-TEST-KEY-TWO", "KEY -🎄- TWO"),
                        ],
                        "PT25S",
                    )
                ),
            )
        );
    }

    #[test]
    fn parse_ical_context_terminated_property_lookahead() {
        let categories_property =
            ContentLine::from(
                (
                    "CATEGORIES",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    r#"  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
                )
            );

        let related_property =
            ContentLine::from(
                (
                    "RELATED-TO",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("RELTYPE", "X-CUSTOM-RELTYPE"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    "  UID",
                )
            );

        let x_property =
            ContentLine::from(
                (
                    "X-PROPERTY",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    "Experimental property text.",
                )
            );

        let resources_property =
            ContentLine::from(
                (
                    "RESOURCES",
                    vec![
                        ("ALTREP", r#""http://xyzcorp.com/conf-rooms/f123.vcf""#),
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    r#"  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
                )
            );

        let joined_ical =
            [
                categories_property.render_ical(),
                related_property.render_ical(),
                x_property.render_ical(),
                resources_property.render_ical(),
            ].join(" ");

        assert_parser_output!(
            separated_list1(wsp, ContentLine::parse_ical)(ParserInput::from(joined_ical.as_str())),
            (
                "",
                vec![
                    categories_property,
                    related_property,
                    x_property,
                    resources_property,
                ]
            )
        );
    }

    #[test]
    fn parse_ical_for_x_property_context_terminated_property_lookahead() {
        let x_property =
            ContentLine::from(
                (
                    "X-PROPERTY",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    "Experimental property text.",
                )
            );

        let resources_property =
            ContentLine::from(
                (
                    "RESOURCES",
                    vec![
                        ("ALTREP", r#""http://xyzcorp.com/conf-rooms/f123.vcf""#),
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    r#"  APPOINTMENT ,EDUCATION,"QUOTED, + 🎄 STRING", TESTING\nESCAPED\,CHARS:OK"#,
                )
            );

        let joined_ical =
            [
                x_property.render_ical(),
                resources_property.render_ical(),
            ].join(" ");

        assert_parser_output!(
            ContentLine::parse_ical_for_x_property()(ParserInput::from(joined_ical.as_str())),
            (
                format!(" {}", resources_property.render_ical()),
                x_property,
            )
        );
    }

    #[test]
    fn parse_ical_for_property_context_terminated_property_lookahead() {
        let related_property =
            ContentLine::from(
                (
                    "RELATED-TO",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("RELTYPE", "X-CUSTOM-RELTYPE"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    "  UID",
                )
            );

        let x_property =
            ContentLine::from(
                (
                    "X-PROPERTY",
                    vec![
                        ("X-TEST-KEY-ONE", r#"VALUE_ONE,"VALUE_TWO""#),
                        ("LANGUAGE", "ENGLISH"),
                        ("X-TEST-KEY-TWO", r#""KEY -🎄- TWO""#),
                    ],
                    "Experimental property text.",
                )
            );

        let joined_ical =
            [
                related_property.render_ical(),
                x_property.render_ical(),
            ].join(" ");

        assert_parser_output!(
            ContentLine::parse_ical_for_property("RELATED-TO")(ParserInput::from(joined_ical.as_str())),
            (
                format!(" {}", x_property.render_ical()),
                related_property,
            )
        );
    }
}
