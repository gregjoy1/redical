# RDCL.EVT_DEL

### Syntax
```bash
RDCL.EVT_DEL key event-uid
```

Delete the Event with the specified `event-uid` stored within the Calendar stored on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### event-uid
The UID of the desired event stored within the calendar.

## Return value 

`RDCL.EVT_DEL` returns an [integer](https://redis.io/docs/reference/protocol-spec/#integers) representing a boolean, `1` if successful and `0` if not.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Delete an event stored within calendar:
```bash
redis> RDCL.EVT_DEL CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU
(integer) 1
```

Delete a non-existent event within a calendar:
```bash
redis> RDCL.EVT_DEL CALENDAR_UID NON_EXISTENT_UID
(integer) 0
```

## See also

[`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
