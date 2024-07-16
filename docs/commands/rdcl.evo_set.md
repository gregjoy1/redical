# RDCL.EVO_SET

### Syntax
```bash
RDCL.EVO_SET key event-uid occurrence-date-string property [property ...]
```

Create (or update if the `event-uid` and `occurrence-date-string` is already in use) an event occurrence override on the specified calendar event with the provided iCalendar properties.

The properties overridden are then reflected in the event instances extrapolated from the event.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

### occurrence-date-string
The date-string of the event occurrence `DTSTART` to override.

### property
The overridden iCalendar ([RFC-5545](https://datatracker.ietf.org/doc/html/rfc5545)) property content lines for a specific date-time/occurrence of an event.

Event overrides in RediCal support most of [RFC-5545 component properties](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1) available to defining events.

The exceptions to this are:
* `RRULE` properties
* `EXRULE` properties
* `RDATE` properties
* `EXDATE` properties

This is because the event occurrence being overridden is derived from the presence of those properties on the event, thus not relevant at the occurrence override level.

#### General properties

##### [`LAST-MODIFIED` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.7.3)
This property specifies the date/time that the stored event occurrence override was last updated (only UTC date time strings are accepted).

If not provided, it is automatically populated with the current date/time.

If provided and **more recent** than that stored on the existing event occurrence override, the command proceeds and the event occurrence override is updated.

If provided and **less recent** than that stored on the existing event occurrence override, the command does **not** proceed, the event occurrence override is **not** updated, and false is returned.

An example of how this can be utilised is when bulk importing event occurrence override data on top of sporadically real time added event occurrence override data. Suppose a stored RediCal calendar is to be populated, a real time/event driven process of updating calendar event occurrence overrides can be enabled whilst a batch process of collecting and adding all event occurrence overrides in bulk can also be started. Any calendar event occurrence override added in real time via the event driven process is not overwritten by the bulk import process if more recent.

###### Examples:

The following is an example of this property defining a last modified UTC date time to second precision.

```
LAST-MODIFIED:20050809T050000Z
```

The following is an example of this property defining a last modified UTC date time to millisecond precision.

```
LAST-MODIFIED;X-MILLIS=123:20050809T050000Z
```

#### Schedule properties

##### [`DTSTART` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.4) - optional
This defines the date/time of the event occurrence being overridden.

> [!NOTE]
> This is optional but if present it should match `occurrence-date-string` parameter.

###### Examples:

The following is an example of this property defining a UTC date time:

```
DTSTART:19980118T073000Z
```

The following is an example of this property defining a date time in the `Europe/London` timezone:

```
DTSTART;TZ=Europe/London:19980118T073000
```

##### [`DTEND` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.2) - optional
This property is used to define an overridden duration for a specific event occurrence.

The `DURATION` of the event is calculated from this property relative to the occurrence `DTSTART` property.

If `DTEND` and `DURATION` are not specified, the `DURATION` will be not be overridden and the occurrence will have the duration defined on the event.

###### Examples:

The following is an example of this property defining a UTC end date time:

```
DTEND:19980118T073000Z
```

The following is an example of this property defining an end date time in the `Europe/London` timezone:

```
DTEND;TZ=Europe/London:19980118T073000
```

##### [`DURATION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.5) - optional
This property is used to define an overridden duration for a specified event occurrence.

This is the relative alternative to defining an event occurrence override absolutely via the `DTEND` property.

If `DTEND` and `DURATION` are not specified, the `DURATION` will be not be overridden and the occurrence will have the duration defined on the event.

###### Examples:

The following is an example of this property that specifies an interval of time of one hour and zero minutes and zero seconds:

```
DURATION:PT1H0M0S
```

The following is an example of this property that specifies an interval of time of 15 minutes.

```
DURATION:PT15M
```

#### Indexed properties

These properties are used to override those actively indexed on the event but for a specific occurrence. This allows overridden event occurrences to be reflected in the query results as exceptions to the event it is associated with.

At the time of writing, there are only a handful of indexed properties, but if used carefully these can offer a lot of flexibility.

##### [`CATEGORIES` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.2)
This property defines the categories overridden for a specific calendar event occurrence.

We can use this to specify "tags", for example `INSTRUCTOR_ABC123`, `FITNESS`, etc.

Especially since we use an inverted index to index this property, this means that we need to query with an exact tag string (case sensitive).

###### Examples:

The following is an example of this property a list of attributes we want to be able to query:

Event defined `CATEGORIES` property:
```
CATEGORIES:YOGA,FITNESS,INSTRUCTOR_ABC123
```

Event occurrence overridden `CATEGORIES` property where `INSTRUCTOR_ABC123` is removed:
```
CATEGORIES:YOGA,FITNESS
```

Event occurrence overridden `CATEGORIES` property where `INSTRUCTOR_ABC123` is replaced with `INSTRUCTOR_DEF456:
```
CATEGORIES:YOGA,FITNESS,INSTRUCTOR_DEF456
```

Event occurrence overridden `CATEGORIES` property where all categories are removed:
```
CATEGORIES:
```

Property parameters can be specified but are ignored when it comes to indexing:

```
CATEGORIES;X-KEY=VALUE;KEY=VALUE:YOGA
```

##### [`LOCATION-TYPE` property](https://datatracker.ietf.org/doc/html/rfc9073#section-6.1)
This property defines the location types for a calendar event.

> [!NOTE]
> This property was introduced later in RFC 9073 and was intended to only be present nested within a `VLOCATION` component (contained within a `VEVENT` component to better describe it's location). 
>
> RediCal intends to eventually implement the `VLOCATION` component in the future, but for now this is available to be specified at the event (`VEVENT`) level.

We can use this to specify **location specific** "tags", for example `ONLINE`, `OFFLINE`, `HOTEL`, `RESTAURANT`, etc.

Especially since we use an inverted index to index this property, this means that we need to query with an exact tag string (case sensitive).

Although similar in nature to the `CATEGORIES` property, the intention behind also including `LOCATION-TYPE` as an indexed property is to add additional granularity to an event definition. Especially when it comes to overrides, conflating location and general concerns within the `CATEGORIES` property would require extensive updates to numerous event occurrence override `CATEGORIES` properties if either concerns are updated.

An example of this is if an event is updated to be hosted online resulting in the only update required being an update to the location specific `LOCATION-TYPE` property on the event itself and those overridden within associated event occurrence overrides. Instead of having to update all `CATEGORIES` properties on all overrides referencing the original location specific tag.

###### Examples:

Event defined `LOCATION-TYPE` property:
```
LOCATION-TYPE:OFFLINE,VILLAGE_HALL
```

Event occurrence overridden `LOCATION-TYPE` property where a specific event is hosted online as a video call:
```
LOCATION-TYPE:ONLINE,ZOOM
```

Resulting in:
```
LOCATION-TYPE:ONLINE,ZOOM
```

Property parameters can be specified but are ignored when it comes to indexing:

```
LOCATION-TYPE;X-KEY=VALUE;KEY=VALUE:HOTEL
```

##### [`RELATED-TO` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.5)
This property defines the relationship or references overridden for a specific calendar event occurrence.

This is similar to `CATEGORIES` except the purpose of this property is to define relationships to tags.

We use an inverted index to index this property, this means that we need to query with an exact `RELTYPE` and tag/UID/slug string value (case sensitive).

It is important to note that we index these with the `RELTYPE` parameter concatenated with the value.

For example, `RELATED-TO;RELTYPE=CHILD:SLUG_ABC_123` is indexed as `CHILDSLUG_ABC_123`, and `RELATED-TO;RELTYPE=X-IMAGE:SLUG_ABC_123` is indexed as `X-IMAGESLUG_ABC_123`.

###### Examples:

The following is an example use of this property where we want to define an experimental relationship with an event and a calendar slug/UID from another system:

Event defined `RELATED-TO` property:
```
RELATED-TO;RELTYPE=X-CALENDAR:CALENDAR_ABC123
RELATED-TO:EVENT_ABC123
```

Event occurrence overridden `RELATED-TO` property where an additional relationship is added:
```
RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123
```

Resulting in:
```
RELATED-TO;RELTYPE=X-CALENDAR:CALENDAR_ABC123
RELATED-TO:EVENT_ABC123
RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123
```

Event occurrence overridden `RELATED-TO` property where the `PARENT` relation is updated:
```
RELATED-TO;RELTYPE=PARENT:OCCURRENCE_DEF456
```

Resulting in:
```
RELATED-TO;RELTYPE=X-CALENDAR:CALENDAR_ABC123
RELATED-TO:OCCURRENCE_DEF456
```

##### [`CLASS` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.3)
This property defines the overridden access classification for a specific calendar event occurrence.

Essentially, as an exception to the associated event, is access to this event occurrence overridden to be public, private, or exclusive.

Similarly to `CATEGORIES`, we use an inverted index to index this property, this means that we need to query with an exact classification string (case sensitive).

###### Examples:

We can use this to specify that an occurrence override for an unpublished event is published:

Event defined `CLASS` property:
```
CLASS:PRIVATE
```

Event occurrence overridden `CLASS` property to reflect that only this specific occurrence is published:
```
CLASS:PUBLIC
```

##### [`GEO` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.6)
This property specifies overridden information related to the global position of a specific event occurrence.

Unlike other indexed properties, we index this with a geospatial index.

Essentially, we use this to record an exception latitude and longitude of a specific event occurrence so that we are able to query for it seperatly to the event it is associated to.

###### Examples:

We can use this to define that a specific event occurrence of an event located in Bristol UK is overridden to occurr in Oxford UK.

Event defined `GEO` property reflecting it being located in Bristol UK:
```
GEO:37.386013;-122.082932
```

Event occurrence overridden `GEO` property defining an exception to this where this it occurrs in Oxford UK instead:
GEO:51.751365550307604;-1.2601196837753945

#### Passive properties

These passive **non**-indexed properties defined on an event can be overridden on for a specific occurrence.

This is useful for properties like `DESCRIPTION`, `LOCATION`, and `IMAGE` that can be overridden and present in the enriched extrapolated event instances.

##### [`SUMMARY` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.12)
This property defines a short summary or subject for the calendar event.

Example: `SUMMARY;LANGUAGE=en-US:Company Holiday Party`

##### [`DESCRIPTION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.5)
This property provides a more complete description of the calendar event than that provided by the "SUMMARY" property.

Example: `DESCRIPTION:This is a long description that exists on a long line.`

##### [`LOCATION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.7)
This property defines the intended venue for the activity defined by a calendar event.

Example: `LOCATION;LANGUAGE=en:Germany`

##### [`CALSCALE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.7.1)
This property defines the calendar scale used for the calendar information specified in the iCalendar object.

Example: `CALSCALE:GREGORIAN`

##### [`METHOD` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.7.2)
This property defines the iCalendar object method associated with the calendar object.

Example: `METHOD:REQUEST`

##### [`PRODID` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.7.3)
This property specifies the identifier for the product that created the iCalendar object.

Example: `PRODID:-//hacksw/handcal//NONSGML v1.0//EN`

##### [`VERSION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.1)
This property specifies the identifier corresponding to the highest version number or the minimum and maximum range of the iCalendar specification that is required in order to interpret the iCalendar object.

Example: `VERSION:2.0`

##### [`ATTACH` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.1)
This property provides the capability to associate a document object with a calendar event.

Example: `ATTACH:http://example.com/public/quarterly-report.doc`

##### [`COMMENT` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.4)
This property specifies non-processing information intended to provide a comment to the calendar user.

Example: `COMMENT:This iCalendar file contains busy time information for`

##### [`PERCENT-COMPLETE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.8)
This property is used by an assignee or delegatee of a to-do to convey the percent completion of a to-do to the "Organizer".

Example: `PERCENT-COMPLETE:39`

##### [`PRIORITY` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.9)
This property defines the relative priority for a calendar event.

Example: `PRIORITY:1`

##### [`STATUS` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.11)
This property defines the overall status or confirmation for the calendar event.

Example: `STATUS:NEEDS-ACTION`

##### [`COMPLETED` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.1)
This property defines the date and time that a to-do was actually completed.

Example: `COMPLETED:20070707T100000Z`

##### [`DUE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.3)
This property defines the date and time that a to-do is expected to be completed.

Example: `DUE;VALUE=DATE:20070501`

##### [`FREEBUSY` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.6)
This property defines one or more free or busy time intervals.

Example: `FREEBUSY;FBTYPE=BUSY:19980415T133000Z/19980415T170000Z`

##### [`TRANSP` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.7)
This property defines whether or not an event is transparent to busy time searches.

Example: `TRANSP:TRANSPARENT`

##### [`TZID` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.3.1)
This property specifies the text value that uniquely identifies the "VTIMEZONE" calendar event in the scope of an iCalendar object.

Example: `TZID:America/New_York`

##### [`TZNAME` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.3.2)
This property specifies the customary designation for a time zone description.

Example: `TZNAME:EDT`

##### [`TZOFFSETFROM` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.3.3)
This property specifies the offset that is in use prior to this time zone observance.

Example: `TZOFFSETFROM:-0400`

##### [`TZOFFSETTO` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.3.4)
This property specifies the offset that is in use in this time zone observance.

Example: `TZOFFSETTO:-0400`

##### [`TZURL` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.3.5)
This property provides a means for a "VTIMEZONE" component to point to a network location that can be used to retrieve an up-to-date version of itself.

Example: `TZURL:http://zones.example.com/tz/America-New_York.ics`

##### [`ATTENDEE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.1)
This property defines an "Attendee" within a calendar event.

Example: `ATTENDEE;RSVP=TRUE;ROLE=REQ-PARTICIPANT:mailto:person@email.com`

##### [`ORGANIZER` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.2)
This property is used to represent contact information or alternately a reference to contact information associated with the calendar event.

Example: `CONTACT:Jim Dolittle\, ABC Industries\, +1-919-555-1234`

##### [`ORGANIZER` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.3)
This property defines the organizer for a calendar event.

Example: `ORGANIZER;CN="John Smith":mailto:jsmith@example.com`

##### [`URL` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.6)
This property defines a Uniform Resource Locator (URL) associated with the iCalendar object.

Example: `URL:http://example.com/pub/busy/jpublic-01.ifb`

##### [`ACTION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.6.1)
This property defines the action to be invoked when an alarm is triggered.

Example: `ACTION:AUDIO`

##### [`REPEAT` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.6.2)
This property defines the number of times the alarm should be repeated, after the initial trigger.

Example: `REPEAT:4`

##### [`TRIGGER` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.6.3)
This property specifies when an alarm will trigger.

Example: `TRIGGER;RELATED=END:PT5M`

##### [`CREATED` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.7.1)
This property specifies the date and time that the calendar information was created by the calendar user agent in the calendar store.

Example: `CREATED:19960329T133000Z`

##### [`DTSTAMP` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.7.2)
In the case of an iCalendar object that specifies a "METHOD" property, this property specifies the date and time that the instance of the iCalendar object was created.

Example: `DTSTAMP:19970610T172345Z`

##### [`LAST-MODIFIED` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.7.3)
This property specifies the date and time that the information associated with the calendar event was last revised in the calendar store.

Example: `LAST-MODIFIED:20050809T050000Z`

##### [`SEQUENCE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.7.4)
This property defines the revision sequence number of the calendar event within a sequence of revisions.

Example: `SEQUENCE:0`

##### [`REQUEST-STATUS` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.8.3)
This property defines the status code returned for a scheduling request.

Example: `REQUEST-STATUS:2.0;Success`

##### [Non-Standard `X-` prefixed property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.8.2)
Any property name with a "X-" prefix definines any vendor specific non-standard properties.

Examples:
```
X-ABC-MMSUBJ;VALUE=URI;FMTTYPE=audio/basic:http://www.example.org/mysubj.au
X-ONLINE-MEETING-URL;PROVIDER=XYZ:https://xyz.com/meeting/abc123
```

## Return value 

`RDCL.EVO_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each ICalendar property of the created/updated event occurrence override, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Create/update an event stored within a calendar:
```bash
redis> RDCL.EVT_SET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1 DTSTART:20201231T183000Z DTEND:20201231T190000Z RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE GEO:51.454481838260214;-2.588329192623361
1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
2) DTEND:20201231T190000Z
3) DTSTART:20201231T183000Z
4) GEO:51.454481838260214;-2.588329192623361
5) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
6) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
7) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
8) UID:EVENT_IN_BRISTOL_TUE_THU
```

Then override the second occurrence of this event to be located in Oxford UK, with added `RELATED-TO` occurrence relationship thirdparty UID and different `SUMMARY`, `CATEGORIES`, and `DURATION` properties:
```bash
redis> RDCL.EVO_SET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU 20210105T183000Z SUMMARY:Extra long special event in Oxford at 6:30PM DURATION:PT2H30M RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123 RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY_ONE GEO:51.751365550307604;-1.2601196837753945
1) CATEGORIES:CATEGORY_ONE
2) DTSTART:20210105T183000Z
3) DURATION:PT2H30M
4) GEO:51.751365550307604;-1.2601196837753945
5) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
6) RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123
7) SUMMARY:Extra long special event in Oxford at 6:30PM
```

This override is reflected in the second occurrence of the extrapolated event instances for the stored calendar event:
```bash
redis> RDCL.EVI_LIST CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU
1) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
   2) DTEND:20201231T190000Z
   3) DTSTART:20201231T183000Z
   4) DURATION:PT30M
   5) GEO:51.454481838260214;-2.588329192623361
   6) RECURRENCE-ID;VALUE=DATE-TIME:20201231T183000Z
   7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
   8) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
   9) UID:EVENT_IN_BRISTOL_TUE_THU
2)  1) CATEGORIES:CATEGORY_ONE
    2) DTEND:20210105T210000Z
    3) DTSTART:20210105T183000Z
    4) DURATION:PT2H30M
    5) GEO:51.751365550307604;-1.2601196837753945
    6) RECURRENCE-ID;VALUE=DATE-TIME:20210105T183000Z
    7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
    8) RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123
    9) SUMMARY:Extra long special event in Oxford at 6:30PM
   10) UID:EVENT_IN_BRISTOL_TUE_THU
3) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
   2) DTEND:20210107T190000Z
   3) DTSTART:20210107T183000Z
   4) DURATION:PT30M
   5) GEO:51.454481838260214;-2.588329192623361
   6) RECURRENCE-ID;VALUE=DATE-TIME:20210107T183000Z
   7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
   8) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
   9) UID:EVENT_IN_BRISTOL_TUE_THU
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
