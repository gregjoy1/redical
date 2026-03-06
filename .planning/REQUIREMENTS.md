# Requirements: RediCal RDB Fast-Path Serialization

**Defined:** 2026-03-06
**Core Value:** Calendar RDB load/save must be fast for same-version deployments while never corrupting or losing data across version boundaries.

## v1 Requirements

### Safety

- [x] **SAFE-01**: `aof_rewrite` replaced with an empty no-op stub (remove `todo!()` to prevent Redis crash on AOF rewrite)
- [x] **SAFE-02**: `from_utf8_unchecked` in `rdb_save` replaced with a safe alternative (use `save_string_buffer` if available after redis-module upgrade, otherwise safe conversion)

### Upgrade

- [x] **UPGR-01**: `redis-module` Cargo.toml version updated from `2.0.2` to `2.0.4` (already resolved in lockfile; Cargo.toml string alignment)

### Serde

- [x] **SERD-01**: `serde` dependency added to `redical_ical/Cargo.toml` (currently zero serde infrastructure in that crate)
- [x] **SERD-02**: `#[derive(Serialize, Deserialize)]` added to all `redical_ical` property types that appear in `Calendar`'s field graph (compiler-driven discovery)
- [x] **SERD-03**: `#[derive(Serialize, Deserialize)]` added to `redical_core` types: `Calendar`, `Event`, `EventOccurrenceOverride`, and all nested value types
- [x] **SERD-04**: `#[serde(skip)]` applied to all computed/index fields: `Calendar::indexed_categories`, `Calendar::indexed_geo`, `Calendar::indexed_class`, `Calendar::indexed_related_to`, `Calendar::indexed_location_type`; same fields on `Event`; `ScheduleProperties::parsed_rrule_set`
- [x] **SERD-05**: `chrono` serde feature confirmed enabled in workspace `Cargo.toml` (verify, add if missing)

### RDB Format

- [x] **RDB-01**: `RDBCalendarDump` struct added to `rdb_data.rs` with fields: `version: Option<String>`, `raw_dump: Vec<u8>`, `dump: RDBCalendar`
- [x] **RDB-02**: `rdb_save` serializes `RDBCalendarDump`: `version` from `option_env!("GIT_SHA")`, `raw_dump` from bincode of `Calendar`, `dump` from existing `RDBCalendar`
- [x] **RDB-03**: `rdb_load` implements three-layer dispatch:
  1. Attempt `RDBCalendarDump` deserialization — if fails, fall back to legacy bare `RDBCalendar` path
  2. If `RDBCalendarDump` succeeds: if `version` is `None` or mismatches current `GIT_SHA`, load from `dump` (iCal path)
  3. If version matches: attempt fast-path bincode deserialization of `raw_dump` into `Calendar`
- [x] **RDB-04**: Fast-path `raw_dump` deserialization wrapped in `std::panic::catch_unwind` with `AssertUnwindSafe`; on panic or `Err`, falls back to `dump` (`RDBCalendar` iCal path)
- [x] **RDB-05**: After fast-path deserialization, `rebuild_indexes()` called on resulting `Calendar` before returning

### Integration Tests

- [x] **TEST-01**: Pre-generated binary fixture `tests/fixtures/rdb_calendar_legacy.bin` committed — bare `RDBCalendar` bincode bytes
- [x] **TEST-02**: Pre-generated binary fixture `tests/fixtures/rdb_calendar_dump_mismatch.bin` committed — `RDBCalendarDump` with deliberately mismatched version string
- [x] **TEST-03**: `#[ignore]`-gated generator test in `rdb_data.rs` to regenerate fixtures (run manually before committing new fixture files)
- [x] **TEST-04**: Integration test: loading `rdb_calendar_legacy.bin` via `rdb_load` logic produces correct `Calendar` (backward compat)
- [x] **TEST-05**: Integration test: loading `rdb_calendar_dump_mismatch.bin` falls back to iCal path and produces correct `Calendar`
- [x] **TEST-06**: In-process unit test: `rdb_save` → `rdb_load` round-trip within same build produces identical `Calendar` via fast path

## v2 Requirements

### AOF

- **AOF-01**: `aof_rewrite` functional implementation — emit `RICAL.SET` command to reconstruct key

### Performance

- **PERF-01**: Benchmark comparison of legacy vs fast-path load times for large calendars

## Out of Scope

| Feature | Reason |
|---------|--------|
| Downgrade path (new binary reading old `RDBCalendarDump`) | Not required; fallback to legacy handles version mismatches |
| Cross-platform binary fixture portability | Fixtures are for CI on a single arch; cross-arch guarantees not needed |
| serde derives on index types (`InvertedCalendarIndex`, `GeoSpatialCalendarIndex`) | Indexes are always rebuilt post-load; serializing them adds size and complexity |
| AOF rewrite functional implementation | Deferred to v2; stub unblocks compilation |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| SAFE-01 | Phase 1 | Complete |
| SAFE-02 | Phase 1 | Complete |
| UPGR-01 | Phase 1 | Complete |
| SERD-01 | Phase 2 | Complete |
| SERD-02 | Phase 2 | Complete |
| SERD-03 | Phase 2 | Complete |
| SERD-04 | Phase 2 | Complete |
| SERD-05 | Phase 2 | Complete |
| RDB-01 | Phase 3 | Complete |
| RDB-02 | Phase 3 | Complete |
| RDB-03 | Phase 3 | Complete |
| RDB-04 | Phase 3 | Complete |
| RDB-05 | Phase 3 | Complete |
| TEST-01 | Phase 4 | Complete |
| TEST-02 | Phase 4 | Complete |
| TEST-03 | Phase 4 | Complete |
| TEST-04 | Phase 4 | Complete |
| TEST-05 | Phase 4 | Complete |
| TEST-06 | Phase 4 | Complete |

**Coverage:**
- v1 requirements: 19 total
- Mapped to phases: 19
- Unmapped: 0 ✓

---
*Requirements defined: 2026-03-06*
*Last updated: 2026-03-06 after initial definition*
