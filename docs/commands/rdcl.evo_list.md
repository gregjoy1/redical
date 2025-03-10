# RDCL.EVO_LIST

### Syntax
```bash
RDCL.EVO_LIST CALENDAR_UID EVENT_UID [offset] [count]
```

Get all occurrence overrides for a specific Event with the UID: `event_uid` within the Calendar on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event_uid
The UID of the event stored within the calendar to extrapolate event instances for.

## Optional arguments

### offset
The paged offset for the results returned.

### count
The number of results returned at once (defaulting to 50).

## Return value 

`RDCL.EVO_LIST` returns a nested [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each occurrence override for each event with each iCalendar property, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Get first 50 event occurrence overides for an event:
```bash
redis> RDCL.EVO_LIST CALENDAR_UID EVENT_UID
```

Get second 50 event occurrence overides for an event:
```bash
redis> RDCL.EVO_LIST CALENDAR_UID EVENT_UID 49 50
```

Get second 20 event occurrence overides for an event:
```bash
redis> RDCL.EVO_LIST CALENDAR_UID EVENT_UID 19 20
```

## See also

[`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_GET`](rdcl.evt_get.md) | [`RDCL.EVT_KEYS`](rdcl.evt_keys.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVI_LIST`](rdcl.evi_list.md)
