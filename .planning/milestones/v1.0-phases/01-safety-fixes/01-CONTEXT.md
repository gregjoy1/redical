# Phase 1: Safety Fixes - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Close `aof_rewrite` `todo!()` crash, replace `from_utf8_unchecked` UB in `rdb_save`, and align `redis-module` Cargo.toml to 2.0.4. No user-visible behavior changes. No new capabilities — this phase purely removes crash risks and UB before RDB format work begins.

</domain>

<decisions>
## Implementation Decisions

### Upgrade order (UPGR-01 before SAFE-02)
- Upgrade `redis-module` to 2.0.4 first and audit the 2.0.3–2.0.4 changelog
- If `save_string_buffer` (or equivalent raw byte save API) is available in 2.0.4, use it for SAFE-02
- SAFE-02 is gated on the upgrade completing — the upgrade result determines which approach to take

### SAFE-02 fallback if save_string_buffer unavailable
- Replace `from_utf8_unchecked` with an explicit `unsafe` block containing a thorough `// SAFETY:` comment
- The comment must explain: Redis C API is binary-safe (bytes are stored and returned verbatim), the `&str` is only passed to `save_string` which passes the pointer+length to C, and the bytes are never inspected as UTF-8 by any Rust code
- **Critical constraint**: the fix must produce identical bytes on disk — no encoding (base64, hex, etc.) that would break existing production RDB files
- If `save_string_buffer` IS available: use it and eliminate the unsafe block entirely

### SAFE-01 (aof_rewrite stub)
- Empty function body — no `todo!()`, no panic, no logging
- Just remove the `todo!()` and leave the body blank

### Backward compatibility with production RDB files
- Existing production RDB files (bare `RDBCalendar` bincode bytes) must continue to load
- The Phase 3 three-layer dispatch already handles this via the legacy fallback path
- No changes in Phase 1 affect the binary format on disk — the fix is purely a Rust type-safety concern

</decisions>

<specifics>
## Specific Ideas

- The existing comment `// no save_string_buffer available in redis-module :(` is the starting point for the SAFE-02 investigation — check whether 2.0.4 closes this gap
- Production is live and existing `dump.rdb` files must rehydrate correctly — no encoding changes

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `redical_redis/src/datatype/mod.rs`: contains all three targets — `rdb_load`, `rdb_save`, `aof_rewrite`
- `redical_redis/build.rs`: already handles `GIT_SHA` and other build-time env vars — no changes needed here

### Established Patterns
- `raw::load_string_buffer(rdb)` already exists on the load side — the save-side equivalent (`save_string_buffer`) is the expected counterpart in newer redis-module versions
- `unsafe extern "C"` function signatures are the established pattern for Redis module callbacks

### Integration Points
- `redical_redis/Cargo.toml`: `redis-module = "2.0.2"` → `"2.0.4"` (one line)
- `aof_rewrite` at `mod.rs:85` — remove `todo!()`, leave empty body

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 01-safety-fixes*
*Context gathered: 2026-03-06*
