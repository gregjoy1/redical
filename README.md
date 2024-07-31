# RediCal

[![Continuous integration](https://github.com/gregjoy1/redical/actions/workflows/CI.yml/badge.svg)](https://github.com/gregjoy1/redical/actions/workflows/CI.yml)

> [!WARNING]
> This project is experimental and is currently a work in progress and is **not** production ready.

RediCal is a [Redis](https://redis.io/) module that facilitates the storage, extrapolation, and querying of overridable calendar events.

This is achieved with the introduction of a native Redis Calendar data type, of which contains Events, (their associated occurrence property overrides), and indexes for quickly querying all extrapolated event instances for the contained Events.

It is based on the [iCalendar](https://icalendar.org/) standard for how events (their overrides) are defined, parsed, and serialized.

> [!NOTE]
> This project strives to progress closer to the [iCalendar](https://icalendar.org/) over time.

## Build

### From Source

Ensure you have Rust installed: https://www.rust-lang.org/tools/install

Run the following on the main directory:
```bash
cargo build --release
```

To run all unit tests, run:

```bash
cargo test --all
```

To include integration tests, run:

```bash
cargo build && RUST_BACKTRACE=1 cargo test --all integration:: -- --nocapture
```

## Run

### Via Docker
You can build and run RediCal within docker by running the following:
```bash
docker pull gregjoy/redical
docker run -p 6379:6379 -it gregjoy/redical:latest
```

### From Source
Run Redis pointing to the newly built module:

#### Linux
```bash
redis-server --loadmodule ./target/release/libredical.so
```

#### MacOS
```bash
redis-server --loadmodule ./target/release/libredical.dylib
```

Alternatively add the following to a redis.conf file:
```bash
loadmodule /path/to/modules/libredical.so
```

## Getting started

Add a new calendar to the `DEMO_CALENDAR_UID` key:

```bash
redis> RDCL.CAL_SET DEMO_CALENDAR_UID
```

Add a simple recurring event to the newly added calendar:

```bash
redis> RDCL.EVT_SET DEMO_CALENDAR_UID EVENT_IN_OXFORD_MON_WED SUMMARY:Event in Oxford on Mondays and Wednesdays at 5:00PM RRULE:BYDAY=MO,WE;COUNT=4;FREQ=WEEKLY;INTERVAL=1 DTSTART:20201231T170000Z DTEND:20201231T173000Z RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY TWO,CATEGORY_ONE GEO:51.751365550307604;-1.2601196837753945
```

Override the properties for the initial occurrence of the newly added event:

```bash
redis> RDCL.EVO_SET DEMO_CALENDAR_UID EVENT_IN_OXFORD_MON_WED 20210104T170000Z SUMMARY:Overridden event in Oxford summary text RELATED-TO;RELTYPE=PARENT:OVERRIDDEN_PARENT_UUID CATEGORIES:OVERRIDDEN_CATEGORY
```

View all overrides for the newly added event:

```bash
redis> RDCL.EVO_LIST DEMO_CALENDAR_UID EVENT_IN_OXFORD_MON_WED
```

View all extrapolated event instances (including the override) for the newly added event:

```bash
redis> RDCL.EVI_LIST DEMO_CALENDAR_UID EVENT_IN_OXFORD_MON_WED
```

Add an additional event to the calendar:

```bash
redis> RDCL.EVT_SET DEMO_CALENDAR_UID EVENT_IN_BRISTOL_TUE_THU SUMMARY:Event in Bristol on Tuesdays and Thursdays at 6:30PM RRULE:BYDAY=TU,TH;COUNT=3;FREQ=WEEKLY;INTERVAL=1 DTSTART:20201231T183000Z DTEND:20201231T190000Z RELATED-TO;RELTYPE=PARENT:PARENT_UUID CATEGORIES:CATEGORY_FOUR,CATEGORY_ONE GEO:51.454481838260214;-2.588329192623361
```

List all events present in the calendar:

```bash
redis> RDCL.EVT_LIST DEMO_CALENDAR_UID
```

Query all the combined extrapolated event instances for all events in the calendar:

* Empty query -- returns everything
  ```bash
  redis> RDCL.EVI_QUERY DEMO_CALENDAR_UID
  ```

* Empty query -- returns everything ordered by distance to Reading
  ```bash
  redis> RDCL.EVI_QUERY DEMO_CALENDAR_UID X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448
  ```

* Empty query -- returns everything ordered by distance to Reading (grouped by UID)
  ```bash
  redis> RDCL.EVI_QUERY DEMO_CALENDAR_UID X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448 X-DISTINCT:UID
  ```

* Find all events within 60KM of Western-Super-Mare OR with the `OVERRIDDEN_CATEGORY`:
  ```bash
  redis> RDCL.EVI_QUERY DEMO_CALENDAR_UID (X-GEO;DIST=60KM:51.3432622;-3.1608606 OR X-CATEGORIES:OVERRIDDEN_CATEGORY) X-ORDER-BY:GEO-DIST-DTSTART;51.4514278;-1.078448
  ```

## Documentation
For more information, read the following additional documentation:
* [RediCal configuration](docs/docs/configuration.md)
* [RediCal commands and keyspace notifications](docs/docs/commands.md)
* [RediCal technical overview](docs/docs/technical_overview.md).
