/*
use serde::{Deserialize, Serialize};

use nom::{
    branch::alt,
    combinator::{all_consuming, map, opt},
    error::context,
    multi::separated_list1,
};

use std::str::FromStr;

use crate::{convert_error, ParserInput, ParserResult, ParserError};

use crate::grammar::wsp;

use crate::properties::component::{
    descriptive::{
        categories::Categories,
        class::Class,
        geo::Geo,
    },
    date_and_time::{
        dtstart::Dtstart,
        dtend::Dtend,
        duration::Duration,
    },
    recurrence::{
        rrule::Rrule,
        rdate::Rdate,
        exdate::Exdate,
    },
};
    Resources(ResourcesProperty),       //  "RESOURCES"
    Categories(CategoriesProperty),     //  "CATEGORIES"
    Class(ClassProperty),               //  "CLASS"
    Geo(GeoProperty),                   //  "GEO"
    Description(DescriptionProperty),   //  "DESCRIPTION"
    RecurrenceID(RecurrenceIDProperty), // "RECURRENCE-ID"
    DTEnd(DTEndProperty),               //  "DTEND"
    DTStart(DTStartProperty),           //  "DTSTART"
    Duration(DurationProperty),         //  "DURATION"
    ExDate(ExDateProperty),             //  "EXDATE"
    ExRule(ExRuleProperty),             //  "EXRULE"
    RRule(RRuleProperty),               //  "RRULE"
    Location(LocationProperty),         //  "LOCATION"
    RDate(RDateProperty),               //  "RDATE"
    RelatedTo(RelatedToProperty),       //  "RELATED-TO"
    Summary(SummaryProperty),           //  "SUMMARY"
    UID(UIDProperty),                   //  "UID"
    X(XProperty),                       //  "X-*"

use crate::ical::properties::*;
use crate::ical::serializer::{
    SerializableICalProperty, SerializationPreferences, SerializedValue,
};

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq, Clone, Ord, PartialOrd)]
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
    Resources(ResourcesProperty),       //  "RESOURCES"
    Categories(CategoriesProperty),     //  "CATEGORIES"
    Class(ClassProperty),               //  "CLASS"
    Geo(GeoProperty),                   //  "GEO"
    Description(DescriptionProperty),   //  "DESCRIPTION"
    RecurrenceID(RecurrenceIDProperty), // "RECURRENCE-ID"
    DTEnd(DTEndProperty),               //  "DTEND"
    DTStart(DTStartProperty),           //  "DTSTART"
    Duration(DurationProperty),         //  "DURATION"
    ExDate(ExDateProperty),             //  "EXDATE"
    ExRule(ExRuleProperty),             //  "EXRULE"
    RRule(RRuleProperty),               //  "RRULE"
    Location(LocationProperty),         //  "LOCATION"
    RDate(RDateProperty),               //  "RDATE"
    RelatedTo(RelatedToProperty),       //  "RELATED-TO"
    Summary(SummaryProperty),           //  "SUMMARY"
    UID(UIDProperty),                   //  "UID"
    X(XProperty),                       //  "X-*"
}

impl FromStr for Property {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        all_consuming(Property::parse_ical)(input)
            .map(|(_remaining, property)| property)
            .map_err(|error| {
                if let nom::Err::Error(error) = error {
                    convert_error(input, error)
                } else {
                    error.to_string()
                }
            })
    }
}

impl SerializableICalProperty for Property {
    fn serialize_to_split_ical(
        &self,
        preferences: Option<&SerializationPreferences>,
    ) -> (String, Option<Vec<(String, String)>>, SerializedValue) {
        match self {
            Self::Resources(property) => property.serialize_to_split_ical(preferences),
            Self::Categories(property) => property.serialize_to_split_ical(preferences),
            Self::Class(property) => property.serialize_to_split_ical(preferences),
            Self::Geo(property) => property.serialize_to_split_ical(preferences),
            Self::Description(property) => property.serialize_to_split_ical(preferences),
            Self::RecurrenceID(property) => property.serialize_to_split_ical(preferences),
            Self::DTEnd(property) => property.serialize_to_split_ical(preferences),
            Self::DTStart(property) => property.serialize_to_split_ical(preferences),
            Self::Duration(property) => property.serialize_to_split_ical(preferences),
            Self::ExDate(property) => property.serialize_to_split_ical(preferences),
            Self::ExRule(property) => property.serialize_to_split_ical(preferences),
            Self::RRule(property) => property.serialize_to_split_ical(preferences),
            Self::Location(property) => property.serialize_to_split_ical(preferences),
            Self::RDate(property) => property.serialize_to_split_ical(preferences),
            Self::RelatedTo(property) => property.serialize_to_split_ical(preferences),
            Self::Summary(property) => property.serialize_to_split_ical(preferences),
            Self::UID(property) => property.serialize_to_split_ical(preferences),
            Self::X(property) => property.serialize_to_split_ical(preferences),
        }
    }
}

impl Property {
    // Compare property names only, ignore the content.
    pub fn property_name_eq(&self, other: &Self) -> bool {
        // Use std::mem::discriminant to compare enum variant without comparing the data.
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn parse_ical(input: &str) -> ParserResult<&str, Self> {
        context(
            "property",
            alt((
                map(ResourcesProperty::parse_ical, Self::Resources), //  "RESOURCES"
                map(CategoriesProperty::parse_ical, Self::Categories), //  "CATEGORIES"
                map(ClassProperty::parse_ical, Self::Class),         //  "CLASS"
                map(GeoProperty::parse_ical, Self::Geo),             //  "GEO"
                map(DescriptionProperty::parse_ical, Self::Description), //  "DESCRIPTION"
                map(RecurrenceIDProperty::parse_ical, Self::RecurrenceID), //  "RECURRENCE-ID"
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
        let parsed_properties =
            all_consuming(separated_list1(wsp, Property::parse_ical))(input);

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
            "RECURRENCE-ID:20201231T183000Z",
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
            Some(Property::RecurrenceID(_))
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

*/
