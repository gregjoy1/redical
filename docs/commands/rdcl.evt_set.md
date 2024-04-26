# RDCL.EVT_SET

### Syntax
```bash
RDCL.EVT_DEL key event-uid property [property ...]
```

Description text.

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

##### [DTSTART property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.4) - required
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

##### [DTEND property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.2) - optional
This property specifies the date and time that a calendar component ends.

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

##### [DURATION property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.2.5) - optional
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

##### [RRULE property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.3) - optional
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

##### [RDATE property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.2) - optional
This property defines the list of DATE-TIME values for explicitly defining adhoc recurring events.

###### Examples:

The following are examples of this property:

```
RDATE:19970714T123000Z
RDATE;TZID=America/New_York:19970714T083000

RDATE;VALUE=DATE:19970101,19970120,19970217,19970421,19970526,19970704,19970901,19971014,19971128,19971129,19971225
```

##### [EXDATE property](https://datatracker.ietf.org/doc/html/rfc5545#section-3.8.5.1) - optional
This property defines the list of DATE-TIME exceptions for recurring events

###### Examples:

The following is an example of this property:

```
EXDATE:19960402T010000Z,19960403T010000Z,19960404T010000Z
```

## Return value 

`RDCL.EVT_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each ICalendar event property, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Example text.
```bash
redis> <COMMAND> key
```

## See also

[`COMMAND`](doc.path.md) | [`COMMAND`](doc.path.md)
