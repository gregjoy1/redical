## Overview

### RediCal Commands
* [RDCL.EVT_SET](../commands/rdcl.evt_set.md)
* [RDCL.EVT_GET](../commands/rdcl.evt_get.md)
* [RDCL.EVT_DEL](../commands/rdcl.evt_del.md)
* [RDCL.EVT_LIST](../commands/rdcl.evt_list.md)
* [RDCL.EVI_LIST](../commands/rdcl.evi_list.md)
* [RDCL.EVO_GET](../commands/rdcl.evo_get.md)
* [RDCL.EVO_SET](../commands/rdcl.evo_set.md)
* [RDCL.EVO_DEL](../commands/rdcl.evo_del.md)
* [RDCL.EVO_LIST](../commands/rdcl.evo_list.md)
* [RDCL.CAL_SET](../commands/rdcl.cal_set.md)
* [RDCL.CAL_GET](../commands/rdcl.cal_get.md)
* [RDCL.CAL_QUERY](../commands/rdcl.cal_query.md)
* [RDCL.CAL_IDX_DISABLE](../commands/rdcl.cal_idx_disable.md)
* [RDCL.CAL_IDX_REBUILD](../commands/rdcl.cal_idx_rebuild.md)

### Keyspace notifications

RediCal dispatches various keyspace notifications for all non-read-only commands, key expiries, and evictions available to be monitored via pub/sub.

To enable this, please ensure at least the following keyspace events are defined in the Redis configuration:

> [!NOTE]
> * `K` - Keyspace events, published with `__keyspace@<db>__ prefix`.
> * `e` - Evicted events (events generated when a key is evicted for `maxmemory` Redis configuration)
> * `g` - Generic commands (non-type specific) like DEL, EXPIRE, RENAME, ...
> * `d` - Module (RediCal) key type events

```
notify-keyspace-events Kegd
```

#### `RDCL.CAL_SET` keyspace event

This event is dispatched each time a key containing a RediCal calendar data type is created or updated.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.cal_set"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.cal_set"
```

#### `RDCL.CAL_DEL` keyspace event

This event is dispatched each time a key containing a RediCal calendar data type is:
* Deleted (via `DEL KEY_NAME`)
* Expired (via `EXPIRE KEY_NAME 0`)
* Evicted (via `maxmemory-policy` configuration on Redis exceeding the memory usage defined within the `maxmemory` configuration)

> [!NOTE]
> This keyspace event message is helpful when monitoring the state of calendars, events, and event overrides stored/cached within RediCal and handling the reimport of calendar event data in the event of an eviction.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.cal_del"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.cal_del"
```

#### `RDCL.CAL_IDX_REBUILD` keyspace event

This event is dispatched each time the indexes stored within the RediCal calendar key data type are re-enabled and rebuilt.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.cal_idx_rebuild"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.cal_idx_rebuild"
```

#### `RDCL.CAL_IDX_DISABLE` keyspace event

This event is dispatched each time the indexes stored within the RediCal calendar key data type are disabled.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.cal_idx_disable"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.cal_idx_disable"
```

#### `RDCL.EVT_SET` keyspace event

This keyspace event is dispatched each time a RediCal event contained within a RediCal calendar key data type is updated via the `RDCL.EVT_SET` command.

> [!NOTE]
> This keyspace event message contains the rendered `LAST-MODIFIED` iCalendar property belonging to the event created/updated.
>
> This is helpful when monitoring the version of events stored/cached within RediCal and reconciling those stored within another master data-store.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.evt_set:<EVENT_UID> LAST-MODIFIED:<LAST_MODIFIED_DATE_STRING>"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.evt_set:EVENT_UID LAST-MODIFIED:20210501T090000Z"
```

#### `RDCL.EVT_DEL` keyspace event

This keyspace event is dispatched each time a RediCal event contained within a RediCal calendar key data type is deleted via the `RDCL.EVT_DEL` command.

##### Format:

```
"__keyspace@0__:<KEY_NAME>:<EVENT_UID>", "rdcl.evt_del"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID:EVENT_UID", "rdcl.evt_del"
```

#### `RDCL.EVO_SET` keyspace event

This keyspace event is dispatched each time an occurrence specific override of a RediCal event contained within a RediCal calendar key data type is updated via the `RDCL.EVO_SET` command.

> [!NOTE]
> This keyspace event message contains the rendered `LAST-MODIFIED` iCalendar property belonging to the event occurrence override created/updated.
>
> This is helpful when monitoring the version of occurrence specific event override stored/cached within RediCal and reconciling those stored within another master data-store.

##### Format:

```
"__keyspace@0__:<KEY_NAME>", "rdcl.evo_set:<EVENT_UID>:<OCCURRENCE_DATE_STRING> LAST-MODIFIED:<LAST_MODIFIED_DATE_STRING>"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID", "rdcl.evo_set:EVENT_UID:20210722T143000Z LAST-MODIFIED:20210501T090000Z"
```

#### `RDCL.EVO_DEL` keyspace event

This keyspace event is dispatched each time an occurrence specific override of a RediCal event contained within a RediCal calendar key data type is deleted via the `RDCL.EVO_DEL` command.

##### Format:

```
"__keyspace@0__:<KEY_NAME>:<EVENT_UID>:<OCCURRENCE_DATE_STRING>", "rdcl.evo_del"
```

##### Example:

```
"__keyspace@0__:CALENDAR_UID:EVENT_UID:20210722T143000Z", "rdcl.evo_del"
```
