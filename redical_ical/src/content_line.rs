use nom::error::context;
use nom::bytes::complete::tag;
use nom::sequence::{preceded, terminated, tuple};
use nom::multi::many0;
use nom::combinator::{cut, map, opt};

use crate::grammar::{colon, semicolon, name, param, value, crlf};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Clone, Eq, PartialEq, Default)]
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

    fn render_ical(&self) -> String {
        format!("{}={}", self.0, self.1)
    }
}

impl_icalendar_entity_traits!(ContentLineParam);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ContentLineParams(pub Vec<ContentLineParam>);

impl<'a> From<Vec<ContentLineParam>> for ContentLineParams {
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

    fn render_ical(&self) -> String {
        let mut output = String::new();

        for param in &self.0 {
            output.push_str(format!(";{}", param.render_ical()).as_str());
        }

        output
    }
}

impl_icalendar_entity_traits!(ContentLineParams);

#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct ContentLine(pub String, pub ContentLineParams, pub String);

impl<'a> From<(ParserInput<'a>, ContentLineParams, ParserInput<'a>)> for ContentLine {
    fn from(content_line: (ParserInput, ContentLineParams, ParserInput)) -> Self {
        ContentLine(
            content_line.0.to_string(),
            content_line.1,
            content_line.2.to_string(),
        )
    }
}

impl From<(String, Vec<(String, String)>, String)> for ContentLine {
    fn from(content_line: (String, Vec<(String, String)>, String)) -> Self {
        ContentLine(
            content_line.0.into(),
            content_line.1.into(),
            content_line.2.into(),
        )
    }
}

impl From<(&str, Vec<(&str, &str)>, &str)> for ContentLine {
    fn from(content_line: (&str, Vec<(&str, &str)>, &str)) -> Self {
        ContentLine(
            content_line.0.to_string(),
            content_line.1.into(),
            content_line.2.to_string(),
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
                        terminated(
                            preceded(colon, value),
                            opt(crlf),
                        )
                    )
                ),
                ContentLine::from,
            )
        )(input)
    }

    fn render_ical(&self) -> String {
        format!("{}{}:{}", self.0, self.1.render_ical(), self.2)
    }
}

impl ContentLine {
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
                                    terminated(
                                        preceded(colon, value),
                                        opt(crlf),
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

impl_icalendar_entity_traits!(ContentLine);

#[cfg(test)]
mod tests {
    use super::*;

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
}
