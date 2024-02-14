# RDCL.CAL_GET

### Syntax
```bash
RDCL.CAL_GET key
```

Get the Calendar on `key`.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

## Return value 

`RDCL.CAL_SET` returns an [array](https://redis.io/docs/reference/protocol-spec/#resp-arrays) of string replies for each iCalendar property, or `error`, if the matching key value is not present or not a Calendar.

For more information about replies, see [Redis serialization protocol specification](https://redis.io/docs/reference/protocol-spec).

## Examples

Get a calendarL
```bash
redis> RDCL.CAL_GET key
```

## See also

[`RDCL.CAL_SET`](rdcl.cal_set.md) | [`RDCL.CAL_IDX_DISABLE`](rdcl.cal_idx_disable.md) | [`RDCL.CAL_IDX_REBUILD`](rdcl.cal_idx_rebuild.md)
