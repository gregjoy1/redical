# RDCL.EVT_SET

### Syntax
```bash
RDCL.EVT_SET key event-uid property [property ...]
```

Create (or update if the `event-uid` is already in use) an event on the specified calendar with the provided iCalendar properties.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

### property
The iCalendar ([RFC-5545](https://datatracker.ietf.org/doc/html/rfc5545)) property content lines defining the event to be created with (or updated to reflect).

Whilst RediCal supports all [RFC-5545 component properties](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1) for defining events, how it treats these properties may vary. 

RediCal has the following types of property:

#### Schedule properties

These are used to define when an event should occur, and are instrumental in extrapolating the occurring event instances for the event.

##### [`DTSTART` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.4) - required
This property specifies when the calendar event begins.

###### Examples:

The following is an example of this property defining a UTC date time:

```
DTSTART:19980118T073000Z
```

The following is an example of this property defining a date time in the `Europe/London` timezone:

```
DTSTART;TZ=Europe/London:19980118T073000
```

The following is an example of this property defining a date in the `Europe/London` timezone (starts at midnight):

```
DTSTART;VALUE=DATE;TZ=Europe/London:19980118
```

##### [`DTEND` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.2) - optional
This property specifies the date and time that a calendar event ends.

The `DURATION` of the event is calculated from this property relative to the `DTSTART` property.

If `DTEND` and `DURATION` are not specified, the `DURATION` will be considered 0 and the `DTEND` will match the `DTSTART`.

###### Examples:

The following is an example of this property defining a UTC end date time:

```
DTEND:19980118T073000Z
```

The following is an example of this property defining an end date time in the `Europe/London` timezone:

```
DTEND;TZ=Europe/London:19980118T073000
```

The following is an example of this property defining an end date in the `Europe/London` timezone (at midnight):

```
DTEND;VALUE=DATE;TZ=Europe/London:19980118
```

##### [`DURATION` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.5) - optional
This property specifies a positive duration of time to compute an event instance `DTEND` property from relative to it's `DTSTART`.

This is the relative alternative to defining `DTEND` explicitly.

If `DTEND` and `DURATION` are not specified, the `DURATION` will be considered 0 and the `DTEND` will match the `DTSTART`.

###### Examples:

The following is an example of this property that specifies an interval of time of one hour and zero minutes and zero seconds:

```
DURATION:PT1H0M0S
```

The following is an example of this property that specifies an interval of time of 15 minutes.

```
DURATION:PT15M
```

##### [`RRULE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.3) - optional
This property defines a rule or repeating pattern for recurring events.

###### Examples:

Daily for 10 occurrences:

```
RRULE:FREQ=DAILY;COUNT=10
```

Daily until December 24, 1997:

```
RRULE:FREQ=DAILY;UNTIL=19971224T000000Z
```

Every other day - forever:

```
RRULE:FREQ=DAILY;INTERVAL=2
```

Every 10 days, 5 occurrences:

```
RRULE:FREQ=DAILY;INTERVAL=10;COUNT=5
```

Every day in January, for 3 years:

```
RRULE:FREQ=YEARLY;UNTIL=20000131T140000Z;BYMONTH=1;BYDAY=SU,MO,TU,WE,TH,FR,SA
```

or

```
RRULE:FREQ=DAILY;UNTIL=20000131T140000Z;BYMONTH=1
```

##### EXRULE property - optional
Officially this is deprecated but still supported by RediCal. It is the same as the `RRULE` except the resulting occurrences of this rule are exceptions.

###### Examples:

##### [`RDATE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.2) - optional
This property defines the list of DATE-TIME values for explicitly defining adhoc recurring events.

###### Examples:

The following are examples of this property:

```
RDATE:19970714T123000Z
RDATE;TZID=America/New_York:19970714T083000

RDATE;VALUE=DATE:19970101,19970120,19970217,19970421,19970526,19970704,19970901,19971014,19971128,19971129,19971225
```

##### [`EXDATE` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.1) - optional
This property defines the list of DATE-TIME exceptions for recurring events

###### Examples:

The following is an example of this property:

```
EXDATE:19960402T010000Z,19960403T010000Z,19960404T010000Z
```

#### Indexed properties

These properties are actively indexed, this means that any events (or overrides) using them are able to be queried.

At the time of writing, there are only a handful of indexed properties, but if used carefully these can offer a lot of flexibility.

##### [`CATEGORIES` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.2)
This property defines the categories for a calendar event.

We can use this to specify "tags", for example `ONLINE`, `OFFLINE`, `INSTRUCTOR_ABC123`, `FITNESS`, etc.

Especially since we use an inverted index to index this property, this means that we need to query with an exact tag string (case sensitive).

###### Examples:

The following is an example of this property a list of attributes we want to be able to query:

```
CATEGORIES:ONLINE,FITNESS,INSTRUCTOR_ABC123
```

##### [`RELATED-TO` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.4.5)
This property is used to represent a relationship or reference between one event and another.

This is similar to `CATEGORIES` except the purpose of this property is to define relationships to tags.

We use an inverted index to index this property, this means that we need to query with an exact `RELTYPE` and tag/UID/slug string value (case sensitive).

It is important to note that we index these with the `RELTYPE` parameter concatenated with the value.

For example, `RELATED-TO;RELTYPE=CHILD:SLUG_ABC_123` is indexed as `CHILDSLUG_ABC_123`, and `RELATED-TO;RELTYPE=X-IMAGE:SLUG_ABC_123` is indexed as `X-IMAGESLUG_ABC_123`.

###### Examples:

The following is an example use of this property where we want to define an experimental relationship with an event and a calendar slug/UID from another system:

```
RELATED-TO;RELTYPE=X-CALENDAR:CALENDAR_ABC123
```

This would mean that you can search for all events that are (or overrides that result in) a relationship with the calendar slug/UID.

Another example of this would be if we wanted to represent a relationship with a "parent" entity UID.

```
RELATED-TO;RELTYPE=PARENT:SOME_UID_ABC123
```

The default `RELTYPE` is `PARENT` which means we can omit `RELTYPE` like so:

```
RELATED-TO:SOME_UID_ABC123
```

##### [`CLASS` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.3)
This property defines the access classification for a calendar event.

Essentially, is access to this event public, private, or exclusive.

Similarly to `CATEGORIES`, we use an inverted index to index this property, this means that we need to query with an exact classification string (case sensitive).

###### Examples:

We can use this to specify that an event (or an override) is published:

```
CLASS:PUBLIC
```

We can also use this to specify that an event (or an override) is unpublished:

```
CLASS:PRIVATE
```

Or even define it as neither published or unpublished, but access to it is exclusive:

```
CLASS:CONFIDENTIAL
```

##### [`GEO` property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.1.6)
This property specifies information related to the global position of the event.

Unlike other indexed properties, we index this with a geospatial index.

Essentially, we use this to record the latitude and longitude of the event so that we are able to query for events occurring within a specified radius of a given point, or ordered by distance to a given point. 

###### Examples:

We can use this to define the latitude and longitude of an event (or an override):

```
GEO:37.386013;-122.082932
```

#### Passive properties

These properties are **not** indexed, this means that any events (or overrides) using them **cannot** be queried by them.

If present on an event (or an event override), these are simply blindly and naively "regurgitated" onto each event instance extrapolated from them.

This is useful for properties like `DESCRIPTION`, `LOCATION`, and `IMAGE` that can enrich event instances for re-consumption later for faster access to information instead.

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

## Return value 

`RDCL.EVT_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each ICalendar property of the created/updated, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Create/update a recurring event (every Tuesday and Thursday for three weeks excluding the first Tuesday) stored within a calendar:
```bash
redis> RDCL.EVT_SET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1 EXDATE:20210105T183000Z DTSTART:20201231T183000Z DTEND:20201231T190000Z RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE GEO:51.454481838260214;-2.588329192623361
1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
2) DTEND:20201231T190000Z
3) DTSTART:20201231T183000Z
3) EXDATE:20210105T183000Z
4) GEO:51.454481838260214;-2.588329192623361
5) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
6) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
7) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
8) UID:EVENT_IN_BRISTOL_TUE_THU
```

Create/update an unpublished one off online event stored within a calendar with duration:
```bash
redis> RDCL.EVT_SET CALENDAR_UID ONLINE_EVENT_ON_WED SUMMARY:Online meeting on Wednesday at 6:30PM DTSTART:20210106T183000Z DURATION:PT1H RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY_ONE,ONLINE_EVENT LOCATION:Online meeting - check your email for a link! CLASS:PRIVATE
1) CATEGORIES:CATEGORY_ONE,ONLINE_EVENT
2) CLASS:PRIVATE
3) DTSTART:20210106T183000Z
4) DURATION:PT1H
5) LOCATION:Online meeting - check your email for a link!
6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
7) SUMMARY:Online meeting on Wednesday at 6:30PM
8) UID:ONLINE_EVENT_ON_WED
```

## See also

[`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
