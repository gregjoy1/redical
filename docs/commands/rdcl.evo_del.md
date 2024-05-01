# RDCL.EVO_DEL

### Syntax
```bash
RDCL.EVO_DEL key event-uid occurrence-date-string
```

Delete the specific occurrence override for an Event stored with the UID: `event_uid` within the Calendar on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

### occurrence-date-string
The date-string of the overridden event occurrence `DTSTART` to return.

## Return value 

`RDCL.EVO_DEL` returns an [integer](https://redis.io/docs/reference/protocol-spec/#integers) representing a boolean, `1` if successful and `0` if not.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Delete an event occurrence override stored within calendar:
```bash
redis> RDCL.EVO_DEL CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU 20210105T183000Z
(integer) 1
```

Delete a non-existent event occurrence override within a calendar:
```bash
redis> RDCL.EVO_DEL CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU 20220105T183000Z
(integer) 0
```

## See also

[`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
