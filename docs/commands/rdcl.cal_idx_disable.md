# RDCL.CAL_IDX_DISABLE

### Syntax
```bash
RDCL.CAL_IDX_DISABLE key
```

Disable and clear all indexes within the Calendar on `key`.

This is useful when doing bulk imports to reduce the operations performed and rebuild the indexes in bulk once finished.

## Required arguments

### key
The key to store the calendar (also representing it's UID).

## Return value 

`RDCL.CAL_IDX_DISABLE` returns a [boolean](https://redis.io/docs/reference/protocol-spec/#booleans) reply indicating success, or `error`, if unsuccessful, the matching key value is not present or not a Calendar.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Create a calendar, disable indexes, bulk import, and rebuild and re-enable the indexes:
```bash
redis> RDCL.CAL_SET key
redis> RDCL.CAL_IDX_DISABLE key

./bulk_import_script.sh|redis-cli --pipe

redis> RDCL.CAL_IDX_REBUILD key
```

## See also

[`RDCL.CAL_GET`](rdcl.cal_get.md) | [`RDCL.CAL_SET`](rdcl.cal_set.md) | [`RDCL.CAL_IDX_DISABLE`](rdcl.cal_idx_disable.md) | [`RDCL.CAL_IDX_REBUILD`](rdcl.cal_idx_rebuild.md)
