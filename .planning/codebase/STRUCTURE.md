# Codebase Structure

**Analysis Date:** 2026-03-06

## Directory Layout

```
redical/                              # Workspace root
├── Cargo.toml                        # Workspace manifest + shared dependencies
├── Cargo.lock
├── Makefile                          # Build / test / run targets
├── Dockerfile                        # Container build
├── rustfmt.toml                      # Rust formatting config
├── ramp.yml                          # Release automation config
│
├── redical_ical/                     # iCalendar parsing crate (no domain logic)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                    # Public API: traits, parser types, macros, helpers
│       ├── grammar.rs                # Low-level nom combinators (wsp, contentline, etc.)
│       ├── content_line.rs           # ContentLine + ContentLineParams types
│       ├── properties/
│       │   ├── mod.rs                # ICalendarProperty, ICalendarGeoProperty, ICalendarDateTimeProperty traits
│       │   ├── uid.rs
│       │   ├── recurrence_id.rs
│       │   ├── last_modified.rs
│       │   ├── event/               # Event-specific property types (RRULE, DTSTART, GEO, etc.)
│       │   ├── calendar/            # Calendar-level property types (UID)
│       │   └── query/               # Query property types (WHERE, ORDER, RANGE, etc.)
│       └── values/
│           ├── mod.rs
│           ├── date_time.rs
│           ├── date.rs
│           ├── duration.rs
│           ├── recur.rs
│           ├── text.rs
│           ├── list.rs
│           ├── integer.rs
│           ├── float.rs
│           ├── tzid.rs
│           ├── reltype.rs
│           ├── where_operator.rs
│           ├── where_range_operator.rs
│           └── where_range_property.rs
│
├── redical_core/                     # Domain model + query engine crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                    # Re-exports all public items
│       ├── calendar.rs               # Calendar struct + CalendarIndexUpdater
│       ├── event.rs                  # Event struct + parse_ical + rebuild_indexes
│       ├── event_diff.rs             # Diff logic between event versions
│       ├── event_instance.rs         # EventInstance (materialised occurrence)
│       ├── event_occurrence_iterator.rs  # Iterator over recurrence occurrences (wraps rrule)
│       ├── event_occurrence_override.rs  # EventOccurrenceOverride (per-occurrence patch)
│       ├── inverted_index.rs         # InvertedCalendarIndex + InvertedEventIndex + IndexedConclusion
│       ├── geo_index.rs              # GeoSpatialCalendarIndex (rstar R-tree)
│       ├── utils.rs                  # KeyValuePair, UpdatedHashMapMembers, UpdatedSetMembers
│       ├── queries/
│       │   ├── mod.rs
│       │   ├── query.rs              # Query + QueryIndexAccessor traits
│       │   ├── event_query.rs        # EventQuery implementation
│       │   ├── event_instance_query.rs  # EventInstanceQuery implementation
│       │   ├── query_parser.rs       # QueryParser (iCal-like query text → Query struct)
│       │   ├── indexed_property_filters.rs  # WhereConditional, WhereOperator, filter types
│       │   ├── results.rs            # QueryResults, QueryableEntity trait
│       │   ├── results_ordering.rs   # OrderingCondition, QueryResultOrdering
│       │   └── results_range_bounds.rs  # LowerBoundRangeCondition, UpperBoundRangeCondition
│       └── testing/                  # Test helpers (gated behind #[cfg(test)])
│
├── redical_redis/                    # Redis module crate (cdylib entry point)
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs                    # redis_module! macro, command registration, event handlers, config
│       ├── utils.rs                  # run_with_timeout helper
│       ├── datatype/
│       │   ├── mod.rs                # CALENDAR_DATA_TYPE RedisType + RDB hooks
│       │   └── rdb_data.rs           # RDBCalendar/RDBEvent/RDBEventOccurrenceOverride (serde/bincode)
│       └── commands/
│           ├── mod.rs                # Re-exports all command functions
│           ├── rdcl_cal_set.rs       # rdcl.cal_set
│           ├── rdcl_cal_get.rs       # rdcl.cal_get
│           ├── rdcl_cal_idx_disable.rs  # rdcl.cal_idx_disable
│           ├── rdcl_cal_idx_rebuild.rs  # rdcl.cal_idx_rebuild
│           ├── rdcl_evt_set.rs       # rdcl.evt_set
│           ├── rdcl_evt_get.rs       # rdcl.evt_get
│           ├── rdcl_evt_del.rs       # rdcl.evt_del
│           ├── rdcl_evt_list.rs      # rdcl.evt_list
│           ├── rdcl_evt_query.rs     # rdcl.evt_query
│           ├── rdcl_evt_prune.rs     # rdcl.evt_prune
│           ├── rdcl_evi_list.rs      # rdcl.evi_list
│           ├── rdcl_evi_query.rs     # rdcl.evi_query
│           ├── rdcl_evo_get.rs       # rdcl.evo_get
│           ├── rdcl_evo_set.rs       # rdcl.evo_set
│           ├── rdcl_evo_del.rs       # rdcl.evo_del
│           ├── rdcl_evo_list.rs      # rdcl.evo_list
│           └── rdcl_evo_prune.rs     # rdcl.evo_prune
│
├── redical_ical_afl_fuzz_targets/    # AFL fuzz testing harness (not in default workspace members)
│   ├── Cargo.toml
│   ├── input_seeds/
│   └── src/bin/
│
├── tests/                            # Integration tests (run against live Redis instance)
│   ├── integration.rs                # Test functions exercising rdcl.* commands end-to-end
│   ├── macros.rs                     # Test macros (set_and_assert_calendar!, assert_keyspace_events_published!, etc.)
│   ├── utils.rs                      # Redis connection helpers, listen_for_keyspace_events
│   └── redis_test_config.conf        # Redis config for test instance
│
└── docs/                             # Documentation
    ├── commands/                     # Per-command documentation
    └── docs/                         # General documentation
```

## Directory Purposes

**`redical_ical/src/properties/event/`:**
- Purpose: One file per event iCal property (RRULE, DTSTART, DTEND, GEO, CATEGORIES, CLASS, RELATED-TO, LOCATION-TYPE, etc.)
- Key files: each property struct implements `ICalendarEntity` (parse + render) and `ICalendarProperty` (to ContentLine)

**`redical_ical/src/properties/query/`:**
- Purpose: Query-DSL property types — WHERE clauses, ORDER BY, RANGE bounds, LIMIT, OFFSET, TZID, DISTINCT
- Pattern: Same parse/render traits as event properties

**`redical_core/src/queries/`:**
- Purpose: Complete query subsystem — parsing query text, filtering via indexes, iterating occurrences, ordering and paginating results
- Key files: `query_parser.rs` (text → struct), `event_instance_query.rs` (the main occurrence query), `indexed_property_filters.rs` (WHERE tree evaluation)

**`redical_redis/src/commands/`:**
- Purpose: One file per Redis command; command names map directly to file names (e.g. `rdcl.evt_set` → `rdcl_evt_set.rs`)

**`redical_redis/src/datatype/`:**
- Purpose: Redis native type definition and RDB (persistence) serialization/deserialization
- `rdb_data.rs` defines intermediate `RDB*` structs with `serde` derive; iCal text is the interchange format between `RDBCalendar` and domain structs during load

**`tests/`:**
- Purpose: End-to-end integration tests; require a running Redis with the module loaded
- Key files: `integration.rs` (test scenarios), `macros.rs` (assertion macros over Redis protocol responses)

## Key File Locations

**Entry Points:**
- `redical_redis/src/lib.rs`: Module registration; all commands, data types, event handlers, and config registered here via `redis_module!` macro

**Configuration:**
- `Cargo.toml` (workspace root): Shared dependency versions for all crates
- `rustfmt.toml`: Formatting rules
- `redical_redis/src/lib.rs`: `ical-parser-timeout-ms` runtime config (default 500ms, range 1–60000)

**Core Domain:**
- `redical_core/src/calendar.rs`: `Calendar` and `CalendarIndexUpdater`
- `redical_core/src/event.rs`: `Event` including `parse_ical`, `rebuild_indexes`, occurrence iteration delegation
- `redical_core/src/inverted_index.rs`: Index data structures and `IndexedConclusion` merge logic
- `redical_core/src/geo_index.rs`: Geospatial index (`rstar` R-tree)

**Parsing Foundations:**
- `redical_ical/src/lib.rs`: `ICalendarEntity`, `ICalendarComponent`, `ParserInput`, `ParserResult`, `ParserError`, `map_err`, `terminated_lookahead`, `impl_icalendar_entity_traits!` macro
- `redical_ical/src/grammar.rs`: Fundamental nom combinators used by all property parsers

**Persistence:**
- `redical_redis/src/datatype/rdb_data.rs`: `RDBCalendar`, `RDBEvent`, `RDBEventOccurrenceOverride` — bincode-serialized persistence structs

**Testing:**
- `tests/integration.rs`: Integration test functions
- `tests/macros.rs`: `set_and_assert_calendar!`, `assert_keyspace_events_published!`, etc.
- `redical_ical/tests/`: Unit-level parser tests including fuzz regression cases (`tests/fuzz_finds/`)

## Naming Conventions

**Files:**
- Snake_case for all `.rs` files
- Command handlers named after the Redis command: `rdcl_evt_set.rs` → `rdcl.evt_set`
- Entity/concept files named after the type they primarily define: `calendar.rs`, `inverted_index.rs`

**Directories:**
- Snake_case
- Crate roots prefixed with `redical_`: `redical_ical`, `redical_core`, `redical_redis`

**Types:**
- PascalCase structs and enums: `Calendar`, `EventOccurrenceIterator`, `InvertedCalendarIndexTerm`
- Trait names prefixed with `I` for iCalendar domain traits: `ICalendarEntity`, `ICalendarProperty`, `ICalendarComponent`, `ICalendarGeoProperty`, `ICalendarDateTimeProperty`

**Commands:**
- Redis command names: `rdcl.<entity_abbr>_<action>` (e.g. `rdcl.evt_set`, `rdcl.evi_query`, `rdcl.evo_del`, `rdcl.cal_idx_rebuild`)
  - `evt` = event, `evi` = event instance, `evo` = event occurrence override, `cal` = calendar

## Where to Add New Code

**New iCal property type:**
- Parser + renderer: `redical_ical/src/properties/event/` (for event properties) or `redical_ical/src/properties/query/` (for query properties)
- Add to `EventProperty` enum in `redical_ical/src/properties/event/mod.rs`
- Unit tests: inline `#[cfg(test)]` module within the property file

**New domain index:**
- Index struct: `redical_core/src/` (e.g. a new `*_index.rs` file)
- Add field to `Calendar` in `redical_core/src/calendar.rs`
- Add update methods to `CalendarIndexUpdater`
- Add rebuild logic to `Calendar::rebuild_indexes`
- Add search method to `QueryIndexAccessor` trait in `redical_core/src/queries/query.rs`

**New Redis command:**
- Handler: `redical_redis/src/commands/rdcl_<entity>_<action>.rs`
- Register in `redical_redis/src/commands/mod.rs` (mod + pub use)
- Register in `redis_module!` macro in `redical_redis/src/lib.rs`
- Integration test: `tests/integration.rs`

**New query filter/clause:**
- Query property type: `redical_ical/src/properties/query/`
- Filter evaluation: `redical_core/src/queries/indexed_property_filters.rs`
- Wire into `QueryParser`: `redical_core/src/queries/query_parser.rs`

**Shared utilities:**
- Core-level helpers: `redical_core/src/utils.rs`
- Redis-layer helpers: `redical_redis/src/utils.rs`

## Special Directories

**`redical_ical_afl_fuzz_targets/`:**
- Purpose: AFL++ fuzzing harnesses for the iCal parser
- Generated: No
- Committed: Yes; excluded from default workspace build members

**`redical_ical/tests/fuzz_finds/`:**
- Purpose: Regression test cases discovered during fuzzing (converted to unit tests)
- Generated: No (manually committed after fuzz runs)
- Committed: Yes

**`target/`:**
- Purpose: Cargo build output
- Generated: Yes
- Committed: No

---

*Structure analysis: 2026-03-06*
