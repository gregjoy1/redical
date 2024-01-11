use nom::{
    branch::alt,
    bytes::complete::tag,
    combinator::{map, opt},
    error::{context, convert_error},
    multi::separated_list1,
    sequence::terminated,
};
use std::str::FromStr;

use crate::core::ical::parser::common::ParserResult;
use crate::core::ical::properties::*;
use crate::core::ical::serializer::{SerializableICalProperty, SerializedValue};

#[derive(Debug, Eq, PartialEq, Clone, Ord, PartialOrd)]
pub enum Property {
    // TODO: Implement "CALSCALE"
    // TODO: Implement "METHOD"
    // TODO: Implement "PRODID"
    // TODO: Implement "VERSION"
    // TODO: Implement "ATTACH"
    // TODO: Implement "COMMENT"
    // TODO: Implement "PERCENT-COMPLETE"
    // TODO: Implement "PRIORITY"
    // TODO: Implement "STATUS"
    // TODO: Implement "COMPLETED"
    // TODO: Implement "DUE"
    // TODO: Implement "FREEBUSY"
    // TODO: Implement "TRANSP"
    // TODO: Implement "TZID"
    // TODO: Implement "TZNAME"
    // TODO: Implement "TZOFFSETFROM"
    // TODO: Implement "TZOFFSETTO"
    // TODO: Implement "TZURL"
    // TODO: Implement "ATTENDEE"
    // TODO: Implement "CONTACT"
    // TODO: Implement "ORGANIZER"
    // TODO: Implement "RECURRENCE-ID"
    // TODO: Implement "URL"
    // TODO: Implement "ACTION"
    // TODO: Implement "REPEAT"
    // TODO: Implement "TRIGGER"
    // TODO: Implement "CREATED"
    // TODO: Implement "DTSTAMP"
    // TODO: Implement "LAST-MODIFIED"
    // TODO: Implement "SEQUENCE"
    // TODO: Implement "REQUEST-STATUS"
    // TODO: Implement "XML"
    // TODO: Implement "TZUNTIL"
    // TODO: Implement "TZID-ALIAS-OF"
    // TODO: Implement "BUSYTYPE"
    // TODO: Implement "NAME"
    // TODO: Implement "REFRESH-INTERVAL"
    // TODO: Implement "SOURCE"
    // TODO: Implement "COLOR"
    // TODO: Implement "IMAGE"
    // TODO: Implement "CONFERENCE"
    // TODO: Implement "CALENDAR-ADDRESS"
    // TODO: Implement "LOCATION-TYPE"
    // TODO: Implement "PARTICIPANT-TYPE"
    // TODO: Implement "RESOURCE-TYPE"
    // TODO: Implement "STRUCTURED-DATA"
    // TODO: Implement "STYLED-DESCRIPTION"
    // TODO: Implement "ACKNOWLEDGED"
    // TODO: Implement "PROXIMITY"
    // TODO: Implement "CONCEPT"
    // TODO: Implement "LINK"
    // TODO: Implement "REFID"
    Resources(ResourcesProperty),     //  "RESOURCES"
    Categories(CategoriesProperty),   //  "CATEGORIES"
    Class(ClassProperty),             //  "CLASS"
    Geo(GeoProperty),                 //  "GEO"
    Description(DescriptionProperty), //  "DESCRIPTION"
    DTEnd(DTEndProperty),             //  "DTEND"
    DTStart(DTStartProperty),         //  "DTSTART"
    Duration(DurationProperty),       //  "DURATION"
    ExDate(ExDateProperty),           //  "EXDATE"
    ExRule(ExRuleProperty),           //  "EXRULE"
    RRule(RRuleProperty),             //  "RRULE"
    Location(LocationProperty),       //  "LOCATION"
    RDate(RDateProperty),             //  "RDATE"
    RelatedTo(RelatedToProperty),     //  "RELATED-TO"
    Summary(SummaryProperty),         //  "SUMMARY"
    UID(UIDProperty),                 //  "UID"
    X(XProperty),                     //  "X-*"
}

impl SerializableICalProperty for Property {
    fn serialize_to_split_ical(&self) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        match self {
            Self::Resources(property) => property.serialize_to_split_ical(),
            Self::Categories(property) => property.serialize_to_split_ical(),
            Self::Class(property) => property.serialize_to_split_ical(),
            Self::Geo(property) => property.serialize_to_split_ical(),
            Self::Description(property) => property.serialize_to_split_ical(),
            Self::DTEnd(property) => property.serialize_to_split_ical(),
            Self::DTStart(property) => property.serialize_to_split_ical(),
            Self::Duration(property) => property.serialize_to_split_ical(),
            Self::ExDate(property) => property.serialize_to_split_ical(),
            Self::ExRule(property) => property.serialize_to_split_ical(),
            Self::RRule(property) => property.serialize_to_split_ical(),
            Self::Location(property) => property.serialize_to_split_ical(),
            Self::RDate(property) => property.serialize_to_split_ical(),
            Self::RelatedTo(property) => property.serialize_to_split_ical(),
            Self::Summary(property) => property.serialize_to_split_ical(),
            Self::UID(property) => property.serialize_to_split_ical(),
            Self::X(property) => property.serialize_to_split_ical(),
        }
    }
}

impl Property {
    pub fn parse_ical(input: &str) -> ParserResult<&str, Self> {
        context(
            "property",
            alt((
                map(ResourcesProperty::parse_ical, Self::Resources), //  "RESOURCES"
                map(CategoriesProperty::parse_ical, Self::Categories), //  "CATEGORIES"
                map(ClassProperty::parse_ical, Self::Class),         //  "CLASS"
                map(GeoProperty::parse_ical, Self::Geo),             //  "GEO"
                map(DescriptionProperty::parse_ical, Self::Description), //  "DESCRIPTION"
                map(DTEndProperty::parse_ical, Self::DTEnd),         //  "DTEND"
                map(DTStartProperty::parse_ical, Self::DTStart),     //  "DTSTART"
                map(DurationProperty::parse_ical, Self::Duration),   //  "DURATION"
                map(ExDateProperty::parse_ical, Self::ExDate),       //  "EXDATE"
                map(ExRuleProperty::parse_ical, Self::ExRule),       //  "EXRULE"
                map(RRuleProperty::parse_ical, Self::RRule),         //  "RRULE"
                map(LocationProperty::parse_ical, Self::Location),   //  "LOCATION"
                map(RDateProperty::parse_ical, Self::RDate),         //  "RDATE"
                map(RelatedToProperty::parse_ical, Self::RelatedTo), //  "RELATED-TO"
                map(SummaryProperty::parse_ical, Self::Summary),     //  "SUMMARY"
                map(UIDProperty::parse_ical, Self::UID),             //  "UID"
                map(XProperty::parse_ical, Self::X),                 //  "X-*"
            )),
        )(input)
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Properties(pub Vec<Property>);

impl FromStr for Properties {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parsed_properties = terminated(
            separated_list1(tag(" "), Property::parse_ical),
            opt(tag(" ")),
        )(input);

        match parsed_properties {
            Ok((_remaining, properties)) => Ok(Properties(properties)),

            Err(error) => {
                if let nom::Err::Error(error) = error {
                    Err(convert_error(input, error))
                } else {
                    Err(error.to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use pretty_assertions_sorted::assert_eq;

    #[test]
    fn test_parse_ical_property() {
        let inputs = vec![
            "CATEGORIES:APPOINTMENT",
            "RELATED-TO:UID",
            "X-PROPERTY:Experimental property text.",
            "RESOURCES:APPOINTMENT",
            "CLASS:PUBLIC",
            "DTEND:20201231T183000Z",
            "DTSTART:20201231T183000Z",
            "EXDATE:20201231T183000Z",
            "RDATE:20201231T183000Z",
            "RRULE:FREQ=DAILY;COUNT=10;INTERVAL=2",
            "EXRULE:FREQ=DAILY;COUNT=10;INTERVAL=2",
            "SUMMARY:Summary text.",
            "DESCRIPTION:Description text.",
            "GEO:37.386013;-122.082932",
            "UID:UID text.",
            "LOCATION:Location text.",
        ];

        let joined_inputs = inputs
            .clone()
            .into_iter()
            .map(String::from)
            .collect::<Vec<String>>()
            .join(" ");

        let parser_result = Properties::from_str(&joined_inputs);

        let Ok(parsed_properties) = parser_result else {
            panic!(
                r#"Expected Property::parse_ical_property to be ok, received: `{:?}`"#,
                parser_result
            );
        };

        let parsed_properties = parsed_properties.0;

        assert_eq!(&parsed_properties.len(), &inputs.len());

        let mut parsed_properties_iter = parsed_properties.into_iter();

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Categories(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::RelatedTo(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::X(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Resources(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Class(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::DTEnd(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::DTStart(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::ExDate(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::RDate(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::RRule(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::ExRule(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Summary(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Description(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Geo(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::UID(_))
        ));

        assert!(matches!(
            parsed_properties_iter.next(),
            Some(Property::Location(_))
        ));

        assert!(matches!(parsed_properties_iter.next(), None));
    }
}
