# RDCL.CAL_SET

### Syntax
```bash
RDCL.CAL_SET key
```

Set the Calendar with `UID` for `key`.

## Required arguments

### key
The key to store the calendar (also representing it's UID).

## Return value 

`RDCL.CAL_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#resp-arrays) of string replies for each iCalendar property, or `error`, if the key matching value is not present or not a Calendar.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Create a calendar:
```bash
redis> RDCL.CAL_SET key
```

## See also

[`RDCL.CAL_GET`](rdcl.cal_get.md) | [`RDCL.CAL_IDX_DISABLE`](rdcl.cal_idx_disable.md) | [`RDCL.CAL_IDX_REBUILD`](rdcl.cal_idx_rebuild.md)
