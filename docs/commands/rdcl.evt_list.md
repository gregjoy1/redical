# RDCL.EVT_LIST

### Syntax
```bash
RDCL.EVT_LIST key [offset] [count]
```

Get all Event contained within the Calendar on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

## Optional arguments

### offset
The paged offset for the results returned.

### count
The number of results returned at once (defaulting to 50).

## Return value 

`RDCL.EVT_LIST` returns a nested [array](https://redis.io/docs/reference/protocol-spec/#arrays) of string replies for each event component with each iCalendar property, or `error`, if unsuccessful.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Get first 50 event for an event:
```bash
redis> RDCL.EVT_LIST CALENDAR_UID EVENT_UID
```

Get second 50 event for an event:
```bash
redis> RDCL.EVT_LIST CALENDAR_UID EVENT_UID 49 50
```

Get second 20 event for an event:
```bash
redis> RDCL.EVT_LIST CALENDAR_UID EVENT_UID 19 20
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
