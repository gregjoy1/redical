# RDCL.EVT_PRUNE

### Syntax
```bash
RDCL.EVT_PRUNE key from-date-string until-date-string
```

Delete all events terminating (final occurrence/recurrence) within the specified date range contained within the specified calendar.

A typical use case for this command is a daily polling process that cleans up historic events no longer occurring in the future.

## Required arguments

### key
The key of the stored calendar (also representing it's UID).

### from-date-string
The date-string representing the lower bound (inclusive) range to prune from.

### until-date-string
The date-string representing the lower bound (inclusive) range to prune until.

## Return value

`RDCL.EVT_PRUNE` returns an [integer](https://redis.io/docs/reference/protocol-spec/#integers) representing the number of events pruned.

## Examples

Delete all events terminating in the year 2021:
```bash
redis> RDCL.EVT_PRUNE CALENDAR_UID 20210101T000000Z 20220101T000000Z
(integer) 1
```
