# Feature Landscape

**Domain:** Versioned binary RDB serialization with fallback for a Rust Redis module
**Researched:** 2026-03-06
**Confidence:** HIGH (grounded entirely in the existing codebase; no external source ambiguity)

---

## Table Stakes

Features that must work correctly or the entire persistence story is broken.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| `RDBCalendarDump` struct | Envelope for version-gated fast path; without it there is no new format | Low | Three fields: `version: Option<String>`, `raw_dump: Vec<u8>`, `dump: RDBCalendar` |
| Legacy `RDBCalendar` load | Any existing RDB file is raw bincode of `RDBCalendar`; failure to load = data loss | Low | `rdb_load` must attempt `RDBCalendarDump` deserialization first, fall back to direct `RDBCalendar` deserialization on failure |
| Fast-path bypass when version absent | `GIT_SHA` env may be absent in detached builds; `version: None` must skip raw_dump entirely | Low | Use `option_env!("GIT_SHA")` — already set in `build.rs`; `None` → always use fallback `dump` |
| Fast-path bypass on version mismatch | Struct layout differs across commits; raw bincode of mismatched `Calendar` is undefined | Low | String equality on `GIT_SHA` is sufficient; any mismatch → use `dump` path |
| `catch_unwind` on raw_dump deserialization | `bincode` 1.3.3 can panic on malformed/mismatched input; Redis process must survive | Medium | Must be on the `raw_dump` path only; `RDBCalendar` deserialization already proved stable; use `std::panic::AssertUnwindSafe` wrapper |
| `rdb_save` writes `RDBCalendarDump` | New format must be emitted on save so fast path activates on next reload | Low | Serialize `Calendar` via `bincode::serialize` directly into `raw_dump`; derive `Serialize` + `Deserialize` on `Calendar` and all nested types |
| `serde` derives on `Calendar` and nested types | Required for `bincode::serialize/deserialize` on the raw path | High | `Calendar` contains `BTreeMap`, `InvertedCalendarIndex<T>`, `GeoSpatialCalendarIndex` (backed by `rstar::RTree`); `rstar` 0.11 has `serde` feature — already enabled in workspace; `geo` 0.26 has `use-serde` — already enabled; each nested type needs derives audited |
| `aof_rewrite` stub replaces `todo!()` | `todo!()` panics if AOF path is triggered; Redis process dies | Trivial | Empty `unsafe extern "C" fn` body — no logic required |
| Pre-generated binary fixture: legacy format | Tests must assert correct load of bytes that were never touched by new code | Medium | Script generates `RDBCalendar` bytes via `bincode::serialize`, commits as `tests/fixtures/legacy_rdb_calendar.bin` |
| Pre-generated binary fixture: new format | Tests must assert correct load of `RDBCalendarDump` bytes | Medium | Script generates with a known GIT_SHA so version-match test is deterministic |
| Integration tests load both fixtures | Confirms the two-path dispatch logic against real bytes, not in-memory synthesis | Medium | Tests live in `redical_redis/src/datatype/rdb_data.rs` or a new `tests/` module; assert resulting `Calendar` matches expected structure |

---

## Differentiators

Features that add value but the persistence story works without them.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Log on fast-path fallback | Observability: operators can see when version mismatch caused degraded path | Low | Only possible inside `rdb_load` — no `ctx` available, but `eprintln!` / `log::warn!` via the module logger works |
| Fixture generation as a `cargo test` helper | Makes it easy to regenerate fixtures after structural changes | Low | Gate behind `#[ignore]` or a separate binary in `redical_redis/bin/` |
| `redismodule-rs` upgrade | Current `redis-module = 2.0.2` may be behind; upgrade unlocks `save_string_buffer` which avoids the `from_utf8_unchecked` hack in `rdb_save` | Medium | Separate task; do not block the core RDB milestone on this |

---

## Anti-Features

Things to deliberately not build.

| Anti-Feature | Why Avoid | What to Do Instead |
|--------------|-----------|-------------------|
| AOF rewrite functional implementation | Out of scope per PROJECT.md; adds complexity with no current consumer | Empty stub; track as future work |
| Cross-platform fixture portability | `bincode` 1.x layout is platform-sensitive for some types; CI fixtures are for the CI arch only | Document fixtures as arch-specific in a comment; do not add endian-conversion logic |
| Downgrade path (new binary reading old `RDBCalendarDump` format) | Not required per PROJECT.md; adds a third deserialization branch with no use case | Skip; version mismatch already falls back to `RDBCalendar` |
| Serde derives on index types that don't need them | `Calendar`'s in-memory indexes (`InvertedCalendarIndex`, `GeoSpatialCalendarIndex`) are rebuilt after load via `rebuild_indexes()` — they do not need to be serialized | Exclude index fields from serde via `#[serde(skip)]`; only `uid` + `events` + `indexes_active` need to round-trip |
| Version-based migration logic | The version field is a binary same/different signal, not a migration registry | Keep the check as a single string equality; do not add a match table of versions |
| Async or threaded fixture generation at test runtime | Fixtures must be committed; generating at test time makes tests non-deterministic | Generate offline, commit binaries |

---

## Edge Cases to Cover in Integration Tests

These are the observable states the version-match / fallback logic can reach. Each must have a test.

### Version-match / Fallback dispatch

| Scenario | Input | Expected Outcome | Test Name |
|----------|-------|-----------------|-----------|
| Legacy `RDBCalendar` bytes (old format, no wrapper) | Raw bincode of `RDBCalendar` | Falls through to `RDBCalendar` path; `Calendar` rehydrated correctly | `test_rdb_load_legacy_format` |
| `RDBCalendarDump` with matching `GIT_SHA` | Dump with `version == Some(current_sha)` | Fast path taken; `raw_dump` deserialized directly into `Calendar` | `test_rdb_load_fast_path_version_match` |
| `RDBCalendarDump` with mismatched `GIT_SHA` | Dump with `version == Some("oldsha")` | Fast path skipped; `dump` field used; `Calendar` rehydrated via `RDBCalendar` | `test_rdb_load_fast_path_version_mismatch` |
| `RDBCalendarDump` with `version == None` | Dump with absent GIT_SHA at save time | Fast path skipped; `dump` field used | `test_rdb_load_no_version` |
| `catch_unwind` catches panic on malformed raw_dump | `raw_dump` contains garbage bytes that would panic bincode | Falls back to `dump`; no process death; `Calendar` rehydrated correctly | `test_rdb_load_raw_dump_panic_recovery` |
| `catch_unwind` catches panic; `dump` also fails | Both `raw_dump` and `dump` are corrupt | Returns `null_mut()` (or error path); process survives | `test_rdb_load_both_paths_fail_gracefully` |
| Round-trip save → load in same build | `rdb_save` then `rdb_load` on same binary | Fast path taken; calendar identity preserved | `test_rdb_round_trip_same_version` |
| Empty `Calendar` (no events) | Calendar with `uid` only, no events | Round-trip succeeds with empty events map | `test_rdb_round_trip_empty_calendar` |
| Calendar with events and occurrence overrides | Full fixture from existing `test_calendar_rdb_entity` | Both legacy and new format round-trip preserves all events and overrides | Extend existing unit test or new integration test |

### Serde correctness

| Scenario | Expected Outcome |
|----------|-----------------|
| `Calendar` with index fields serialized | Index fields excluded via `#[serde(skip)]`; `rebuild_indexes()` called after deserialization |
| `Calendar` with `BTreeMap<String, Box<Event>>` | `Box<T>` is transparent to serde; no special handling needed |
| `Event` with `ScheduleProperties` containing `RRuleSet` | `rrule` crate already has `serde` feature enabled; verify `RRuleSet` derives round-trip correctly |
| `EventOccurrenceOverride` with `Option<DTStartProperty>` | `None` round-trips to `None`; `Some(v)` must serialize/deserialize to the same value |

### `aof_rewrite` stub

| Scenario | Expected Outcome |
|----------|-----------------|
| `aof_rewrite` called by Redis | Returns without panic; no `todo!()` explosion |

---

## Feature Dependencies

```
serde derives on Calendar + nested types
  → rdb_save writes raw_dump into RDBCalendarDump
    → rdb_load fast path (version match)
      → catch_unwind safety wrapper
        → fallback to RDBCalendar dump field

Legacy RDBCalendar deserialization (existing, unchanged)
  → fallback path when RDBCalendarDump deserialization fails entirely

Pre-generated fixture (legacy)
  → integration test: legacy load

Pre-generated fixture (new format, known SHA)
  → integration test: fast-path load
  → integration test: version-mismatch load (same fixture, different SHA at test time)

aof_rewrite stub (independent, no dependencies)
```

---

## MVP Recommendation

Implement in this order to unblock everything else:

1. `aof_rewrite` stub — removes `todo!()` panic risk immediately; zero dependencies
2. `serde` derive audit — identify which nested types need derives and which need `#[serde(skip)]`; must complete before any serialization code compiles
3. `RDBCalendarDump` struct + `rdb_save` update — new save format
4. `rdb_load` two-path dispatch with `catch_unwind` — version-gated fast path
5. Fixture generation script + committed fixtures
6. Integration tests against fixtures

Defer: `redismodule-rs` upgrade — independent; no current blocker from 2.0.2 beyond the `from_utf8_unchecked` cosmetic issue.

---

## Sources

- `redical_redis/src/datatype/mod.rs` — existing `rdb_load`/`rdb_save`/`aof_rewrite` implementations
- `redical_redis/src/datatype/rdb_data.rs` — `RDBCalendar`, `RDBEvent`, `RDBEventOccurrenceOverride` with existing serde derives and round-trip tests
- `redical_core/src/calendar.rs` — `Calendar` struct field inventory (confirmed: no serde derives, index fields present)
- `redical_core/src/event.rs` — `ScheduleProperties`, `Event` struct; `rrule` crate has `serde` feature enabled
- `redical_redis/build.rs` — confirms `GIT_SHA` set via `git rev-parse --short HEAD`; `option_env!("GIT_SHA")` is the correct accessor
- `redical_redis/Cargo.toml` — confirms `bincode 1.3.3`, `serde 1.0.162`, `redis-module 2.0.2`
- `Cargo.toml` workspace — confirms `rstar` has `serde` feature, `geo` has `use-serde` feature
- `.planning/codebase/TESTING.md` — integration test infrastructure (live Redis on port 6480, sequential macro, fixture pattern)
- `.planning/PROJECT.md` — canonical requirements, out-of-scope items, key decisions
