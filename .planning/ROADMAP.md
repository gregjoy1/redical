# Roadmap: RediCal RDB Fast-Path Serialization

## Overview

This milestone closes two crash risks in the existing codebase, derives serde across the full Calendar type graph, implements the versioned dual-representation RDB envelope, and validates all load paths with committed binary fixtures and integration tests.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [x] **Phase 1: Safety Fixes** - Close `aof_rewrite` `todo!()` crash and `from_utf8_unchecked` UB before touching RDB code (completed 2026-03-06)
- [x] **Phase 2: Serde Derive Chain** - Add serde to `redical_ical` and derive `Serialize`/`Deserialize` across the full `Calendar` type graph (completed 2026-03-06)
- [ ] **Phase 3: RDB Format** - Implement `RDBCalendarDump` envelope, update `rdb_save`/`rdb_load` with three-layer fallback and `catch_unwind`
- [ ] **Phase 4: Fixtures and Integration Tests** - Commit pre-generated binary fixtures and cover all dispatch paths with integration tests

## Phase Details

### Phase 1: Safety Fixes
**Goal**: The codebase compiles and runs without crash risks or undefined behaviour on every RDB save
**Depends on**: Nothing (first phase)
**Requirements**: SAFE-01, SAFE-02, UPGR-01
**Success Criteria** (what must be TRUE):
  1. `aof_rewrite` is an empty no-op stub — `BGREWRITEAOF` no longer panics Redis
  2. `rdb_save` uses only safe string conversion — no `from_utf8_unchecked` call remains
  3. `redis-module` version in `Cargo.toml` matches `2.0.4` (already resolved in lockfile)
  4. `cargo build` succeeds with no warnings from the changed files
**Plans**: 1 plan

Plans:
- [ ] 01-01-PLAN.md — Bump redis-module to 2.0.4, empty aof_rewrite stub, replace from_utf8_unchecked with raw::save_slice

### Phase 2: Serde Derive Chain
**Goal**: `bincode::serialize(&calendar)` compiles — every type reachable from `Calendar` derives `Serialize + Deserialize`, and computed index fields are annotated `#[serde(skip)]`
**Depends on**: Phase 1
**Requirements**: SERD-01, SERD-02, SERD-03, SERD-04, SERD-05
**Success Criteria** (what must be TRUE):
  1. `redical_ical/Cargo.toml` declares `serde = { workspace = true }` (previously had no serde dependency)
  2. `bincode::serialize(&calendar)` and `bincode::deserialize::<Calendar>(bytes)` compile without error
  3. All computed/index fields (`indexed_categories`, `indexed_geo`, `indexed_class`, `indexed_related_to`, `indexed_location_type`, `parsed_rrule_set`) carry `#[serde(skip)]`
  4. `cargo test` passes — existing `RDBCalendar` round-trip tests still green
**Plans**: 2 plans

Plans:
- [x] 02-01-PLAN.md — Cargo.toml changes, Tzid custom serde, derive Serialize/Deserialize on all redical_ical types
- [ ] 02-02-PLAN.md — Derive serde on redical_core types with skip annotations, bincode round-trip smoke test

### Phase 3: RDB Format
**Goal**: RDB save always writes the dual-representation `RDBCalendarDump` envelope; RDB load selects the fast path when versions match, falls back to iCal safely on any mismatch or failure
**Depends on**: Phase 2
**Requirements**: RDB-01, RDB-02, RDB-03, RDB-04, RDB-05
**Success Criteria** (what must be TRUE):
  1. `RDBCalendarDump` struct exists in `rdb_data.rs` with `version: Option<String>`, `raw_dump: Vec<u8>`, and `dump: RDBCalendar` fields
  2. `rdb_save` writes both `raw_dump` (bincode of `Calendar`) and `dump` (`RDBCalendar` iCal fallback) inside the envelope
  3. `rdb_load` falls back to the legacy bare `RDBCalendar` path when outer `RDBCalendarDump` deserialization fails (backward compat)
  4. When `GIT_SHA` is absent at build time, fast path is always skipped (version is `None`)
  5. Fast-path deserialization is wrapped in `catch_unwind` — a panic in bincode or `rebuild_indexes()` does not crash Redis
**Plans**: TBD

### Phase 4: Fixtures and Integration Tests
**Goal**: All dispatch paths are covered by tests; legacy and mismatch-version binary fixtures are committed and load correctly
**Depends on**: Phase 3
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06
**Success Criteria** (what must be TRUE):
  1. `tests/fixtures/rdb_calendar_legacy.bin` and `tests/fixtures/rdb_calendar_dump_mismatch.bin` exist and are committed
  2. Loading `rdb_calendar_legacy.bin` via `rdb_load` logic produces the correct `Calendar` (backward compat verified)
  3. Loading `rdb_calendar_dump_mismatch.bin` falls back to the iCal path and produces the correct `Calendar`
  4. An in-process `rdb_save` → `rdb_load` round-trip within the same build produces an identical `Calendar` via the fast path
  5. A `#[ignore]`-gated fixture generator test exists and can regenerate fixtures without modifying test logic
**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Safety Fixes | 1/1 | Complete   | 2026-03-06 |
| 2. Serde Derive Chain | 2/2 | Complete   | 2026-03-06 |
| 3. RDB Format | 0/? | Not started | - |
| 4. Fixtures and Integration Tests | 0/? | Not started | - |
