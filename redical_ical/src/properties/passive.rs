use nom::branch::alt;
use nom::combinator::map;

use crate::content_line::{ContentLine, ContentLineParams};

use crate::{ICalendarEntity, ParserInput, ParserResult, impl_icalendar_entity_traits};

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum PassiveProperty {
    Calscale(ContentLineParams, String),
    Method(ContentLineParams, String),
    Prodid(ContentLineParams, String),
    Version(ContentLineParams, String),
    Attach(ContentLineParams, String),
    Comment(ContentLineParams, String),
    PercentComplete(ContentLineParams, String),
    Priority(ContentLineParams, String),
    Status(ContentLineParams, String),
    Completed(ContentLineParams, String),
    Due(ContentLineParams, String),
    Freebusy(ContentLineParams, String),
    Transp(ContentLineParams, String),
    Tzid(ContentLineParams, String),
    Tzname(ContentLineParams, String),
    Tzoffsetfrom(ContentLineParams, String),
    Tzoffsetto(ContentLineParams, String),
    Tzurl(ContentLineParams, String),
    Attendee(ContentLineParams, String),
    Contact(ContentLineParams, String),
    Organizer(ContentLineParams, String),
    Url(ContentLineParams, String),
    Action(ContentLineParams, String),
    Repeat(ContentLineParams, String),
    Trigger(ContentLineParams, String),
    Created(ContentLineParams, String),
    Dtstamp(ContentLineParams, String),
    LastModified(ContentLineParams, String),
    Sequence(ContentLineParams, String),
    RequestStatus(ContentLineParams, String),
    Xml(ContentLineParams, String),
    Tzuntil(ContentLineParams, String),
    TzidAliasOf(ContentLineParams, String),
    Busytype(ContentLineParams, String),
    Name(ContentLineParams, String),
    RefreshInterval(ContentLineParams, String),
    Source(ContentLineParams, String),
    Color(ContentLineParams, String),
    Image(ContentLineParams, String),
    Conference(ContentLineParams, String),
    CalendarAddress(ContentLineParams, String),
    LocationType(ContentLineParams, String),
    ParticipantType(ContentLineParams, String),
    ResourceType(ContentLineParams, String),
    StructuredData(ContentLineParams, String),
    StyledDescription(ContentLineParams, String),
    Acknowledged(ContentLineParams, String),
    Proximity(ContentLineParams, String),
    Concept(ContentLineParams, String),
    Link(ContentLineParams, String),
    Refid(ContentLineParams, String),
    Description(ContentLineParams, String),
    Summary(ContentLineParams, String),
    Location(ContentLineParams, String),
    X(ContentLine),
}

impl ICalendarEntity for PassiveProperty {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> {
        macro_rules! define_property_parser {
            ($enum_key:ident, $property_name:expr) => {
                map(
                    ContentLine::parse_ical_for_property($property_name),
                    |content_line| PassiveProperty::$enum_key(content_line.1, content_line.2)
                )
            }
        }

        alt((
            alt((
                define_property_parser!(Calscale, "CALSCALE"),
                define_property_parser!(Method, "METHOD"),
                define_property_parser!(Prodid, "PRODID"),
                define_property_parser!(Version, "VERSION"),
                define_property_parser!(Attach, "ATTACH"),
                define_property_parser!(Comment, "COMMENT"),
                define_property_parser!(PercentComplete, "PERCENT-COMPLETE"),
                define_property_parser!(Priority, "PRIORITY"),
                define_property_parser!(Status, "STATUS"),
                define_property_parser!(Completed, "COMPLETED"),
                define_property_parser!(Due, "DUE"),
                define_property_parser!(Freebusy, "FREEBUSY"),
                define_property_parser!(Transp, "TRANSP"),
                define_property_parser!(Tzid, "TZID"),
                define_property_parser!(Tzname, "TZNAME"),
                define_property_parser!(Tzoffsetfrom, "TZOFFSETFROM"),
                define_property_parser!(Tzoffsetto, "TZOFFSETTO"),
                define_property_parser!(Tzurl, "TZURL"),
            )),
            alt((
                define_property_parser!(Attendee, "ATTENDEE"),
                define_property_parser!(Contact, "CONTACT"),
                define_property_parser!(Organizer, "ORGANIZER"),
                define_property_parser!(Url, "URL"),
                define_property_parser!(Action, "ACTION"),
                define_property_parser!(Repeat, "REPEAT"),
                define_property_parser!(Trigger, "TRIGGER"),
                define_property_parser!(Created, "CREATED"),
                define_property_parser!(Dtstamp, "DTSTAMP"),
                define_property_parser!(LastModified, "LAST-MODIFIED"),
                define_property_parser!(Sequence, "SEQUENCE"),
                define_property_parser!(RequestStatus, "REQUEST-STATUS"),
                define_property_parser!(Xml, "XML"),
                define_property_parser!(Tzuntil, "TZUNTIL"),
                define_property_parser!(TzidAliasOf, "TZID-ALIAS-OF"),
                define_property_parser!(Busytype, "BUSYTYPE"),
                define_property_parser!(Name, "NAME"),
                define_property_parser!(RefreshInterval, "REFRESH-INTERVAL"),
            )),
            alt((
                define_property_parser!(Source, "SOURCE"),
                define_property_parser!(Color, "COLOR"),
                define_property_parser!(Image, "IMAGE"),
                define_property_parser!(Conference, "CONFERENCE"),
                define_property_parser!(CalendarAddress, "CALENDAR-ADDRESS"),
                define_property_parser!(LocationType, "LOCATION-TYPE"),
                define_property_parser!(ParticipantType, "PARTICIPANT-TYPE"),
                define_property_parser!(ResourceType, "RESOURCE-TYPE"),
                define_property_parser!(StructuredData, "STRUCTURED-DATA"),
                define_property_parser!(StyledDescription, "STYLED-DESCRIPTION"),
                define_property_parser!(Acknowledged, "ACKNOWLEDGED"),
                define_property_parser!(Proximity, "PROXIMITY"),
                define_property_parser!(Concept, "CONCEPT"),
                define_property_parser!(Link, "LINK"),
                define_property_parser!(Refid, "REFID"),
                define_property_parser!(Description, "DESCRIPTION"),
                define_property_parser!(Summary, "SUMMARY"),
                define_property_parser!(Location, "LOCATION"),
            )),
            map(ContentLine::parse_ical_for_x_property(), Self::X),
        ))(input)
    }

    fn render_ical(&self) -> String {
        ContentLine::from(self).render_ical()
    }
}

impl From<&PassiveProperty> for ContentLine {
    fn from(property: &PassiveProperty) -> Self {
        ContentLine::from(property.to_owned())
    }
}

impl From<PassiveProperty> for ContentLine {
    fn from(property: PassiveProperty) -> Self {
        match property {
            PassiveProperty::Calscale(params, value)          => ContentLine::from(("CALSCALE", (params, value))),
            PassiveProperty::Method(params, value)            => ContentLine::from(("METHOD", (params, value))),
            PassiveProperty::Prodid(params, value)            => ContentLine::from(("PRODID", (params, value))),
            PassiveProperty::Version(params, value)           => ContentLine::from(("VERSION", (params, value))),
            PassiveProperty::Attach(params, value)            => ContentLine::from(("ATTACH", (params, value))),
            PassiveProperty::Comment(params, value)           => ContentLine::from(("COMMENT", (params, value))),
            PassiveProperty::PercentComplete(params, value)   => ContentLine::from(("PERCENT-COMPLETE", (params, value))),
            PassiveProperty::Priority(params, value)          => ContentLine::from(("PRIORITY", (params, value))),
            PassiveProperty::Status(params, value)            => ContentLine::from(("STATUS", (params, value))),
            PassiveProperty::Completed(params, value)         => ContentLine::from(("COMPLETED", (params, value))),
            PassiveProperty::Due(params, value)               => ContentLine::from(("DUE", (params, value))),
            PassiveProperty::Freebusy(params, value)          => ContentLine::from(("FREEBUSY", (params, value))),
            PassiveProperty::Transp(params, value)            => ContentLine::from(("TRANSP", (params, value))),
            PassiveProperty::Tzid(params, value)              => ContentLine::from(("TZID", (params, value))),
            PassiveProperty::Tzname(params, value)            => ContentLine::from(("TZNAME", (params, value))),
            PassiveProperty::Tzoffsetfrom(params, value)      => ContentLine::from(("TZOFFSETFROM", (params, value))),
            PassiveProperty::Tzoffsetto(params, value)        => ContentLine::from(("TZOFFSETTO", (params, value))),
            PassiveProperty::Tzurl(params, value)             => ContentLine::from(("TZURL", (params, value))),
            PassiveProperty::Attendee(params, value)          => ContentLine::from(("ATTENDEE", (params, value))),
            PassiveProperty::Contact(params, value)           => ContentLine::from(("CONTACT", (params, value))),
            PassiveProperty::Organizer(params, value)         => ContentLine::from(("ORGANIZER", (params, value))),
            PassiveProperty::Url(params, value)               => ContentLine::from(("URL", (params, value))),
            PassiveProperty::Action(params, value)            => ContentLine::from(("ACTION", (params, value))),
            PassiveProperty::Repeat(params, value)            => ContentLine::from(("REPEAT", (params, value))),
            PassiveProperty::Trigger(params, value)           => ContentLine::from(("TRIGGER", (params, value))),
            PassiveProperty::Created(params, value)           => ContentLine::from(("CREATED", (params, value))),
            PassiveProperty::Dtstamp(params, value)           => ContentLine::from(("DTSTAMP", (params, value))),
            PassiveProperty::LastModified(params, value)      => ContentLine::from(("LAST-MODIFIED", (params, value))),
            PassiveProperty::Sequence(params, value)          => ContentLine::from(("SEQUENCE", (params, value))),
            PassiveProperty::RequestStatus(params, value)     => ContentLine::from(("REQUEST-STATUS", (params, value))),
            PassiveProperty::Xml(params, value)               => ContentLine::from(("XML", (params, value))),
            PassiveProperty::Tzuntil(params, value)           => ContentLine::from(("TZUNTIL", (params, value))),
            PassiveProperty::TzidAliasOf(params, value)       => ContentLine::from(("TZID-ALIAS-OF", (params, value))),
            PassiveProperty::Busytype(params, value)          => ContentLine::from(("BUSYTYPE", (params, value))),
            PassiveProperty::Name(params, value)              => ContentLine::from(("NAME", (params, value))),
            PassiveProperty::RefreshInterval(params, value)   => ContentLine::from(("REFRESH-INTERVAL", (params, value))),
            PassiveProperty::Source(params, value)            => ContentLine::from(("SOURCE", (params, value))),
            PassiveProperty::Color(params, value)             => ContentLine::from(("COLOR", (params, value))),
            PassiveProperty::Image(params, value)             => ContentLine::from(("IMAGE", (params, value))),
            PassiveProperty::Conference(params, value)        => ContentLine::from(("CONFERENCE", (params, value))),
            PassiveProperty::CalendarAddress(params, value)   => ContentLine::from(("CALENDAR-ADDRESS", (params, value))),
            PassiveProperty::LocationType(params, value)      => ContentLine::from(("LOCATION-TYPE", (params, value))),
            PassiveProperty::ParticipantType(params, value)   => ContentLine::from(("PARTICIPANT-TYPE", (params, value))),
            PassiveProperty::ResourceType(params, value)      => ContentLine::from(("RESOURCE-TYPE", (params, value))),
            PassiveProperty::StructuredData(params, value)    => ContentLine::from(("STRUCTURED-DATA", (params, value))),
            PassiveProperty::StyledDescription(params, value) => ContentLine::from(("STYLED-DESCRIPTION", (params, value))),
            PassiveProperty::Acknowledged(params, value)      => ContentLine::from(("ACKNOWLEDGED", (params, value))),
            PassiveProperty::Proximity(params, value)         => ContentLine::from(("PROXIMITY", (params, value))),
            PassiveProperty::Concept(params, value)           => ContentLine::from(("CONCEPT", (params, value))),
            PassiveProperty::Link(params, value)              => ContentLine::from(("LINK", (params, value))),
            PassiveProperty::Refid(params, value)             => ContentLine::from(("REFID", (params, value))),
            PassiveProperty::Description(params, value)       => ContentLine::from(("DESCRIPTION", (params, value))),
            PassiveProperty::Summary(params, value)           => ContentLine::from(("SUMMARY", (params, value))),
            PassiveProperty::Location(params, value)          => ContentLine::from(("LOCATION", (params, value))),

            PassiveProperty::X(content_line) => content_line,
        }
    }
}

impl_icalendar_entity_traits!(PassiveProperty);

#[cfg(test)]
mod tests {
    use super::*;

    use crate::tests::assert_parser_output;

    #[test]
    fn parse_ical() {
        macro_rules! assert_passive_property_parse_ical {
            ($enum_key:ident, $property_name:expr) => {
                assert_parser_output!(
                    PassiveProperty::parse_ical(concat!($property_name, ";X-KEY=X-VALUE;KEY=VALUE:VALUE").into()),
                    (
                        "",
                        PassiveProperty::$enum_key(ContentLineParams::from(vec![("X-KEY", "X-VALUE"), ("KEY", "VALUE")]), String::from("VALUE"))
                    )
                );
            }
        }

        assert_passive_property_parse_ical!(Calscale, "CALSCALE");
        assert_passive_property_parse_ical!(Method, "METHOD");
        assert_passive_property_parse_ical!(Prodid, "PRODID");
        assert_passive_property_parse_ical!(Version, "VERSION");
        assert_passive_property_parse_ical!(Attach, "ATTACH");
        assert_passive_property_parse_ical!(Comment, "COMMENT");
        assert_passive_property_parse_ical!(PercentComplete, "PERCENT-COMPLETE");
        assert_passive_property_parse_ical!(Priority, "PRIORITY");
        assert_passive_property_parse_ical!(Status, "STATUS");
        assert_passive_property_parse_ical!(Completed, "COMPLETED");
        assert_passive_property_parse_ical!(Due, "DUE");
        assert_passive_property_parse_ical!(Freebusy, "FREEBUSY");
        assert_passive_property_parse_ical!(Transp, "TRANSP");
        assert_passive_property_parse_ical!(Tzid, "TZID");
        assert_passive_property_parse_ical!(Tzname, "TZNAME");
        assert_passive_property_parse_ical!(Tzoffsetfrom, "TZOFFSETFROM");
        assert_passive_property_parse_ical!(Tzoffsetto, "TZOFFSETTO");
        assert_passive_property_parse_ical!(Tzurl, "TZURL");
        assert_passive_property_parse_ical!(Attendee, "ATTENDEE");
        assert_passive_property_parse_ical!(Contact, "CONTACT");
        assert_passive_property_parse_ical!(Organizer, "ORGANIZER");
        assert_passive_property_parse_ical!(Url, "URL");
        assert_passive_property_parse_ical!(Action, "ACTION");
        assert_passive_property_parse_ical!(Repeat, "REPEAT");
        assert_passive_property_parse_ical!(Trigger, "TRIGGER");
        assert_passive_property_parse_ical!(Created, "CREATED");
        assert_passive_property_parse_ical!(Dtstamp, "DTSTAMP");
        assert_passive_property_parse_ical!(LastModified, "LAST-MODIFIED");
        assert_passive_property_parse_ical!(Sequence, "SEQUENCE");
        assert_passive_property_parse_ical!(RequestStatus, "REQUEST-STATUS");
        assert_passive_property_parse_ical!(Xml, "XML");
        assert_passive_property_parse_ical!(Tzuntil, "TZUNTIL");
        assert_passive_property_parse_ical!(TzidAliasOf, "TZID-ALIAS-OF");
        assert_passive_property_parse_ical!(Busytype, "BUSYTYPE");
        assert_passive_property_parse_ical!(Name, "NAME");
        assert_passive_property_parse_ical!(RefreshInterval, "REFRESH-INTERVAL");
        assert_passive_property_parse_ical!(Source, "SOURCE");
        assert_passive_property_parse_ical!(Color, "COLOR");
        assert_passive_property_parse_ical!(Image, "IMAGE");
        assert_passive_property_parse_ical!(Conference, "CONFERENCE");
        assert_passive_property_parse_ical!(CalendarAddress, "CALENDAR-ADDRESS");
        assert_passive_property_parse_ical!(LocationType, "LOCATION-TYPE");
        assert_passive_property_parse_ical!(ParticipantType, "PARTICIPANT-TYPE");
        assert_passive_property_parse_ical!(ResourceType, "RESOURCE-TYPE");
        assert_passive_property_parse_ical!(StructuredData, "STRUCTURED-DATA");
        assert_passive_property_parse_ical!(StyledDescription, "STYLED-DESCRIPTION");
        assert_passive_property_parse_ical!(Acknowledged, "ACKNOWLEDGED");
        assert_passive_property_parse_ical!(Proximity, "PROXIMITY");
        assert_passive_property_parse_ical!(Concept, "CONCEPT");
        assert_passive_property_parse_ical!(Link, "LINK");
        assert_passive_property_parse_ical!(Refid, "REFID");
        assert_passive_property_parse_ical!(Description, "DESCRIPTION");
        assert_passive_property_parse_ical!(Summary, "SUMMARY");
        assert_passive_property_parse_ical!(Location, "LOCATION");

        assert_parser_output!(
            PassiveProperty::parse_ical("X-PROPERTY;X-KEY=X-VALUE;KEY=VALUE:VALUE".into()),
            (
                "",
                PassiveProperty::X(
                    ContentLine::from(("X-PROPERTY", vec![("X-KEY", "X-VALUE"), ("KEY", "VALUE")], "VALUE"))
                )
            )
        );
    }
}
