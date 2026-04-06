# Architecture

**Analysis Date:** 2026-03-06

## Pattern Overview

**Overall:** Layered Rust workspace — iCalendar parsing layer, domain logic layer, Redis module integration layer

**Key Characteristics:**
- Three crates with strict dependency direction: `redical_ical` ← `redical_core` ← `redical_redis`
- `redical_redis` compiles to a `cdylib` loaded into Redis as a native module
- Domain objects (`Calendar`, `Event`) are stored directly in Redis memory via custom `RedisType`; no separate serialization at query time
- iCal text is the wire format for both input (commands) and output (responses)
- Calendar indexes (inverted + geospatial) are maintained incrementally on write, or rebuilt in bulk via `rdcl.cal_idx_rebuild`

## Layers

**iCalendar Parsing (`redical_ical`):**
- Purpose: Parse and render RFC 5545 iCalendar text; no domain logic
- Location: `redical_ical/src/`
- Contains: Grammar combinators (`grammar.rs`), `ContentLine` type, property types (`properties/`), value types (`values/`), `ICalendarEntity` and `ICalendarComponent` traits, `ParserInput`/`ParserResult` type aliases
- Depends on: `nom`, `nom_locate`, `chrono`, `chrono-tz`, `itertools`
- Used by: `redical_core`, `redical_redis`

**Domain Core (`redical_core`):**
- Purpose: Calendar/Event domain model, index structures, occurrence iteration, query execution
- Location: `redical_core/src/`
- Contains: `Calendar`, `Event`, `EventOccurrenceOverride`, `EventInstance`, `EventOccurrenceIterator`, `InvertedCalendarIndex`/`InvertedEventIndex`, `GeoSpatialCalendarIndex`, query subsystem (`queries/`)
- Depends on: `redical_ical`, `rrule`, `rstar`, `geo`, `geohash`, `chrono`, `chrono-tz`
- Used by: `redical_redis`

**Redis Module (`redical_redis`):**
- Purpose: Expose `rdcl.*` Redis commands; own the Redis data type lifecycle (RDB persistence, copy, free)
- Location: `redical_redis/src/`
- Contains: Command handlers (`commands/`), `CALENDAR_DATA_TYPE` (`datatype/`), RDB serialization via `bincode` (`datatype/rdb_data.rs`), `run_with_timeout` utility, allocator shim
- Depends on: `redical_core`, `redical_ical`, `redis-module`, `redis-module-macros`, `bincode`, `rayon`, `libc`
- Used by: Redis server at runtime (loaded as `.so`)

## Data Flow

**Write command (`rdcl.evt_set`):**

1. Redis routes command to `redical_redis/src/commands/rdcl_evt_set.rs`
2. Command handler parses iCal text via `Event::parse_ical` — run in a timeout-guarded thread (`run_with_timeout`) using the configurable `ical-parser-timeout-ms` limit
3. Parsed `Event` is validated
4. LAST-MODIFIED guard: skip update if incoming event is older than stored event
5. `event.rebuild_indexes()` populates per-event inverted index terms
6. `CalendarIndexUpdater` diffs old vs new index terms and applies incremental updates to calendar-level indexes
7. `calendar.insert_event(event)` stores the domain object in Redis key memory
8. `ctx.replicate_verbatim()` replicates the raw command to replicas
9. Keyspace event published via `ctx.notify_keyspace_event`
10. Serialized iCal lines returned to caller as `RedisValue::Array`

**Query command (`rdcl.evi_query` / `rdcl.evt_query`):**

1. Redis routes to command handler in `redical_redis/src/commands/`
2. Handler opens Redis key read-only, retrieves `&Calendar` from `CALENDAR_DATA_TYPE`
3. Query string parsed by `QueryParser` (in `redical_core/src/queries/query_parser.rs`) into a `Query` struct with `WhereConditional`, ordering, range bounds, limit/offset, timezone
4. `query.execute(&calendar)` runs: searches calendar-level inverted/geo indexes to narrow event set, then iterates occurrences via `EventOccurrenceIterator` (backed by `rrule`) merging `EventOccurrenceOverride` data
5. Results returned as iCal-serialized content lines

**RDB Persistence:**

1. On `rdb_save`: `Calendar` → `RDBCalendar` (via `TryFrom`) → `bincode::serialize` → saved as Redis string
2. On `rdb_load`: bytes → `bincode::deserialize` → `RDBCalendar` → `Calendar::try_from` (parallel parse via `rayon`)

**State Management:**
- All calendar state lives in Redis key memory as heap-allocated `Calendar` structs owned by the Redis module type system
- No external database; Redis RDB/AOF provides persistence
- Indexes are in-process data structures inside `Calendar` (`InvertedCalendarIndex`, `GeoSpatialCalendarIndex`)

## Key Abstractions

**`ICalendarEntity` trait:**
- Purpose: Defines `parse_ical(input) -> ParserResult<Self>` and `render_ical() -> String` — the parse/render contract for all iCal value and property types
- Examples: all types in `redical_ical/src/values/`, `redical_ical/src/properties/`
- Pattern: Implemented on concrete structs; `impl_icalendar_entity_traits!` macro derives `FromStr` and `Display`

**`ICalendarComponent` trait:**
- Purpose: Render a composite object (Calendar, Event, EventOccurrenceOverride) as a `BTreeSet<ContentLine>`
- Examples: `redical_core/src/calendar.rs`, `redical_core/src/event.rs`
- Pattern: `to_content_line_set_with_context(context)` with optional `RenderingContext` for timezone/unit conversion

**`Calendar` struct:**
- Purpose: Root domain aggregate; owns events and all indexes
- Location: `redical_core/src/calendar.rs`
- Pattern: `BTreeMap<String, Box<Event>>` for events; separate `InvertedCalendarIndex<T>` fields per indexed property (categories, location_type, related_to, class) plus `GeoSpatialCalendarIndex`

**`InvertedCalendarIndex` / `InvertedEventIndex`:**
- Purpose: Per-property inverted indexes supporting AND/OR/NOT query operations with occurrence-level exceptions
- Location: `redical_core/src/inverted_index.rs`
- Pattern: `IndexedConclusion::Include(exceptions)` / `IndexedConclusion::Exclude(exceptions)` — exceptions are sets of occurrence timestamps that flip the conclusion for specific recurrence instances

**`GeoSpatialCalendarIndex`:**
- Purpose: R-tree spatial index for geo-distance queries
- Location: `redical_core/src/geo_index.rs`
- Pattern: Backed by `rstar::RTree`; stores `GeomWithData<Point, (event_uid, IndexedConclusion)>`

**`Query` trait:**
- Purpose: Polymorphic query execution over `Calendar`; implemented by `EventQuery` and `EventInstanceQuery`
- Location: `redical_core/src/queries/query.rs`
- Pattern: `execute(&Calendar) -> QueryResults<T>`; parsed from iCal-like query text by `QueryParser`

**`CALENDAR_DATA_TYPE`:**
- Purpose: Redis native type registration; owns `rdb_load`, `rdb_save`, `free`, `copy` C-ABI hooks
- Location: `redical_redis/src/datatype/mod.rs`
- Pattern: `RedisType::new(...)` static; `RDBCalendar` intermediate serde struct for bincode persistence

## Entry Points

**Redis module init:**
- Location: `redical_redis/src/lib.rs`
- Triggers: Redis loads `.so` via `MODULE LOAD`
- Responsibilities: Registers all `rdcl.*` commands, `CALENDAR_DATA_TYPE`, keyspace event handlers, and `ical-parser-timeout-ms` config

**Command handlers:**
- Location: `redical_redis/src/commands/rdcl_*.rs`
- Triggers: Client issues `rdcl.*` Redis command
- Responsibilities: Argument parsing, key access, delegation to `redical_core`, iCal serialization of response, keyspace notification, replication

**`Event::parse_ical`:**
- Location: `redical_core/src/event.rs`
- Triggers: Called by write command handlers inside `run_with_timeout`
- Responsibilities: Orchestrates property-by-property nom parsing of iCal event text

## Error Handling

**Strategy:** `Result<T, String>` throughout `redical_core` and `redical_ical`; command handlers map `String` errors to `RedisError::String`; hard parse failures use `nom::Err::Failure` (non-recoverable), soft failures use `nom::Err::Error` (recoverable/backtracking)

**Patterns:**
- `ParserError` carries span, message, and context chain for descriptive error messages
- `convert_error` renders parser errors as single-line strings (Redis-friendly)
- `map_err` / `map_err_message!` macro for enriching recoverable nom errors
- Timeout enforced by `run_with_timeout` returning `TimeoutError`; logged as warning and surfaced as `RedisError::String`

## Cross-Cutting Concerns

**Logging:** `ctx.log_debug(...)` and `ctx.log_warning(...)` via `redis_module::Context`; only available inside command handlers in `redical_redis`

**Validation:** `ICalendarEntity::validate()` on parsed values/properties; `Event::validate()` called before insert in write commands

**Authentication:** Delegated entirely to Redis (no application-level auth)

---

*Architecture analysis: 2026-03-06*
