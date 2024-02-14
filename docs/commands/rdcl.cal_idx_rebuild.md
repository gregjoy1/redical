# RDCL.CAL_IDX_REBUILD

### Syntax
```bash
RDCL.CAL_IDX_REBUILD key
```

Rebuild all indexes within the Calendar (and all contained Events) on `key`.

This is useful at the tail end of doing a bulk import, as this reduces the operations performed until the end and rebuild more efficiently in one go.

Also helpful if indexes need to be rebuild for what ever reason.

## Required arguments

### key
The key to store the calendar (also representing it's UID).

## Return value 

`RDCL.CAL_IDX_REBUILD` returns a [boolean](https://redis.io/docs/reference/protocol-spec/#booleans) reply indicating success, or `error`, if unsuccessful, the matching key value is not present or not a Calendar.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Create a calendar, disable indexes, bulk import, and rebuild and re-enable the indexes:
```bash
redis> RDCL.CAL_SET key
redis> RDCL.CAL_IDX_DISABLE key

./bulk_import_script.sh|redis-cli --pipe

redis> RDCL.CAL_IDX_REBUILD key
```

Simple re-build a calendar's indexes:
```bash
redis> RDCL.CAL_IDX_REBUILD key
```

## See also

[`RDCL.CAL_GET`](rdcl.cal_get.md) | [`RDCL.CAL_SET`](rdcl.cal_set.md) | [`RDCL.CAL_IDX_DISABLE`](rdcl.cal_idx_disable.md) | [`RDCL.CAL_IDX_DISABLE`](rdcl.cal_idx_disable.md)
