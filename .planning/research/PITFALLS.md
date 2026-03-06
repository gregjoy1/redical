# Domain Pitfalls

**Domain:** Versioned binary RDB serialization with fallback in a Rust Redis module
**Researched:** 2026-03-06
**Confidence:** HIGH (derived from direct codebase inspection and known Rust/bincode/Redis module behaviours)

---

## Critical Pitfalls

### Pitfall 1: Serializing computed index fields into the raw dump

**What goes wrong:** `Calendar` and `Event` both carry derived indexes (`indexed_categories`, `indexed_location_type`, `indexed_related_to`, `indexed_geo`, `indexed_class`) that are rebuilt from the canonical event data via `rebuild_indexes()`. If `#[derive(Serialize, Deserialize)]` is added to `Calendar` or `Event` without skipping these fields, the raw dump will encode the index state at save time. On load, any schema change to an index type — even adding a field to `InvertedCalendarIndexTerm` or `GeoSpatialCalendarIndex` — will silently deserialize stale or mismatched index data. The loaded `Calendar` will appear valid but query results will be wrong.

**Why it happens:** Adding blanket `#[derive(Serialize, Deserialize)]` is the path of least resistance. Index fields have no visual distinction from canonical fields in the struct definition.

**Consequences:** Corrupt query results with no error. Only caught by querying post-load; no deserialization error surfaces.

**Prevention:**
- Annotate every computed index field with `#[serde(skip)]` on `Calendar` and `Event`.
- After fast-path deserialization, always call `rebuild_indexes()` before returning the loaded value — do not rely on deserialized index state even if it appears valid.
- Document the `#[serde(skip)]` annotations with a comment explaining why.

**Warning signs:** `indexed_categories`, `indexed_location_type`, `indexed_related_to`, `indexed_geo`, `indexed_class` fields on both `Calendar` and `Event` appearing in serialized output size benchmarks; test calendars with stale category results after round-trip.

**Phase:** Implementation of `#[derive(Serialize, Deserialize)]` on `Calendar`/`Event`/nested types.

---

### Pitfall 2: Serializing `parsed_rrule_set` into the raw dump

**What goes wrong:** `ScheduleProperties` contains `pub parsed_rrule_set: Option<rrule::RRuleSet>`. The `rrule` crate is already pulled in with `features = ["serde", "exrule"]`, so `RRuleSet` will serialize without a compile error. However, `parsed_rrule_set` is a derived cache of `rrule`/`exrule`/`rdate`/`exdate` fields — it is rebuilt by `build_parsed_rrule_set()` during `validate()`. If serialized into the raw dump, any internal change to how `RRuleSet` serializes (across rrule crate versions) will break fast-path loads. It also bloats the dump unnecessarily.

**Why it happens:** `RRuleSet` serializes without error, so the derive compiles silently. The field looks like an ordinary `Option<T>`.

**Consequences:** Fast-path load fails (or silently loads stale recurrence state) after rrule crate upgrade. The fast path then falls through to the iCal string fallback — which is correct behaviour but defeats the purpose.

**Prevention:**
- Annotate `parsed_rrule_set` with `#[serde(skip)]`.
- After fast-path deserialization, call `event.validate()` (which calls `build_parsed_rrule_set()`) for every event before returning the loaded `Calendar`.

**Warning signs:** Round-trip test passes but load is slower than expected (rrule re-parse is happening on every load); version mismatch after rrule upgrade causes fast-path fallback on every boot.

**Phase:** Implementation of `#[derive(Serialize, Deserialize)]` on `ScheduleProperties`.

---

### Pitfall 3: `catch_unwind` across an FFI boundary without `UnwindSafe` enforcement

**What goes wrong:** `rdb_load` is declared `pub extern "C" fn` — it is called from C (Redis). Rust's `catch_unwind` is designed to stop a panic from crossing an FFI boundary, but it only works correctly if the closure contains no non-`UnwindSafe` types. `bincode::deserialize::<Calendar>` will operate on a `&[u8]` (which is `UnwindSafe`), so the closure itself is safe. The danger is forgetting to wrap the call: if the `catch_unwind` is omitted or placed around too narrow a scope (e.g., only wrapping `bincode::deserialize` but not the subsequent `rebuild_indexes()` call), a panic inside index construction will still propagate across the FFI boundary into Redis. The existing code at `mod.rs:52` already has `bincode::deserialize(...).unwrap()` with no `catch_unwind` at all.

**Why it happens:** `catch_unwind` is easy to scope too narrowly. Developers wrap the deserialization call but forget that `rebuild_indexes()`, `validate()`, and `build_parsed_rrule_set()` can all panic via internal `unwrap()` chains.

**Consequences:** Redis process crashes on RDB load. Data is intact on disk but Redis cannot start. Requires manual intervention to clear or migrate the RDB.

**Prevention:**
- Wrap the entire `rdb_load` body (from raw bytes through to returning the `*mut c_void`) in a single `catch_unwind` closure.
- Return `null_mut()` and log the error string on `Err` from `catch_unwind`.
- Confirm that `DateTime::from_utc_timestamp` (known to panic on out-of-range timestamps per CONCERNS.md) cannot be reached via the deserialization path without a prior `Result` check.

**Warning signs:** Any code path inside `rdb_load` that calls `.unwrap()` or `.expect()` without a prior `?`; `rebuild_indexes()` calling methods on `GeoSpatialCalendarIndex` which uses `rstar::RTree` operations that can panic on out-of-bounds coordinates.

**Phase:** Implementation of `catch_unwind` wrapper in `rdb_load`.

---

### Pitfall 4: `from_utf8_unchecked` on bincode bytes is undefined behaviour

**What goes wrong:** The existing `rdb_save` at `mod.rs:80` calls `std::str::from_utf8_unchecked(&bytes[..])` because `redis-module` 2.0.2 does not expose `save_string_buffer`. Bincode output is arbitrary binary; it will routinely contain byte sequences that are not valid UTF-8. This is undefined behaviour in Rust — the compiler is permitted to miscompile code that invokes it.

This must be resolved before adding the fast-path dump, because the new `raw_dump` field (raw bincode of `Calendar`) is even more likely to contain non-UTF-8 bytes than the existing `RDBCalendar` bincode.

**Why it happens:** The comment already acknowledges this is a known workaround. The upgrade to the latest `redis-module` is listed as a milestone requirement and may provide `save_string_buffer` or an equivalent.

**Consequences:** Undefined behaviour on every save. In practice, Redis's string storage is binary-safe, so this often works — but the compiler is free to break it without warning, particularly under optimisation.

**Prevention:**
- Upgrade `redis-module` first and check for `save_string_buffer` or equivalent binary-safe save API.
- If not available post-upgrade: encode bytes as base64 before `save_string` and decode after `load_string`; this is safe and the overhead is small relative to iCal parse time.
- Do not introduce additional `from_utf8_unchecked` calls for the new `raw_dump` path.

**Warning signs:** `redis-module` changelog; any new field in the serialized output that contains arbitrary binary; integration test failures on non-ASCII calendar UIDs or event properties.

**Phase:** `redis-module` upgrade (prerequisite); must be resolved before RDBCalendarDump serialization is written.

---

### Pitfall 5: bincode 1.3.3 is not self-describing — struct layout changes silently corrupt data

**What goes wrong:** bincode 1.x encodes structs as a sequence of field values in declaration order with no field names, type tags, or version markers. Adding, removing, or reordering a field in `Calendar`, `Event`, `ScheduleProperties`, or any nested type changes the binary layout. A fixture generated with the old layout will deserialize into the wrong fields when decoded with the new layout — silently, because bincode does not detect the mismatch, and the data fits (e.g., a `u64` length prefix is read as a valid-looking string length). The result is either a panic (on clearly invalid data) or a silently corrupt `Calendar`.

**Why it happens:** Developers add a field to a struct for unrelated reasons and do not realise the raw dump fixture is now invalid. No compile-time or test-time warning occurs if the fixture still deserializes without a panic.

**Consequences:** Fast-path produces wrong `Calendar` state. The fixture-based backward compatibility test passes (fixture deserializes without panic) but the resulting `Calendar` has wrong field values. The `GIT_SHA` version check prevents the fast path from running in production on a different build, but within the same build the corruption would be invisible.

**Prevention:**
- Keep the set of fields in the raw-dumped types (`Calendar`, `Event`, `ScheduleProperties`, `IndexedProperties`, `PassiveProperties`, `EventOccurrenceOverride`) as stable as possible.
- After any struct field addition, re-generate fixtures and confirm old fixture now correctly falls through to the iCal fallback (because version will differ).
- Include a field-count assertion or a magic byte header at the start of the `raw_dump` to make truncation detectable.
- Document in the struct definition which fields are part of the raw dump serialization contract.

**Warning signs:** Fixture byte length changes without a corresponding `GIT_SHA` change; round-trip test passes but a field-by-field equality check of the deserialized `Calendar` fails; `bincode::deserialize` returns `Ok` but the resulting struct has nonsensical values.

**Phase:** Implementation of `RDBCalendarDump`; fixture generation; any future struct modification in `redical_core`.

---

### Pitfall 6: `aof_rewrite` hard `todo!()` crashes Redis during AOF rewrite

**What goes wrong:** The current `aof_rewrite` at `mod.rs:90` is a hard `todo!()` macro, which expands to a `panic!`. If Redis is configured with AOF persistence (`appendonly yes`), or if an operator manually triggers `BGREWRITEAOF`, Redis will invoke `aof_rewrite` for every RICAL_CAL key. Each invocation panics. Because this is an `extern "C"` function with no `catch_unwind`, the panic crosses the FFI boundary and crashes the Redis process.

**Why it happens:** The milestone already identifies this — replacing `todo!()` with an empty stub is listed as an active requirement. The risk is forgetting to do this before adding the new RDB serialization work, leaving the process in a state where the new fast-path save works but AOF rewrite is still fatal.

**Consequences:** Redis crash on AOF rewrite; data loss if the crash occurs mid-rewrite.

**Prevention:**
- Replace `todo!()` with an empty stub (or a Redis log call) as the very first change in the implementation phase, before any other changes.
- Add a test or CI check that invokes the AOF rewrite path (even as a no-op).

**Warning signs:** `todo!()` still present in `aof_rewrite` after any other RDB change has been made; CI running with `appendonly yes` in a Redis test config.

**Phase:** First task in implementation, before RDB format changes.

---

## Moderate Pitfalls

### Pitfall 7: `GIT_SHA` is a short SHA — not stable across rebases and force-pushes

**What goes wrong:** `build.rs` sets `GIT_SHA` via `git rev-parse --short HEAD`. A short SHA (7 hex chars by default) is the version discriminator for whether the fast path is trusted. Any rebase, amend, or force-push changes the SHA. In a CI environment where the test suite rebuilds after a rebase, the SHA will differ between the fixture-generating build and the test-loading build, causing the fast path to be skipped on every CI run even within the same codebase state.

This is by design for production deployments (where the SHA correctly identifies the exact binary), but it means CI fixture tests cannot rely on `GIT_SHA` matching — they must either re-generate fixtures at test time or use a separate mechanism.

**Prevention:**
- Fixture tests that exercise the fast path must generate the `raw_dump` bytes and `RDBCalendarDump` bytes within the same test binary (same build), not from committed fixture files.
- Committed fixture files should exercise the *fallback path* (legacy `RDBCalendar` bytes and mismatched-version `RDBCalendarDump` bytes). These do not need a matching SHA.
- Document this distinction clearly in the fixture generation script.

**Warning signs:** Fast-path CI test that loads a committed `raw_dump` fixture — it will always fall back to iCal parse and the test asserts on a `Calendar` that is correct but the fast path was never exercised.

**Phase:** Fixture generation and integration test design.

---

### Pitfall 8: `GeoSpatialCalendarIndex` contains `RTree` which has non-trivial serde behaviour

**What goes wrong:** `GeoSpatialCalendarIndex` wraps `RTree<GeomWithData<GeoPoint, InvertedCalendarIndexTerm>>`. The `rstar` crate includes `features = ["serde"]` in `redical_core/Cargo.toml`, meaning `RTree` will derive `Serialize`/`Deserialize`. However, if `Calendar` is given blanket serde derives without `#[serde(skip)]` on `indexed_geo`, the `RTree` will be serialized into the raw dump. `RTree` serde output encodes internal node structure, not just the point data. Any change to the `rstar` crate version will break deserialization of committed fixtures.

**Prevention:**
- `#[serde(skip)]` on `Calendar::indexed_geo` and `Event::indexed_geo`.
- Do not rely on `RTree` serde for the raw dump path even if it compiles.

**Warning signs:** `indexed_geo` appears in the output of `bincode::serialized_size(&calendar)` being much larger than expected; rstar upgrade causes fixture load failure.

**Phase:** Implementation of serde derives on `Calendar`.

---

### Pitfall 9: `InvertedCalendarIndex` / `InvertedEventIndex` contain `HashMap` — bincode encoding is non-deterministic across runs

**What goes wrong:** `InvertedCalendarIndexTerm` stores `events: HashMap<String, IndexedConclusion>`. `HashMap` in Rust uses a random seed by default (`HashDoS` protection). Bincode encodes the HashMap by iterating its entries — in an arbitrary order. Two serializations of the same logical index will produce different byte sequences. If index fields are not skipped, fixture comparison will be non-deterministic.

**Prevention:**
- `#[serde(skip)]` on all index fields (already required for correctness per Pitfall 1).
- If any `HashMap` is unavoidably part of the raw dump (not currently the case), replace with `BTreeMap` for deterministic ordering before serializing.

**Warning signs:** Fixture byte comparison test is flaky (passes sometimes, fails sometimes) with no code changes.

**Phase:** Implementation of serde derives; fixture generation.

---

### Pitfall 10: `redis-module` upgrade breaking changes to `RedisModuleTypeMethods`

**What goes wrong:** Upgrading `redis-module` from 2.0.2 to the latest version may add new fields to `RedisModuleTypeMethods`. The struct is initialised as a literal in `mod.rs:22-42`. If the new version adds required fields without defaults, the code will not compile. If it removes or renames fields (e.g., `copy2`, `free_effort2`), the existing initialisers will fail.

**Prevention:**
- Review the `redis-module` changelog before upgrading; check for breaking changes to `RedisModuleTypeMethods`.
- Treat the upgrade as a separate commit from the RDB format changes so any breakage is isolated.
- Run `cargo check` immediately after bumping the version before writing any new code.

**Warning signs:** `error[E0063]: missing field` or `error[E0560]: struct ... has no field named` at `mod.rs:22` after version bump.

**Phase:** `redis-module` upgrade (prerequisite step).

---

## Minor Pitfalls

### Pitfall 11: `bincode::serialize` on `RDBCalendarDump` may grow significantly with raw dump included

**What goes wrong:** `RDBCalendarDump` contains both `raw_dump: Vec<u8>` (raw bincode of `Calendar`) and `dump: RDBCalendar` (the existing iCal string representation). This means every saved calendar carries two full representations. For calendars with thousands of events, this doubles the RDB file size compared to the current single-representation approach.

**Prevention:**
- Benchmark `RDBCalendarDump` serialized size against the current `RDBCalendar` before committing to the dual-representation design.
- If size is unacceptable, consider storing only `raw_dump` in fast-path builds and falling back to a separate `RDBCalendar`-only save when `version` is `None`.

**Warning signs:** RDB file size doubles on first save with new format; Redis BGSAVE takes significantly longer.

**Phase:** Design review before implementation; benchmarking after initial implementation.

---

### Pitfall 12: `rdb_load` calling `rayon::par_iter` during Redis startup

**What goes wrong:** `rdb_data.rs` uses `rayon::prelude::par_iter` to parallelise event deserialization inside `Calendar::try_from(&rdb_calendar)`. Rayon spawns a thread pool. During Redis RDB load (startup), many calendars are loaded concurrently by Redis's own I/O. Rayon's global thread pool may be contended, and the interaction between Redis's fork-based persistence and Rayon's threads is not guaranteed safe. This is pre-existing but becomes more relevant when the fast path adds another deserialization layer.

**Prevention:**
- No immediate action needed; this is a pre-existing behaviour. Note it as a potential issue if Redis startup hangs or performance degrades after adding the fast path.
- The fast path's `catch_unwind` closure must be `Send + 'static` compatible — verify that no Rayon thread-local state escapes the closure boundary.

**Warning signs:** Redis startup time increases proportionally to number of calendars; Rayon thread pool exhaustion errors in Redis logs.

**Phase:** Integration testing; monitoring during fast-path implementation.

---

## Phase-Specific Warnings

| Phase Topic | Likely Pitfall | Mitigation |
|-------------|---------------|------------|
| `aof_rewrite` stub | Pitfall 6: `todo!()` crashes Redis on AOF rewrite | Do this first, before any other change |
| `redis-module` upgrade | Pitfall 10: breaking changes to `RedisModuleTypeMethods` | Upgrade in isolation; check changelog |
| `from_utf8_unchecked` fix | Pitfall 4: UB on every save | Resolve before `RDBCalendarDump` is written |
| serde derives on `Calendar`/`Event` | Pitfall 1, 2, 8, 9: computed/derived fields serialized | `#[serde(skip)]` on all index and cache fields |
| `ScheduleProperties` serde | Pitfall 2: `parsed_rrule_set` serialized | `#[serde(skip)]` on `parsed_rrule_set`; call `validate()` after load |
| `catch_unwind` implementation | Pitfall 3: scope too narrow, panic still crosses FFI | Wrap entire `rdb_load` body, not just bincode call |
| Fixture generation | Pitfall 5, 7: layout changes; SHA instability | Fast-path fixtures generated at test time; fallback fixtures committed |
| `RDBCalendarDump` struct design | Pitfall 5: bincode field order fragility | Document field order as serialization contract; never reorder |
| `GIT_SHA` version check | Pitfall 7: SHA changes on rebase | Do not assert fast path exercised in CI fixture tests |
| Dual-representation save | Pitfall 11: RDB size doubles | Benchmark before committing to design |

## Sources

- Direct inspection of `redical_redis/src/datatype/mod.rs` (current `rdb_load`/`rdb_save`/`aof_rewrite` implementations)
- Direct inspection of `redical_redis/src/datatype/rdb_data.rs` (RDB struct layout and bincode serialization patterns)
- Direct inspection of `redical_core/src/calendar.rs` and `redical_core/src/event.rs` (struct fields, computed indexes, `RRuleSet` cache)
- Direct inspection of `redical_core/src/inverted_index.rs` and `redical_core/src/geo_index.rs` (index types, `RTree` wrapping, `HashMap` internals)
- Direct inspection of `redical_redis/build.rs` (`GIT_SHA` generation via short SHA)
- `.planning/codebase/CONCERNS.md` (pre-identified fragile areas: `from_utf8_unchecked`, `todo!()` in `aof_rewrite`, `rdb_load`/`rdb_save` panic behaviour, `DateTime` panic)
- `.planning/PROJECT.md` (milestone requirements and constraints)
- `redical_core/Cargo.toml` and `redical_redis/Cargo.toml` (rrule serde feature, rstar serde feature, bincode 1.3.3, redis-module 2.0.2)
- Rust reference: `catch_unwind` and FFI boundary safety (HIGH confidence — compiler-enforced `UnwindSafe` bound)
- bincode 1.x documentation: no self-description, field-order encoding (HIGH confidence — version confirmed from Cargo.toml)
