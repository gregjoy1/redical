# RDCL.EVO_GET

### Syntax
```bash
RDCL.EVO_GET key event-uid occurrence-date-string
```

Get a specific occurrence override for an Event stored with the UID: `event_uid` within the Calendar on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

### occurrence-date-string
The date-string of the overridden event occurrence `DTSTART` to return.

## Return value 

`RDCL.EVO_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each ICalendar property of the requested event occurrence override, or `nil` if not unsuccessful.

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

Request this specific existing override:
```bash
redis> RDCL.EVO_GET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU 20210105T183000Z
1) CATEGORIES:CATEGORY_ONE
2) DTSTART:20210105T183000Z
3) DURATION:PT2H30M
4) GEO:51.751365550307604;-1.2601196837753945
5) RELATED-TO;RELTYPE=PARENT:PARENT_UUID
6) RELATED-TO;RELTYPE=X-OCCURRENCE:OCCURRENCE_ABC123
7) SUMMARY:Extra long special event in Oxford at 6:30PM
```

Request a non-existent override:
```bash
redis> RDCL.EVO_GET CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU 20220105T183000Z
(nil)
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
