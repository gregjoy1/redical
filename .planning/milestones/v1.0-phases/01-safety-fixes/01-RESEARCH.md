# Phase 1: Safety Fixes - Research

**Researched:** 2026-03-06
**Domain:** Rust / Redis module FFI safety
**Confidence:** HIGH

---

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

- Upgrade `redis-module` to 2.0.4 first (UPGR-01 before SAFE-02)
- Audit 2.0.3–2.0.4 changelog to check `save_string_buffer` (or equivalent) availability
- SAFE-02 approach is gated on upgrade result: use raw byte API if available, else safe `unsafe` block with thorough `// SAFETY:` comment
- SAFE-02 fallback comment must explain: Redis C API is binary-safe; bytes passed pointer+length to C; never inspected as UTF-8 by Rust
- SAFE-02 fix must produce identical bytes on disk — no encoding (base64, hex, etc.)
- SAFE-01: empty function body — remove `todo!()`, leave body blank, no logging

### Claude's Discretion

None specified.

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope.
</user_constraints>

---

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SAFE-01 | Replace `aof_rewrite` `todo!()` with empty no-op stub | Confirmed: empty body is valid for `unsafe extern "C"` functions; Redis won't crash on AOF rewrite |
| SAFE-02 | Replace `from_utf8_unchecked` in `rdb_save` with safe alternative | Key finding: `raw::save_slice(&[u8])` already exists in 2.0.2 and 2.0.4 — no unsafe block needed at all |
| UPGR-01 | Bump `redis-module` in `Cargo.toml` from `2.0.2` to `2.0.4` | Lockfile already resolves 2.0.4; Cargo.toml and workspace are the only lines to change |
</phase_requirements>

---

## Summary

Phase 1 is three precise, low-risk edits to a single file (`redical_redis/src/datatype/mod.rs`) plus one version string change in `Cargo.toml`. The entire surface area is fully known before planning begins.

The most important research finding is that `raw::save_slice` — which takes `&[u8]` directly — already exists in `redis-module` 2.0.2. The comment in the code (`// no save_string_buffer available in redis-module :(`) was either an error or referred to a differently named function. In any case, the upgrade to 2.0.4 is not a blocker for SAFE-02: `raw::save_slice` is available now and is the correct replacement for the `from_utf8_unchecked` pattern.

The build currently succeeds (`cargo build` finishes without errors) and all 75 tests pass. No test infrastructure gaps exist for the changes in this phase — the changes are too small to warrant unit tests beyond verifying `cargo build` succeeds with no warnings from the changed files.

**Primary recommendation:** Apply all three fixes in a single commit — UPGR-01 (Cargo.toml version bump), SAFE-01 (empty `aof_rewrite` body), SAFE-02 (replace `from_utf8_unchecked` with `raw::save_slice`) — then verify `cargo build` and `cargo test` are clean.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| redis-module | 2.0.4 | Rust bindings for Redis Modules C API | Project's primary Redis FFI layer |
| bincode | 1.3.3 | Binary serialisation of `RDBCalendar` | Already used for RDB serialise/deserialise |

### Supporting

No additional libraries needed for this phase.

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `raw::save_slice` | `raw::save_string` with SAFETY comment | `save_slice` is fully safe Rust — preferred |
| `raw::save_slice` | `raw::save_redis_string` | Requires constructing a `RedisModuleString`; unnecessary indirection |

**Installation:** No new dependencies. Version bump only:

```bash
# In redical_redis/Cargo.toml — change one line
redis-module = "2.0.4"

# In Cargo.toml (workspace) — change one line
redis-module = "2.0.4"
```

---

## Architecture Patterns

### Files Touched

```
redical_redis/
├── Cargo.toml              # UPGR-01: "2.0.2" → "2.0.4"
└── src/datatype/
    └── mod.rs              # SAFE-01 (line 90) + SAFE-02 (line 80)
Cargo.toml                  # UPGR-01: workspace redis-module version
```

### Pattern 1: Empty `unsafe extern "C"` callback

**What:** Redis module callbacks registered via `RedisModuleTypeMethods` must have the correct `unsafe extern "C"` signature. An empty body is valid — Redis calls it, nothing happens, no crash.

**When to use:** When a callback is required by the API contract but the feature is not yet implemented (AOF rewrite deferred to v2).

**Example:**

```rust
// Source: redical_redis/src/datatype/mod.rs (after fix)
unsafe extern "C" fn aof_rewrite(
    _aof: *mut RedisModuleIO,
    _key: *mut RedisModuleString,
    _value: *mut c_void,
) {
    // no-op: AOF rewrite not yet implemented
}
```

### Pattern 2: Save raw bytes with `raw::save_slice`

**What:** `raw::save_slice(rdb, &[u8])` writes a byte buffer to the RDB stream. The corresponding load is `raw::load_string_buffer(rdb)` which returns `Result<RedisBuffer, Error>`. This is the symmetric pair — no unsafe code required on the save side.

**When to use:** Whenever the value being persisted is binary (not guaranteed valid UTF-8).

**Example:**

```rust
// Source: docs.rs/redis-module/2.0.4/redis_module/raw/fn.save_slice.html
// Before (UB):
let str = std::str::from_utf8_unchecked(&bytes[..]);
raw::save_string(rdb, str);

// After (safe):
raw::save_slice(rdb, &bytes);
```

### Anti-Patterns to Avoid

- **`from_utf8_unchecked` on arbitrary bytes:** Bincode output is not guaranteed UTF-8. Passing non-UTF-8 bytes through a `&str` is undefined behaviour in Rust, even if the C API treats the buffer as opaque bytes.
- **Encoding bytes before saving (base64/hex):** Breaks RDB backward compatibility — existing `dump.rdb` files store raw bincode bytes. Never encode.
- **Logging inside `aof_rewrite`:** Adds a Redis module log call that could panic or have unexpected side-effects; empty body is safer.

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Save `&[u8]` to RDB | Custom FFI call to `RedisModule_SaveStringBuffer` | `raw::save_slice` | Already wrapped, safe Rust, no FFI glue needed |
| Load `&[u8]` from RDB | Custom FFI call | `raw::load_string_buffer` | Already used on load side; symmetric |

**Key insight:** The redis-module crate already wraps all required Redis C API persistence functions. There is no need to reach into `redis_sys` directly.

---

## Common Pitfalls

### Pitfall 1: Cargo.toml vs workspace Cargo.toml

**What goes wrong:** Bumping only `redical_redis/Cargo.toml` leaves the workspace root `Cargo.toml` at `"2.0.2"`, which can cause confusion if other workspace members share the workspace dependency.

**Why it happens:** Two separate version strings exist — one in the workspace root `[workspace.dependencies]` and one in `redical_redis/Cargo.toml`. The redis-module crate is declared in both.

**How to avoid:** Update both files in the same commit. The lockfile already resolves 2.0.4, so `cargo build` will succeed either way, but the `Cargo.toml` strings should be consistent.

**Warning signs:** `cargo tree` shows redis-module 2.0.2 alongside 2.0.4 after the bump.

### Pitfall 2: `save_slice` vs `save_string` bytes-on-disk identity

**What goes wrong:** Assuming `save_slice` and `save_string` write different wire formats, which would break existing `dump.rdb` files.

**Why it happens:** Concern about whether Redis stores a length prefix differently for string vs buffer saves.

**How to avoid:** Both `save_string` and `save_slice` call `RedisModule_SaveStringBuffer` under the hood (the C API has only one string-save primitive). The bytes written to disk are identical — the Rust wrapper just skips the UTF-8 validity assertion.

**Confidence:** MEDIUM — inferred from docs.rs source links; verifiable by reading redis-module source at GitHub if needed.

### Pitfall 3: Forgetting the `raw::` prefix import

**What goes wrong:** `save_slice` is used in `mod.rs` but `raw` is already imported via `use redis_module::{..., raw, ...}`. No import change is needed — `raw::save_slice` works as-is.

**How to avoid:** Check existing `use` statement before adding imports. The current import block already covers `raw`.

---

## Code Examples

### SAFE-01: Empty `aof_rewrite`

```rust
// redical_redis/src/datatype/mod.rs line ~85 (after fix)
unsafe extern "C" fn aof_rewrite(
    _aof: *mut RedisModuleIO,
    _key: *mut RedisModuleString,
    _value: *mut c_void,
) {
}
```

### SAFE-02: Replace `from_utf8_unchecked` with `save_slice`

```rust
// redical_redis/src/datatype/mod.rs rdb_save function (after fix)
pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let rdb_calendar = match RDBCalendar::try_from(calendar) {
        Ok(rdb_calendar) => rdb_calendar,

        Err(error) => {
            panic!("rdb_save failed for Calendar with error: {error:#?}");
        },
    };

    let bytes: Vec<u8> = bincode::serialize(&rdb_calendar).unwrap();

    raw::save_slice(rdb, &bytes);
}
```

### UPGR-01: Version bump

```toml
# redical_redis/Cargo.toml
redis-module = "2.0.4"
redis-module-macros = "2.0.4"

# Cargo.toml (workspace root)
redis-module-macros = "2.0.4"
redis-module = "2.0.4"
```

---

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| `from_utf8_unchecked` + `save_string` | `save_slice` directly | Eliminates UB; identical bytes on disk |
| `todo!()` in `aof_rewrite` | Empty body | No panic on Redis AOF rewrite |

**Deprecated/outdated:**
- The comment `// no save_string_buffer available in redis-module :(` is incorrect — `raw::save_slice` provides the same capability and was available since at least 2.0.2. Remove the comment.

---

## Open Questions

1. **`redis-module-macros` version alignment**
   - What we know: `Cargo.toml` has `redis-module-macros = "2.0.2"` in both `redical_redis/Cargo.toml` and workspace root
   - What's unclear: Whether `redis-module-macros` should also be bumped to `2.0.4` for consistency, or if 2.0.2 is the latest for macros
   - Recommendation: Check crates.io for latest `redis-module-macros` version. If 2.0.4 exists, bump both in the same commit. If not, leave at 2.0.2 and note the discrepancy.

---

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust built-in (`cargo test`) |
| Config file | none (uses Cargo defaults) |
| Quick run command | `cargo build --package redical_redis 2>&1 \| grep -E "^error"` |
| Full suite command | `cargo test` |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SAFE-01 | `aof_rewrite` is empty no-op, no panic | smoke | `cargo build` succeeds + no `todo` in aof_rewrite | N/A — compile-time guarantee |
| SAFE-02 | `from_utf8_unchecked` absent from changed files | smoke | `cargo build --package redical_redis 2>&1 \| grep -c from_utf8_unchecked` returns 0 | N/A — compile-time |
| UPGR-01 | `redis-module` version in Cargo.toml = 2.0.4 | smoke | `grep 'redis-module' redical_redis/Cargo.toml` | N/A — file check |

All three requirements are verified by `cargo build` succeeding plus a grep confirming `from_utf8_unchecked` is absent. No new test files required.

### Sampling Rate

- **Per task commit:** `cargo build 2>&1 | grep -E "^error"` — must be empty
- **Per wave merge:** `cargo test`
- **Phase gate:** `cargo test` green before `/gsd:verify-work`

### Wave 0 Gaps

None — existing test infrastructure covers all phase requirements.

---

## Sources

### Primary (HIGH confidence)

- docs.rs/redis-module/2.0.2/redis_module/raw — confirmed `save_slice` exists in 2.0.2
- docs.rs/redis-module/2.0.4/redis_module/raw — confirmed `save_slice` exists in 2.0.4
- docs.rs/redis-module/2.0.2/redis_module/raw/fn.save_slice.html — signature `(rdb: *mut RedisModuleIO, buf: &[u8])`
- docs.rs/redis-module/2.0.2/redis_module/raw/fn.load_string_buffer.html — confirmed symmetric load side
- Cargo.lock (local) — confirms 2.0.4 already resolved in lockfile

### Secondary (MEDIUM confidence)

- `cargo build` and `cargo test` run locally — 75 tests pass, build clean
- `redical_redis/src/datatype/mod.rs` (local) — exact line numbers and current code confirmed

---

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH — verified from docs.rs
- Architecture: HIGH — reading actual source file
- Pitfalls: MEDIUM — `save_slice` wire-format identity inferred from C API docs, not redis-module source

**Research date:** 2026-03-06
**Valid until:** 2026-09-06 (redis-module API is stable; `save_slice` won't be removed)
