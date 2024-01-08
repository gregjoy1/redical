#[macro_use]
pub mod macros;

pub mod uid_property;
pub mod x_property;

pub mod categories_property;
pub mod description_property;
pub mod location_property;
pub mod related_to_property;
pub mod resources_property;
pub mod summary_property;
pub mod class_property;

pub mod dtend_property;
pub mod dtstart_property;
pub mod exdate_property;
pub mod rdate_property;

pub mod exrule_property;
pub mod rrule_property;

pub mod duration_property;

pub enum Properties {
    // TODO: Implement all these...
    //
    //  "CALSCALE"
    //  "METHOD"
    //  "PRODID"
    //  "VERSION"
    //  "ATTACH"
    //  "COMMENT"
    //  "PERCENT-COMPLETE"
    //  "PRIORITY"
    //  "RESOURCES"
    //  "STATUS"
    //  "COMPLETED"
    //  "DUE"
    //  "FREEBUSY"
    //  "TRANSP"
    //  "TZID"
    //  "TZNAME"
    //  "TZOFFSETFROM"
    //  "TZOFFSETTO"
    //  "TZURL"
    //  "ATTENDEE"
    //  "CONTACT"
    //  "ORGANIZER"
    //  "RECURRENCE-ID"
    //  "URL"
    //  "ACTION"
    //  "REPEAT"
    //  "TRIGGER"
    //  "CREATED"
    //  "DTSTAMP"
    //  "LAST-MODIFIED"
    //  "SEQUENCE"
    //  "REQUEST-STATUS"
    //  "XML"
    //  "TZUNTIL"
    //  "TZID-ALIAS-OF"
    //  "BUSYTYPE"
    //  "NAME"
    //  "REFRESH-INTERVAL"
    //  "SOURCE"
    //  "COLOR"
    //  "IMAGE"
    //  "CONFERENCE"
    //  "CALENDAR-ADDRESS"
    //  "LOCATION-TYPE"
    //  "PARTICIPANT-TYPE"
    //  "RESOURCE-TYPE"
    //  "STRUCTURED-DATA"
    //  "STYLED-DESCRIPTION"
    //  "ACKNOWLEDGED"
    //  "PROXIMITY"
    //  "CONCEPT"
    //  "LINK"
    //  "REFID"

    // NOTE: High priority
    //  "GEO"

    Categories(categories_property::CategoriesProperty),    //  "CATEGORIES"
    Class(class_property::ClassProperty),                   //  "CLASS"
    Description(description_property::DescriptionProperty), //  "DESCRIPTION"
    DTEnd(dtend_property::DTEndProperty),                   //  "DTEND"
    DTStart(dtstart_property::DTStartProperty),             //  "DTSTART"
    Duration(duration_property::DurationProperty),          //  "DURATION"
    ExDate(exdate_property::ExDateProperty),                //  "EXDATE"
    ExRule(exrule_property::ExRuleProperty),                //  "EXRULE"
    RRule(rrule_property::RRuleProperty),                   //  "RRULE"
    Location(location_property::LocationProperty),          //  "LOCATION"
    RDate(rdate_property::RDateProperty),                   //  "RDATE"
    RelatedTo(related_to_property::RelatedToProperty),      //  "RELATED-TO"
    Summary(summary_property::SummaryProperty),             //  "SUMMARY"
    UID(uid_property::UIDProperty),                         //  "UID"
    X(x_property::XProperty),                               //  "X-*"
}

/*
impl Properties {
    pub fn parse_ical(input: &str) -> ParserResult<&str, UIDProperty> {
    }
}
*/
