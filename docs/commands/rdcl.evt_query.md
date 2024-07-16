# RDCL.EVT_QUERY

### Syntax
```bash
RDCL.EVT_QUERY key event-uid query-property [query-property ...]
```

Query all the events stored in the specified calendar, only concerned with the properties on the events themselves, not the resulting extrapolated event instances comprised of any overrides.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### query-property
Non-standard iCalendar ([RFC-5545](https://datatracker.ietf.org/doc/html/rfc5545)) property content lines specific to RediCal for querying calendar events.

#### `X-FROM` property
This query property defines the lower occurrence `DTSTART`/`DTEND` bounds to query from.

##### Usage:
```
X-FROM[;PROP=(DTSTART|DTEND)][;OP=(GT|GTE)][;TZID=<timezone-id>]:<date-time-string>
```

###### Params:

`PROP` - The event instance occurrence date-time property to query (either `DTSTART` or `DTEND`) - defaults to `DTSTART`.
`OP` - The query operator (either `GT` or `GTE`) - defaults to `GT`.
`TZID` - The timezone of the date-string provided - defaults to `UTC`.

##### Examples:

All events specified with a `DTSTART` property starting after `19960401T150000Z`:
```
X-FROM:19960401T150000Z
```

All events specified with a `DTSTART` and either `DTEND` or `DURATION` properties resulting in them ending after (or at) `19960401T150000` in `Europe/London` timezone:
```
X-FROM;PROP=DTEND;OP=GTE;TZID=Europe/London:19960401T150000
```

#### `X-UNTIL` property
This query property defines the upper occurrence `DTSTART`/`DTEND` bounds to query until.

##### Usage:
```
X-UNTIL[;PROP=(DTSTART|DTEND)][;OP=(LT|LTE)][;TZID=<timezone-id>]:<date-time-string>
```

###### Params:

`PROP` - The event instance occurrence date-time property to query (either `DTSTART` or `DTEND`) - defaults to `DTSTART`.
`OP` - The query operator (either `LT` or `LTE`) - defaults to `LT`.
`TZID` - The timezone of the date-string provided - defaults to `UTC`.

##### Examples:

All events specified with a `DTSTART` property starting before `19960401T150000Z`:
```
X-UNTIL:19960401T150000Z
```

All events specified with a `DTSTART` and either `DTEND` or `DURATION` properties resulting in them ending before (or at) `19960401T150000` in `Europe/London` timezone:
```
X-UNTIL;PROP=DTEND;OP=GTE;TZID=Europe/London:19960401T150000
```

#### `X-LIMIT` property
This query property limits the number of query results to a specified amount.

##### Usage:
```
X-LIMIT:<number-of-results>
```

##### Example:

Limit query to 50 results:
```
X-LIMIT:50
```

#### `X-OFFSET` property
This property defines the desired query result offset. This can be used alongside `X-LIMIT` to achieve pagination.

##### Usage:
```
X-OFFSET:<number-of-results>
```

##### Example:

Offset the query to the fourth page of 50 query results:
```
X-LIMIT:50
X-OFFSET:150
```

#### `X-ORDER` property
This query property defines the order of the query results.

##### Usage:
```
X-ORDER-BY:(DTSTART|DTSTART-GEO-DIST;<latitude>;<longitude>|GEO-DIST-DTSTART;<latitude>;<longitude>)
```

###### Values:

`DTSTART` - Order events by `DTSTART` ascending.
`DTSTART-GEO-DIST` - Order events by `DTSTART` ascending first, falling back to distance from provided latitude and longitude.
`GEO-DIST-DTSTART` - Order events by distance to provided latitude and longitude ascending, falling back to `DTSTART`.

##### Examples:

Order by `DTSTART` (ascending):
```
X-ORDER-BY:DTSTART
```

Order by `DTSTART`, falling back to distance from latitude: `48.85299` and longitude: `2.36885` (ascending):
```
X-ORDER-BY:DTSTART-GEO-DIST;48.85299;2.36885
```

Order by distance from latitude: `48.85299` and longitude: `2.36885`, falling back to `DTSTART` (ascending):
```
X-ORDER-BY:GEO-DIST-DTSTART;48.85299;2.36885
```

#### `X-TZID` property
This property defines the desired timezone the results of the query should be returned in.

##### Usage:
```
X-TZID:<timezone-id>
```

If not specified, defaults to `UTC`.

##### Example:

Return results in `UTC` timezone:
```
X-TZID:UTC
```

Return results in `Europe/London` timezone:
```
X-TZID:Europe/London
```

#### `X-CATEGORIES` property
This property defines the `CATEGORIES` values on each event instance to query. This can be specified multiple times and outside of a where query group all properties default to the `AND` operator.

##### Usage:
```
X-CATEGORIES[;OP=(AND|OR)]:<categories>[,<categories>...]
```

###### Params:

`OP` - The query operator (either `AND` or `OR`) - defaults to `AND`.

##### Example:

Query all events with both `APPOINTMENT`, **and** `EDUCATION` `CATEGORIES` values:
```
X-CATEGORIES:APPOINTMENT,EDUCATION
```

Equivilent to:
```
X-CATEGORIES;OP=AND:APPOINTMENT,EDUCATION
```

Query all events with either `APPOINTMENT` **or** `EDUCATION` `CATEGORIES` values:
```
X-CATEGORIES;OP=OR:APPOINTMENT,EDUCATION
```

Query all events with `MEETING` `CATEGORIES` values **and** either `APPOINTMENT` **or** `EDUCATION` `CATEGORIES` values:
```
X-CATEGORIES:MEETING X-CATEGORIES;OP=OR:APPOINTMENT,EDUCATION
```

#### `X-UID` property
This property defines the `UID` values on each event instance to query. This can be specified multiple times and outside of a where query group all properties will be queried with the `OR` operator (an event cannot have multiple UIDs defined which precludes the use of the `AND` operator).

##### Usage:
```
X-UID:<uids>[,<uids>...]
```

##### Example:

Query all events with `UID_ONE` `UID` value:
```
X-UID:UID_ONE
```

Query all events with either `UID_ONE`, **or** `UID_TWO` `UID` values:
```
X-UID:UID_ONE,UID_TWO
```

#### `X-LOCATION-TYPE` property
This property defines the `LOCATION-TYPE` values on each event instance to query. This can be specified multiple times and outside of a where query group all properties default to the `AND` operator.

##### Usage:
```
X-LOCATION-TYPE[;OP=(AND|OR)]:<types>[,<types>...]
```

###### Params:

`OP` - The query operator (either `AND` or `OR`) - defaults to `AND`.

##### Example:

Query all events with both `ONLINE`, **and** `ZOOM` `LOCATION-TYPE` values:
```
X-LOCATION-TYPE:ONLINE,ZOOM
```

Equivilent to:
```
X-LOCATION-TYPE;OP=AND:ONLINE,ZOOM
```

Query all events with either `ONLINE` **or** `OFFLINE` `LOCATION-TYPE` values:
```
X-LOCATION-TYPE;OP=OR:ONLINE,OFFLINE
```

Query all events with `ONLINE` `LOCATION-TYPE` values **and** either `ZOOM` **or** `HANGOUTS` `LOCATION-TYPE` values:
```
X-LOCATION-TYPE:ONLINE X-LOCATION-TYPE;OP=OR:ZOOM,HANGOUTS
```

#### `X-RELATED` property
This property defines the `RELATED-TO` values on each event instance to query. This can be specified multiple times and outside of a where query group all properties default to the `AND` operator.

##### Usage:
```
X-RELATED-TO[;OP=(AND|OR)]:<related-to-uid>[,<related-to-uid>...]
```

###### Params:

`OP` - The query operator (either `AND` or `OR`) - defaults to `AND`.

##### Example:

Query all events with both `parent.uid.one`, **and** `parent.uid.two` `RELATED-TO` values:
```
X-RELATED-TO:parent.uid.one,parent.uid.two
```

Equivilent to:
```
X-RELATED-TO;OP=AND:parent.uid.one,parent.uid.two
```

Query all  events with `RELATED-TO` properties containing the `X-RELTYPE` `RELTYPE` and either `x-reltype.uid.one` **or** `x-reltype.uid.two` values:
```
X-RELATED-TO;RELTYPE=X-RELTYPE;OP=OR:x-reltype.uid.one,x-reltype.uid.two
```

Query all  events with `RELATED-TO` properties containing the `PARENT` and `X-RELTYPE` `RELTYPE` parameters with `parent.uid` and either `x-reltype.uid.one` **or** `x-reltype.uid.two` values:
```
X-RELATED-TO:parent.uid X-RELATED-TO;RELTYPE=X-RELTYPE;OP=OR:x-reltype.uid.one,x-reltype.uid.two
```

#### `X-CLASS` property
This property defines the `CLASS` values on each event instance to query. This can be specified multiple times and outside of a where query group all properties default to the `AND` operator.

##### Usage:
```
X-CLASS[;OP=(AND|OR)]:<class>[,<class>...]
```

###### Params:

`OP` - The query operator (either `AND` or `OR`) - defaults to `AND`.

##### Example:

Query all  events with both `PUBLIC`, **and** `PRIVATE` `CLASS` values:
```
X-CLASS:PUBLIC,PRIVATE
```

Equivilent to:
```
X-CLASS;OP=AND:PUBLIC,PRIVATE
```

Query all  events with either `PUBLIC` **or** `PRIVATE` `CLASS` values:
```
X-CLASS;OP=OR:PUBLIC,PRIVATE
```

#### `X-GEO` property
This property filters the  events returned to those with `GEO` properties defined to be within the distance specified from the point specified.

##### Usage:
```
X-GEO[;DIST=<distance>(KM|MI)]:<latitude>;<longitude>
```

###### Params:

`DIST` - The distance to restrict the  events to - defaults to `10KM`

##### Example:

Restrict  events to 10 kilometers from latitude: `48.85299` and longitude: `2.36885`:
```
X-GEO:48.85299;2.36885
```

Restrict  events to 15 kilometers from latitude: `48.85299` and longitude: `2.36885`:
```
X-GEO;DIST=1.5KM:48.85299;2.36885
```

Restrict  events to 30 miles from latitude: `48.85299` and longitude: `2.36885`:
```
X-GEO;DIST=30MI:48.85299;2.36885
```

#### Where group group
This allows the following properties to be grouped into sub-queries which can be delimited by either `AND`/`OR` operators:
* `X-CATEGORIES`
* `X-UID`
* `X-LOCATION-TYPE`
* `X-RELATED-TO`
* `X-CLASS`
* `X-GEO`

##### Usage:
```
([(X-CATEGORIES...|X-UID...|X-LOCATION-TYPE...|X-RELATED-TO...|X-CLASS...|X-GEO...)] [[(AND|OR)] [(X-CATEGORIES...|X-UID...|X-LOCATION-TYPE...|X-RELATED-TO...|X-CLASS...|X-GEO...])] ...)
```

##### Example:

Restrict  events to those with matching `CATEGORIES` `Categories text`:
```
() X-CATEGORIES:Categories text
```

Restrict  events to those with either `PUBLIC` or `PRIVATE` `CLASS` property defined **and** with matching `CATEGORIES` `Categories text`:
```
(X-CLASS;OP=OR:PUBLIC,PRIVATE) X-CATEGORIES:Categories text
```

Restrict  events to those with matching `CATEGORIES` `Categories text` ***and** with either `PUBLIC` or `PRIVATE` `CLASS` property defined **or** with matching `CATEGORIES` `APPOINTMENT` and `EDUCATION` values:
```
(X-CLASS:PUBLIC,PRIVATE OR X-CATEGORIES:APPOINTMENT,EDUCATION) X-CATEGORIES:Categories text
```

Restrict  events to those with `PUBLIC` `CLASS` property defined **and** with matching `APPOINTMENT` `CATEGORIES` **and** with `PRIVATE` `CLASS` **and** with matching `EDUCATION` and `Categories text` `CATEGORIES`:
```
(X-CLASS:PUBLIC X-CATEGORIES:APPOINTMENT (X-CLASS:PRIVATE X-CATEGORIES:EDUCATION)) X-CATEGORIES:Categories text
```

Restrict  events to those with `PUBLIC` `CLASS` **or** `APPOINTMENT` `CATEGORIES` **and** with `PRIVATE` `CLASS` **and** with matching `EDUCATION` and `Categories text` `CATEGORIES`:
```
(X-CLASS:PUBLIC OR X-CATEGORIES:APPOINTMENT AND (X-CLASS:PRIVATE OR X-CATEGORIES:EDUCATION)) X-CATEGORIES:Categories text
```

## Return value 

`RDCL.EVT_QUERY` returns a multi dimensional [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each event instance returned by the query.

This is comprised of two nested arrays:
* The first highlights the utilised ordering attributes of the event instance
* The second contains each ICalendar property of the event instance

```bash
1) 1) 1) DTSTART:20210104T170000Z
      2) X-GEO-DIST:35.633761KM
   2) 1) CATEGORIES:OVERRIDDEN_CATEGORY
      2) DTEND:20210104T173000Z
      3) DTSTART:20210104T170000Z
      4) DURATION:PT30M
      5) GEO:51.751365550307604;-1.2601196837753945
      6) RECURRENCE-ID;VALUE=DATE-TIME:20210104T170000Z
      7) RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID
      8) SUMMARY:Overridden event in Oxford summary text
      9) UID:EVENT_IN_OXFORD_MON_WED
2) 1) 1) ...
      2) ...
   2) 1) ...
      2) ...
      ...
```

If unsuccessful, it simply returns an `error` response.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Empty query -- returns everything
```bash
redis> RDCL.EVT_QUERY CALENDAR_UID
1) 1) 1) DTSTART:20201231T170000Z
      2) X-GEO-DIST:35.633761KM
   2)  1) CATEGORIES:CATEGORY TWO,CATEGORY_ONE
       2) DTEND:20201231T173000Z
       3) DTSTART:20201231T170000Z
       4) EXDATE:20210111T170000Z
       5) GEO:51.751365550307604;-1.2601196837753945
       6) LAST-MODIFIED:20240605T192805Z
       7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
       8) RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1
       9) SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM
      10) UID:EVENT_IN_OXFORD_MON_WED
2) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:85.055714KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.89936851432488;-2.078357552295971
      5) LAST-MODIFIED:20240605T192805Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_CHELTENHAM_TUE_THU
3) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:104.621379KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.454481838260214;-2.588329192623361
      5) LAST-MODIFIED:20240621T203512Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_BRISTOL_TUE_THU
...
```

Empty query -- returns everything ordered by distance to Reading
```bash
redis> RDCL.EVT_QUERY CALENDAR_UID X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448
1) 1) 1) DTSTART:20201231T170000Z
      2) X-GEO-DIST:35.633761KM
   2)  1) CATEGORIES:CATEGORY TWO,CATEGORY_ONE
       2) DTEND:20201231T173000Z
       3) DTSTART:20201231T170000Z
       4) EXDATE:20210111T170000Z
       5) GEO:51.751365550307604;-1.2601196837753945
       6) LAST-MODIFIED:20240605T192805Z
       7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
       8) RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1
       9) SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM
      10) UID:EVENT_IN_OXFORD_MON_WED
2) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:85.055714KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.89936851432488;-2.078357552295971
      5) LAST-MODIFIED:20240605T192805Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_CHELTENHAM_TUE_THU
3) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:104.621379KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.454481838260214;-2.588329192623361
      5) LAST-MODIFIED:20240621T203512Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_BRISTOL_TUE_THU
...
```

Empty query -- returns everything ordered by distance to Reading (grouped by UID) limited to 2 results
```bash
redis> RDCL.EVT_QUERY CALENDAR_UID X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448 X-LIMIT:2
1) 1) 1) DTSTART:20201231T170000Z
      2) X-GEO-DIST:35.633761KM
   2)  1) CATEGORIES:CATEGORY TWO,CATEGORY_ONE
       2) DTEND:20201231T173000Z
       3) DTSTART:20201231T170000Z
       4) EXDATE:20210111T170000Z
       5) GEO:51.751365550307604;-1.2601196837753945
       6) LAST-MODIFIED:20240605T192805Z
       7) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
       8) RRULE:BYDAY=MO,WE;COUNT=3;FREQ=WEEKLY;INTERVAL=1
       9) SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM
      10) UID:EVENT_IN_OXFORD_MON_WED
2) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:85.055714KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.89936851432488;-2.078357552295971
      5) LAST-MODIFIED:20240605T192805Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Cheltenham on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_CHELTENHAM_TUE_THU
```

Find all events with the `PARENT` relation to `PARENT_UID` that are within 60KM of Western-Super-Mare OR with the `OVERRIDDEN_CATEGORY` limited to 2 results:
```bash
redis> RDCL.EVT_QUERY CALENDAR_UID (X-GEO;DIST=60KM:51.3432622;-3.1608606 OR X-CATEGORIES:OVERRIDDEN_CATEGORY) X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448 X-RELATED-TO;RELTYPE=PARENT:PARENT_UUID X-LIMIT:2
1) 1) 1) DTSTART:20201231T183000Z
      2) X-GEO-DIST:104.621379KM
   2) 1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
      2) DTEND:20201231T190000Z
      3) DTSTART:20201231T183000Z
      4) GEO:51.454481838260214;-2.588329192623361
      5) LAST-MODIFIED:20240621T203512Z
      6) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
      7) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
      8) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
      9) UID:EVENT_IN_BRISTOL_TUE_THU
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
