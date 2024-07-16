# RDCL.EVT_GET

### Syntax
```bash
RDCL.EVT_GET key event-uid
```

Get the Event with the specified `event-uid` stored within the Calendar stored on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

## Return value 

`RDCL.EVT_GET` returns an [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each ICalendar event property, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Get an event stored within calendar:
```bash
redis> RDCL.EVT_GET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU
1) CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE
2) DTEND:20201231T190000Z
3) DTSTART:20201231T183000Z
4) GEO:51.454481838260214;-2.588329192623361
5) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
6) RRULE:BYDAY=TH,TU;COUNT=3;FREQ=WEEKLY;INTERVAL=1
7) SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM
8) UID:EVENT_IN_BRISTOL_TUE_THU
```

Get a non-existent event within a calendar:
```bash
redis> RDCL.EVT_GET CALENDAR_UID NON_EXISTENT_UID
(nil)
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
