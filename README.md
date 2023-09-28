# RediCal

## Design doc/notes

### Query events with crude/adapted iCal notation

```
FIND FIRST-OF-GROUP (UUID) WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-GEO-DIST-DTSTART LIMIT 50;
FIND ALL-OF-GROUP (RELATED-TO;RELTYPE=PARENT) WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-GEO-DIST-DTSTART LIMIT 50;
FIND ALL-OF-GROUP (RELATED-TO;RELTYPE=PARENT) WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-DTSTART LIMIT 50;
FIND ALL-OF-GROUP (RELATED-TO;RELTYPE=PARENT) WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-DTSTART-GEO-DIST LIMIT 50;

FIND FIRST WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-GEO-DIST-DTSTART LIMIT 50;
FIND ALL WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-DTSTART-GEO-DIST LIMIT 50;
FIND ALL WHERE (CATEGORIES;OP=ALL:CATEGORY_ONE,CATEGORY_TWO AND RELATED-TO;RELTYPE=PARENT:PARENT_UUID AND GEO;DIST=3KM:48.85299;2.36885) ORDER-BY-DTSTART LIMIT 50;
FIND ALL;
```

### Specify events with crude iCal notation

#### Event specific properties

##### UID

Use key as UID for event (indexed).

##### CATEGORIES

Arbituary text string list for searching (indexed).

##### DTSTART, RRULE, EXRULE, EXDATE, RDATE, TZID

Using [rust-rrule crate](https://github.com/fmeringdal/rust-rrule):

Parse and store each of these individually as `rrule::RRule`/`chrono::{DateTime, TimeZone}` and later concat into `rrule::RRuleSet` for validation/expansion.

Resulting in indexed `DTSTART` list for all event occurrences within a defined period.

##### CLASS

Either `PUBLIC`/`PRIVATE` to denote un/published status for querying.

##### IMAGE

URL to a hosted image (non-indexed).

##### DESCRIPTION, SUMMARY

Free text fields (non-indexed).

##### RELATED-TO

Multi dimensional tags, for searching events by their relationships:

Example to indicate event with queryable:
* Sibling relationship to other events sharing the same `FREE_TEXT_EVENT_REF`
* Child/Membership relationship to calendars identified with either `FREE_TEXT_CALENDAR_X_REF`, or `FREE_TEXT_CALENDAR_Y_REF`
* Proprietary vendor specific relationship to vendor specific entity identified with either `PROPRIETARY_VENDOR_ENTITY_REF`

'UID:uid1@example.com DTSTAMP:19970714T170000Z ORGANIZER;CN=John Doe:MAILTO:john.doe@example.com DTSTART:19970714T170000Z DTEND:19970715T040000Z SUMMARY:Bastille Day Party GEO:48.85299;2.36885'

```
RELATED-TO;RELTYPE=SIBLING:FREE_TEXT_EVENT_REF

RELATED-TO;RELTYPE=PARENT:FREE_TEXT_CALENDAR_X_REF,FREE_TEXT_CALENDAR_Y_REF

# RELATED-TO;RELTYPE=PARENT default if unspecified
RELATED-TO;FREE_TEXT_CALENDAR_X_REF,FREE_TEXT_CALENDAR_Y_REF

RELATED-TO;RELTYPE=X-VENDOR-RELATIONSHIP:PROPRIETARY_VENDOR_ENTITY_REF
```

##### DTEND, DURATION

Either (not both) defined to establish an extrapolated `DTEND` for each event occurrence (potentially indexed).

##### LOCATION

Free text field for location information (non-indexed).

##### GEO

Defined long/lat for event (indexed).

##### CONTACT, ORGANISER

Free text field for contact information for the event (non-indexed).

#### Event Instance specific properties

##### UID

Pulled from event UUID.

##### CATEGORIES

Arbituary overridable text string list for searching (indexed).

##### CLASS

Either `PUBLIC`/`PRIVATE` to denote un/published overridable status for querying.

##### IMAGE

Overridable URL to a hosted image (non-indexed).

##### DESCRIPTION, SUMMARY

overridable free text fields (non-indexed).

##### RELATED-TO

Multi dimensional tags, for searching event instances by their relationships:

Example to indicate event instance with queryable:
* Overriden Child/Membership relationship to calendars identified with either `FREE_TEXT_CALENDAR_X_REF`, or `FREE_TEXT_CALENDAR_Y_REF`
* Overridden proprietary vendor specific relationship to vendor specific entity identified with either `PROPRIETARY_VENDOR_ENTITY_REF`

```
RELATED-TO;RELTYPE=PARENT:FREE_TEXT_CALENDAR_X_REF,FREE_TEXT_CALENDAR_Y_REF

# RELATED-TO;RELTYPE=PARENT default if unspecified
RELATED-TO;FREE_TEXT_CALENDAR_X_REF,FREE_TEXT_CALENDAR_Y_REF

RELATED-TO;RELTYPE=X-VENDOR-RELATIONSHIP:PROPRIETARY_VENDOR_ENTITY_REF
```

##### DTEND, DURATION

Overridable either (not both) defined to establish an extrapolated `DTEND` for each event occurrence (potentially indexed).

##### LOCATION

Overridable free text field for location information (non-indexed).

##### GEO

Overridable defined long/lat for event (indexed).

##### CONTACT, ORGANISER

Overridable free text field for contact information for the event (non-indexed).

##### RECURRENCE-ID

Generated reference to occurrence date/date-time:

```
RECURRENCE-ID;VALUE=DATE:19960401
RECURRENCE-ID;VALUE=DATE-TIME:19960120T120000Z
```
