# Phase 3: RDB Format - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Implement `RDBCalendarDump` envelope struct, update `rdb_save` to write dual-representation (raw bincode + iCal fallback), and update `rdb_load` with three-layer dispatch (new envelope → legacy format → panic) plus `catch_unwind` panic safety on the fast path. No test fixtures — Phase 4 handles those.

</domain>

<decisions>
## Implementation Decisions

### Fallback logging
- Log warning on ALL fallback events using `raw::log_warning` directly (no helper abstraction)
- Log message format: path taken + reason, e.g. "RDB load: fast path skipped (version build digest mismatch: abc123 vs def456), using iCal fallback"
- Log at debug level on successful fast-path load, e.g. "RDB load: fast path OK"
- Include panic payload in catch_unwind fallback log, e.g. "RDB load: fast path panicked (payload: '...'), using iCal fallback"
- Log at info level when falling through to legacy path: "RDB calendar load: not current format, trying legacy"
- Log error on raw::load_string_buffer failure before returning null

### Error handling hierarchy in rdb_load
- `raw::load_string_buffer` fails → log error + return null_mut (Redis-level issue)
- `RDBCalendarDump` bincode deser fails → log info, try legacy RDBCalendar (expected for old data)
- Legacy `RDBCalendar` bincode deser fails → panic (truly corrupted bytes, nothing can help)
- Fast-path bincode deser of `raw_dump` panics/fails → catch_unwind catches it, log warning, fall back to `dump` (iCal path)
- `rebuild_indexes()` panics/fails after fast-path deser → same catch_unwind scope, log warning, fall back to iCal path
- iCal parse (`Calendar::try_from(&rdb_calendar)`) fails → panic (real bug, corrupted source data that previously saved successfully)

### rdb_save hardening
- Keep all panics in rdb_save — if an in-memory Calendar can't serialize, something is fundamentally broken
- Panic if raw_dump (bincode of Calendar) serialization fails — this should never happen for valid in-memory data
- Panic if RDBCalendar::try_from fails — same reasoning

### GIT_SHA version access
- Use a named `const` or `Option<&str>` static for `option_env!("GIT_SHA")` — clearer than inline macro calls
- `option_env!` resolves at compile time so no runtime overhead, but a named constant improves readability

### Claude's Discretion
- Exact function decomposition within rdb_load (helper functions vs inline logic)
- Whether to extract a `load_from_dump` / `load_legacy` helper or keep dispatch inline
- Log message exact wording (as long as it includes path + reason)
- `RDBCalendarDump` derive list (Serialize, Deserialize + whatever else is needed)

</decisions>

<specifics>
## Specific Ideas

- "build digest mismatch" preferred over "version mismatch" in log messages — clearer what's being compared
- catch_unwind must wrap rebuild_indexes() too, not just the bincode call (decided in Phase 2 context / STATE.md)

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `raw::save_slice(rdb, &bytes)` — already used in rdb_save for writing bytes (Phase 1 replaced from_utf8_unchecked)
- `raw::load_string_buffer(rdb)` — already used in rdb_load for reading bytes
- `Calendar::rebuild_indexes()` — must be called after any deserialization (Phase 2)
- `RDBCalendar` / `RDBEvent` / `RDBEventOccurrenceOverride` — existing iCal-based serialization structs with TryFrom impls
- `bincode::serialize` / `bincode::deserialize` — already workspace dependencies
- `redical_redis/build.rs` — already sets `GIT_SHA` env var via `git rev-parse --short HEAD`

### Established Patterns
- `extern "C" fn` signatures for Redis module callbacks — must be maintained
- `Box::into_raw(Box::new(calendar)).cast::<c_void>()` for returning Calendar to Redis
- `null_mut()` return for load failure (Redis treats as "key doesn't exist")
- Rayon parallelization in RDBCalendar → Calendar conversion (par_iter on events)

### Integration Points
- `redical_redis/src/datatype/mod.rs` lines 45-81 — rdb_load and rdb_save are the main targets
- `redical_redis/src/datatype/rdb_data.rs` — RDBCalendarDump struct goes here, alongside existing RDBCalendar
- `use rdb_data::RDBCalendar` import at line 14 — will need RDBCalendarDump added
- CALENDAR_DATA_TYPE_VERSION (line 17) — may need incrementing if Redis requires it for format changes

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 03-rdb-format*
*Context gathered: 2026-03-06*
