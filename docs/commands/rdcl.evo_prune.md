# RDCL.EVO_PRUNE

### Syntax
```bash
RDCL.EVO_PRUNE key [event-uid] from-date-string until-date-string
```

Delete all event occurrence overrides within the specified date range for all events (or a specific event) contained within the specified calendar.

A typical use case for this command is a daily polling process that cleans up historic event occurrence overrides.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### from-date-string
The date-string representing the lower bound (inclusive) range to prune from.

### until-date-string
The date-string representing the lower bound (inclusive) range to prune until.

## Optional arguments

### event-uid
The UID of the desired event stored within the calendar to prune the occurrence overrides for specifically.

## Return value 

`RDCL.EVO_DEL` returns an [integer](https://redis.io/docs/reference/protocol-spec/#integers) representing the number of pruned event occurrence overrides.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec). 

## Examples

Delete all event occurrence overrides on all events stored within calendar for the whole of 2021:
```bash
redis> RDCL.EVO_DEL CALENDAR_UID 20210101T000000Z 20220101T000000Z
(integer) 1
```

Delete all event occurrence overrides on a specific event stored within calendar for the first week of 2021:
```bash
redis> RDCL.EVO_DEL CALENDAR_UID EVENT_UID 20210101T000000Z 20210107T000000Z
(integer) 1
```

## See also

[`RDCL.EVI_QUERY`](rdcl.evi_query.md) | [`RDCL.EVI_SET`](rdcl.evi_set.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_LIST`](rdcl.evt_list.md) | [`RDCL.EVT_SET`](rdcl.evt_set.md) | [`RDCL.EVT_DEL`](rdcl.evt_del.md) | [`RDCL.EVT_QUERY`](rdcl.evt_query.md) | [`RDCL.EVO_SET`](rdcl.evo_set.md) | [`RDCL.EVO_DEL`](rdcl.evo_del.md) | [`RDCL.EVO_GET`](rdcl.evo_get.md) | [`RDCL.EVO_LIST`](rdcl.evo_list.md)
