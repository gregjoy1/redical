# Project Research Summary

**Project:** RediCal RDB Fast-Path Serialization
**Domain:** Versioned binary RDB serialization with fallback for a Rust Redis module
**Researched:** 2026-03-06
**Confidence:** HIGH

## Executive Summary

This milestone adds a versioned binary fast-path to RediCal's RDB persistence. The approach wraps the existing `RDBCalendar` (iCal string-based) serialization in a new `RDBCalendarDump` envelope struct that also carries a raw `bincode` blob of `Calendar`. On load, if the stored `GIT_SHA` matches the current build, the raw blob is used directly — skipping expensive iCal re-parsing. Any mismatch, absence, or deserialization failure falls transparently through to the existing iCal path. The existing path is not modified; correctness is preserved unconditionally.

The main implementation cost is adding `#[derive(Serialize, Deserialize)]` across the `Calendar` type graph in `redical_core` and `redical_ical`. The `redical_ical` crate currently has no serde dependency at all, so this requires a Cargo.toml addition and derives across all property and value types that appear as owned fields. The right approach is compiler-driven: add the derive to `Calendar` first and follow errors bottom-up rather than auditing the full type graph upfront.

The key risks are process-crashing pitfalls: the existing `todo!()` in `aof_rewrite` and the `from_utf8_unchecked` undefined behaviour in `rdb_save` must both be resolved before RDB format work begins. `catch_unwind` must wrap the entire `rdb_load` body — not just the bincode call — because `rebuild_indexes()` and `validate()` also contain internal `unwrap()` chains. Computed index fields (`indexed_categories`, `indexed_geo`, etc.) and the `parsed_rrule_set` cache must be annotated `#[serde(skip)]` to prevent serializing derived state.

---

## Key Findings

### Recommended Stack

No new dependencies are required. `bincode 1.3.3` and `serde 1.0.162` are already present in `redical_redis`. The workspace Cargo.toml already enables `serde` features on `rrule`, `rstar`, and `geo`. The only Cargo change needed is adding `serde = { workspace = true }` to `redical_ical/Cargo.toml` and bumping `redis-module` from `2.0.2` to `2.0.4` in `redical_redis/Cargo.toml` (Cargo.lock already resolves to 2.0.4).

**Core technologies:**
- `bincode 1.3.3` — binary serialization for RDB fast path — stay on 1.x; 2.x has breaking API and format changes that would invalidate existing RDB blobs
- `serde 1.0.162` — derive infrastructure — already workspace-wide; only `redical_ical` needs the dependency added explicitly
- `redis-module 2.0.4` — Redis native type host — bump Cargo.toml to match what Cargo.lock already resolves; no API changes at patch level
- `option_env!("GIT_SHA")` — build-time version token — already set by `build.rs` via `git rev-parse --short HEAD`; `None` when absent, which safely disables the fast path

### Expected Features

**Must have (table stakes):**
- `aof_rewrite` stub — removes the `todo!()` panic risk; must be first change before anything else touches `mod.rs`
- `from_utf8_unchecked` fix — resolves undefined behaviour on every current save; must precede `RDBCalendarDump` serialization
- `serde` derives on `Calendar` and all nested types — prerequisite for `bincode::serialize(&calendar)` to compile
- `#[serde(skip)]` on all computed index fields and `parsed_rrule_set` — prevents silent correctness corruption
- `RDBCalendarDump` struct + updated `rdb_save` — new save format; always writes both `raw_dump` and `dump`
- `rdb_load` two-path dispatch with `catch_unwind` — version-gated fast path with full-body panic containment
- Legacy `RDBCalendar` fallback — unchanged existing path; reached when outer `RDBCalendarDump` deserialization fails
- Pre-generated binary fixtures — committed to `tests/fixtures/`; legacy format and new-format-with-mismatched-SHA (fast-path fixture generated at test time, not committed)
- Integration tests covering all dispatch paths — version match, mismatch, absent version, panic recovery, legacy bytes, round-trip

**Should have (differentiators):**
- Log on fast-path fallback — operator observability when version mismatch forces iCal re-parse
- `#[ignore]`-gated fixture generator — makes fixture regeneration after struct changes reproducible

**Defer:**
- AOF rewrite functional implementation — out of scope per PROJECT.md; empty stub is sufficient
- `redis-module` upgrade beyond 2.0.4 — independent task; 2.0.4 resolves the immediate version mismatch but `save_string_buffer` may require a further upgrade
- Downgrade path / migration registry — not required; version mismatch already falls back safely

### Architecture Approach

The architecture is a layered deserialization fallback with a dual-representation envelope. `rdb_save` always writes both a raw bincode blob of `Calendar` (`raw_dump`) and the existing iCal string tree (`dump`) inside `RDBCalendarDump`. `rdb_load` peels the layers: outer envelope deserialization first, then version check, then `catch_unwind`-wrapped fast-path deserialization of `raw_dump`, with fallback at every layer. The legacy path (bytes written before this change) is reached by catching the outer envelope deserialization failure.

**Major components:**
1. `RDBCalendarDump` (new, `rdb_data.rs`) — envelope struct: `version: Option<String>`, `raw_dump: Vec<u8>`, `dump: RDBCalendar`
2. Updated `rdb_save` (`mod.rs`) — serializes `Calendar` twice; wraps in `RDBCalendarDump`; resolves `from_utf8_unchecked`
3. Updated `rdb_load` (`mod.rs`) — three-layer fallback with `catch_unwind` wrapping the entire body
4. `aof_rewrite` stub (`mod.rs`) — empty `extern "C"` fn; removes `todo!()` crash risk
5. serde derives on `redical_core` types — `Calendar`, `Event`, `ScheduleProperties`, `IndexedProperties`, `PassiveProperties`, `EventOccurrenceOverride`, inverted index types, geo types; all computed fields `#[serde(skip)]`
6. serde dependency + derives on `redical_ical` types — all property and value types reachable from `Calendar`; compiler-driven discovery
7. Fixture generator (`#[ignore]` test in `rdb_data.rs`) + committed fixtures in `tests/fixtures/`
8. Integration tests — cover all dispatch paths including panic recovery

### Critical Pitfalls

1. **Serializing computed index fields** — `Calendar` and `Event` carry derived indexes that must be annotated `#[serde(skip)]`; omitting this produces silently corrupt query results with no deserialization error
2. **`catch_unwind` scoped too narrowly** — must wrap the entire `rdb_load` body including `rebuild_indexes()` and `validate()`, not just the bincode call; a panic in index construction still crashes Redis if not contained
3. **`from_utf8_unchecked` undefined behaviour** — must be resolved before `RDBCalendarDump` is written; `raw_dump` bincode is even more likely to contain non-UTF-8 bytes than existing `RDBCalendar` bincode
4. **`aof_rewrite` `todo!()` crash** — any Redis instance with AOF enabled will crash on `BGREWRITEAOF`; fix this first, before any other change
5. **`GIT_SHA` instability for fixture tests** — short SHA changes on every rebase; fast-path fixture tests must generate bytes within the same test binary rather than loading committed fixtures; only fallback-path fixtures should be committed

---

## Implications for Roadmap

Based on research, suggested phase structure:

### Phase 1: Safety fixes
**Rationale:** Two crash risks exist in the current codebase that must be closed before any new code is written. Both are independent of each other and of the RDB format work. Doing this first means the base is stable for all subsequent phases.
**Delivers:** `aof_rewrite` stub (removes `todo!()` crash); `from_utf8_unchecked` fix in `rdb_save` (removes UB on every save)
**Addresses:** Table-stakes items with zero-dependency; blocks nothing
**Avoids:** Pitfall 6 (`aof_rewrite` crash), Pitfall 4 (`from_utf8_unchecked` UB)

### Phase 2: serde derive chain
**Rationale:** `bincode::serialize(&calendar)` cannot compile until `Calendar` and every transitively-owned type derive `Serialize + Deserialize`. This is the highest-effort phase and gates all serialization work. Compiler-driven discovery is the right approach — add derives top-down and fix errors bottom-up.
**Delivers:** `serde = { workspace = true }` in `redical_ical/Cargo.toml`; `#[derive(Serialize, Deserialize)]` on all `redical_ical` property/value types; `#[derive(Serialize, Deserialize)]` on `Calendar`, `Event`, `ScheduleProperties`, `IndexedProperties`, `PassiveProperties`, `EventOccurrenceOverride`, and all inverted index / geo types in `redical_core`; `#[serde(skip)]` on all computed index fields and `parsed_rrule_set`
**Uses:** `serde 1.0.162` (workspace), `rrule`/`rstar`/`geo` serde features (already enabled)
**Avoids:** Pitfall 1 (index fields serialized), Pitfall 2 (`parsed_rrule_set` serialized), Pitfall 8 (`RTree` in raw dump)

### Phase 3: RDB format — save and load
**Rationale:** With serde derives in place, the new envelope struct and updated hooks can be implemented. `rdb_save` and `rdb_load` are rewritten together because their contract is symmetric. `catch_unwind` must wrap the full `rdb_load` body.
**Delivers:** `RDBCalendarDump` struct in `rdb_data.rs`; updated `rdb_save` (dual-representation write); updated `rdb_load` (three-layer fallback with full-body `catch_unwind`); `redis-module` version bump to `2.0.4`
**Implements:** Dual-format envelope + layered deserialization fallback architecture
**Avoids:** Pitfall 3 (`catch_unwind` scope), Pitfall 5 (bincode field-order fragility via documented struct contract)

### Phase 4: Fixtures and integration tests
**Rationale:** Binary fixtures must be committed after the format is stable, not before. The fixture generator is `#[ignore]`-gated to avoid regenerating at CI time. Integration tests cover all dispatch paths; fast-path tests generate their own bytes in-process rather than loading committed fixtures.
**Delivers:** `tests/fixtures/rdb_calendar_legacy.bin`, `tests/fixtures/rdb_calendar_dump.bin` (mismatched-SHA version); `#[ignore]`-gated fixture generator in `rdb_data.rs`; integration tests for all 8 dispatch scenarios identified in FEATURES.md
**Avoids:** Pitfall 7 (SHA instability in CI fixture tests), Pitfall 5 (format drift detection)

### Phase Ordering Rationale

- Phase 1 before everything: two crash risks must be closed before touching `mod.rs` for RDB changes
- Phase 2 before Phase 3: serde derives are a hard compile-time prerequisite; the RDB code cannot be written until `Calendar` is serde-capable
- Phase 3 before Phase 4: fixtures must be generated from stable format; fixture bytes are meaningless before the save/load code is final
- Phases 2 and 3 are the only phases with significant unknowns; both can proceed in a single implementation pass if the developer is comfortable with compiler-driven discovery

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 2:** `redical_ical` property/value type inventory is large and not fully enumerated; compiler-driven discovery is the plan but the scope is genuinely unknown until the first compile attempt
- **Phase 3:** `from_utf8_unchecked` fix depends on what `redis-module` 2.0.4 exposes; if `save_string_buffer` is still absent, a base64 encode/decode workaround is needed — verify API surface before implementing

Phases with standard patterns (skip research-phase):
- **Phase 1:** `aof_rewrite` stub is a one-line change; `from_utf8_unchecked` fix is a known workaround pattern
- **Phase 4:** fixture generation and integration test patterns are established in the existing codebase (`redical_ical/tests/fuzz_finds/` precedent)

---

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | All dependencies verified from Cargo.lock and workspace Cargo.toml; no new crates required |
| Features | HIGH | Grounded entirely in existing codebase; no external source ambiguity |
| Architecture | HIGH | Derived from direct source inspection; patterns are well-established in the existing `rdb_data.rs` |
| Pitfalls | HIGH | Derived from direct codebase inspection and known Rust/bincode/Redis module behaviours |

**Overall confidence:** HIGH

### Gaps to Address

- `redical_ical` property/value type serde surface: exact set of types needing derives is unknown upfront; resolve via compiler-driven discovery in Phase 2
- `redis-module` 2.0.4 API: `save_string_buffer` availability not verified (WebFetch restricted); check changelog before implementing the `from_utf8_unchecked` fix in Phase 1/3
- `chrono` serde feature: ARCHITECTURE.md flags `chrono` serde feature as needing verification; confirm `serde` feature is included in the workspace `chrono` dependency before Phase 2

---

## Sources

### Primary (HIGH confidence)
- `redical_redis/src/datatype/mod.rs` — existing `rdb_load`/`rdb_save`/`aof_rewrite`
- `redical_redis/src/datatype/rdb_data.rs` — `RDBCalendar`, `RDBEvent`, `RDBEventOccurrenceOverride` with existing serde derives
- `redical_core/src/calendar.rs`, `event.rs`, `inverted_index.rs`, `geo_index.rs`, `event_occurrence_override.rs` — Calendar type graph
- `Cargo.lock` — resolved versions (redis-module 2.0.4, bincode 1.3.3, rstar 0.11.0, rrule 0.10.0)
- Workspace `Cargo.toml` — serde, rrule, rstar, geo feature flags
- `redical_redis/build.rs` — `GIT_SHA` generation
- `.planning/PROJECT.md` — milestone requirements and out-of-scope items
- `.planning/codebase/CONCERNS.md` — pre-identified fragile areas

### Secondary (MEDIUM confidence)
- `redis-module` 2.0.2 → 2.0.4 API compatibility — inferred from patch-level semver bump; CHANGELOG not directly verified

---
*Research completed: 2026-03-06*
*Ready for roadmap: yes*
