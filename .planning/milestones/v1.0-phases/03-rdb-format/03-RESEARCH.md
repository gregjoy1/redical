# Phase 3: RDB Format - Research

**Researched:** 2026-03-06
**Domain:** Rust RDB serialization, bincode, catch_unwind panic safety
**Confidence:** HIGH

## Summary

Phase 3 adds the `RDBCalendarDump` envelope struct to `rdb_data.rs`, rewrites `rdb_save` to produce a dual-representation blob (raw bincode + iCal fallback), and rewrites `rdb_load` with three-layer dispatch plus `catch_unwind` panic safety on the fast path. All required libraries (bincode 1.3.3, serde, redis-module 2.0.4) are already workspace dependencies. The Phase 2 serde derive chain is complete so `Calendar` is bincode-serializable.

The critical technical concern is correct `catch_unwind` scoping: it must wrap both `bincode::deserialize` of `raw_dump` AND `rebuild_indexes()` within a single closure. The `rdb` IO pointer from the extern C callback is NOT passed into the catch_unwind closure (it's not UnwindSafe); logging uses the context-free `redis_module::logging::log_warning()` functions instead.

**Primary recommendation:** Implement in two waves -- (1) RDBCalendarDump struct + rdb_save, (2) rdb_load three-layer dispatch with catch_unwind.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Log warning on ALL fallback events using `raw::log_warning` directly (no helper abstraction)
- Log message format: path taken + reason, e.g. "RDB load: fast path skipped (version build digest mismatch: abc123 vs def456), using iCal fallback"
- Log at debug level on successful fast-path load, e.g. "RDB load: fast path OK"
- Include panic payload in catch_unwind fallback log, e.g. "RDB load: fast path panicked (payload: '...'), using iCal fallback"
- Log at info level when falling through to legacy path: "RDB calendar load: not current format, trying legacy"
- Log error on raw::load_string_buffer failure before returning null
- Error handling hierarchy in rdb_load (see CONTEXT.md for full hierarchy)
- Keep all panics in rdb_save -- if Calendar can't serialize, something is fundamentally broken
- Use a named const or Option<&str> static for option_env!("GIT_SHA")

### Claude's Discretion
- Exact function decomposition within rdb_load (helper functions vs inline logic)
- Whether to extract a load_from_dump / load_legacy helper or keep dispatch inline
- Log message exact wording (as long as it includes path + reason)
- RDBCalendarDump derive list (Serialize, Deserialize + whatever else is needed)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| RDB-01 | `RDBCalendarDump` struct with version, raw_dump, dump fields | Struct definition with Serialize/Deserialize derives; placed in rdb_data.rs alongside existing RDBCalendar |
| RDB-02 | `rdb_save` writes RDBCalendarDump with GIT_SHA version, bincode of Calendar as raw_dump, RDBCalendar as dump | option_env! for GIT_SHA; bincode::serialize for both Calendar and envelope; raw::save_slice for output |
| RDB-03 | `rdb_load` three-layer dispatch: envelope -> legacy -> panic | bincode::deserialize attempts; version comparison with BUILD_VERSION const |
| RDB-04 | Fast-path wrapped in catch_unwind with AssertUnwindSafe | std::panic::catch_unwind + AssertUnwindSafe; must wrap both deserialize and rebuild_indexes |
| RDB-05 | rebuild_indexes() called after fast-path deserialization | Calendar::rebuild_indexes() returns Result<bool, String>; must be inside catch_unwind scope |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bincode | 1.3.3 | Binary serialization of Calendar and RDBCalendarDump | Already in workspace; used for existing RDBCalendar path |
| serde | 1.0.162 | Derive Serialize/Deserialize | Already in workspace; Phase 2 added derives to full Calendar graph |
| redis-module | 2.0.4 | Redis module FFI, raw IO, logging | Already in workspace; provides raw::save_slice, raw::load_string_buffer, logging:: functions |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::panic | stdlib | catch_unwind + AssertUnwindSafe | Fast-path deserialization safety |
| libc | 0.2 | c_void for FFI return types | Already imported |

No new dependencies needed.

## Architecture Patterns

### Target File Structure
```
redical_redis/src/datatype/
  mod.rs         # rdb_load, rdb_save (modified)
  rdb_data.rs    # RDBCalendarDump (new), RDBCalendar (existing)
```

### Pattern 1: RDBCalendarDump Envelope
**What:** New struct wrapping both fast-path and fallback data.
**Fields:**
```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct RDBCalendarDump {
    pub version:  Option<String>,
    pub raw_dump: Vec<u8>,
    pub dump:     RDBCalendar,
}
```
**Notes:**
- `version` is `Option<String>` -- None when GIT_SHA env var absent at build time
- `raw_dump` is bincode of `Calendar` (the core struct with serde derives from Phase 2)
- `dump` is the existing iCal-based RDBCalendar (always valid, always parseable)

### Pattern 2: Build Version Constant
**What:** Named constant for compile-time GIT_SHA.
```rust
const BUILD_VERSION: Option<&str> = option_env!("GIT_SHA");
```
**Where:** Top of `mod.rs` (or `rdb_data.rs`, discretionary).
**Why:** `option_env!` resolves at compile time. Named constant is clearer than inline macro usage. Returns `None` when env var absent -- fast path is always skipped.

### Pattern 3: Three-Layer rdb_load Dispatch
**What:** Ordered deserialization attempts with fallback chain.
```
1. Try bincode::deserialize::<RDBCalendarDump>(bytes)
   OK  -> check version, attempt fast path or use iCal dump
   Err -> try legacy path (step 2)

2. Try bincode::deserialize::<RDBCalendar>(bytes)
   OK  -> Calendar::try_from(&rdb_calendar) (existing iCal path)
   Err -> panic (truly corrupted)

3. Fast path (inside catch_unwind):
   - bincode::deserialize::<Calendar>(&envelope.raw_dump)
   - calendar.rebuild_indexes()
   - On panic/Err -> fall back to envelope.dump (iCal path)
```

### Pattern 4: catch_unwind Scope
**What:** Wrap fast-path deser + rebuild_indexes in a single catch_unwind.
```rust
use std::panic::{catch_unwind, AssertUnwindSafe};

let fast_path_result = catch_unwind(AssertUnwindSafe(|| {
    let mut calendar: Calendar = bincode::deserialize(&envelope.raw_dump)?;
    calendar.rebuild_indexes().map_err(|e| /* convert */)?;
    Ok(calendar)
}));

match fast_path_result {
    Ok(Ok(calendar)) => { /* success, log debug */ },
    Ok(Err(err))     => { /* deser/rebuild error, log warning, fall back */ },
    Err(panic_info)  => { /* panic caught, log warning with payload, fall back */ },
}
```
**Critical:** `AssertUnwindSafe` is required because `Vec<u8>` slice refs and the closure capture aren't automatically `UnwindSafe`. This is safe here because on panic we discard all captured state and fall back to the iCal path.

### Pattern 5: Panic Payload Extraction
**What:** Extract human-readable message from catch_unwind Err payload.
```rust
Err(panic_payload) => {
    let message = if let Some(s) = panic_payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    };
    // use message in log
}
```

### Anti-Patterns to Avoid
- **catch_unwind too narrow:** Must include rebuild_indexes() -- a panic in index construction crosses FFI boundary if not caught.
- **Passing `rdb` pointer into catch_unwind closure:** `*mut RedisModuleIO` is not UnwindSafe and should not be used inside the closure. Do all logging after the catch_unwind returns.
- **Using `unwrap()` on bincode::deserialize in fast path:** The whole point is graceful fallback; errors and panics are expected on version mismatch.

## Logging API

**Important finding:** The CONTEXT.md says "use `raw::log_warning` directly." However, the actual redis-module 2.0.4 API for context-free logging is:

```rust
use redis_module::logging;

logging::log_warning("message");   // WARNING level
logging::log_debug("message");     // DEBUG level
logging::log_notice("message");    // NOTICE level
```

There is also `logging::log_io_error(rdb, LogLevel::Warning, "message")` which takes the IO handle -- potentially better for rdb_load/rdb_save since Redis associates the log with the IO operation. However, this should NOT be used inside catch_unwind (the rdb pointer isn't UnwindSafe).

**Recommended approach:**
- Inside catch_unwind: no logging (return result/error)
- After catch_unwind match: use `logging::log_warning()` / `logging::log_debug()` for the context-free variants
- For load_string_buffer failure: `logging::log_io_error(rdb, LogLevel::Warning, ...)` is available but `logging::log_warning()` also works

**Note:** Redis log levels don't have "info" -- closest is "notice" or "verbose". The CONTEXT.md says "log at info level when falling through to legacy path." Map this to `logging::log_notice()`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Binary serialization | Custom byte packing | bincode 1.3.3 | Already proven in existing RDBCalendar path |
| Panic catching | Signal handlers or custom abort hooks | std::panic::catch_unwind | Standard Rust mechanism; catches unwind panics |
| Version detection | Runtime git commands | option_env!("GIT_SHA") | Compile-time resolution, zero runtime cost, build.rs already sets it |
| Redis logging | eprintln or custom loggers | redis_module::logging | Goes through Redis log infrastructure |

## Common Pitfalls

### Pitfall 1: catch_unwind Scope Too Narrow
**What goes wrong:** Wrapping only bincode::deserialize but not rebuild_indexes -- a panic in index construction crosses FFI and crashes Redis.
**Why it happens:** Natural instinct is to wrap "the risky call."
**How to avoid:** Single catch_unwind closure wraps deserialize + rebuild_indexes + any validation.
**Warning signs:** Any `unwrap()` or `panic!` path outside the catch_unwind scope on the fast path.

### Pitfall 2: Passing rdb Pointer Into catch_unwind
**What goes wrong:** Compiler error or undefined behavior -- `*mut RedisModuleIO` is not UnwindSafe.
**How to avoid:** Clone/copy any needed data before the closure. Log after the closure returns using the non-IO logging API.

### Pitfall 3: bincode Format Ordering Sensitivity
**What goes wrong:** bincode 1.x serializes structs by field order. Adding/reordering fields in Calendar between versions produces incompatible bytes.
**Why it happens:** bincode has no schema versioning.
**How to avoid:** This is exactly why the version check exists. Mismatched GIT_SHA -> skip fast path -> iCal fallback always works.

### Pitfall 4: option_env! Returns None in Tests
**What goes wrong:** Tests run without GIT_SHA set, so BUILD_VERSION is None, fast path is always skipped.
**How to avoid:** This is correct behavior per RDB-02. Unit tests of fast-path logic can be structured to test the inner function directly rather than relying on version matching. Phase 4 handles test fixtures.

### Pitfall 5: CALENDAR_DATA_TYPE_VERSION
**What goes wrong:** Forgetting to consider whether Redis needs the type version incremented for format changes.
**Why it matters:** Redis uses this version as encver parameter in rdb_load. The current code ignores _encver.
**How to avoid:** Since rdb_load already handles format detection via bincode deserialization attempts (not encver), and the envelope is backward-compatible (legacy path exists), incrementing is optional. However, if incremented, old Redis instances can't load new RDB files at all (Redis rejects higher encver). Recommendation: keep at 1 for backward compatibility.

## Code Examples

### RDBCalendarDump Struct Definition
```rust
// In rdb_data.rs
#[derive(Serialize, Deserialize, Debug)]
pub struct RDBCalendarDump {
    pub version:  Option<String>,
    pub raw_dump: Vec<u8>,
    pub dump:     RDBCalendar,
}
```

### Build Version Constant
```rust
// In mod.rs (top of file)
const BUILD_VERSION: Option<&str> = option_env!("GIT_SHA");
```

### rdb_save (Updated)
```rust
pub unsafe extern "C" fn rdb_save(rdb: *mut raw::RedisModuleIO, value: *mut c_void) {
    let calendar = unsafe { &*(value as *mut Calendar) };

    let raw_dump = bincode::serialize(calendar).unwrap();

    let rdb_calendar = RDBCalendar::try_from(calendar).unwrap();

    let envelope = RDBCalendarDump {
        version:  BUILD_VERSION.map(String::from),
        raw_dump,
        dump:     rdb_calendar,
    };

    let bytes = bincode::serialize(&envelope).unwrap();

    raw::save_slice(rdb, &bytes);
}
```

### rdb_load (Updated - Sketch)
```rust
pub extern "C" fn rdb_load(rdb: *mut raw::RedisModuleIO, _encver: c_int) -> *mut c_void {
    let Ok(buffer) = raw::load_string_buffer(rdb) else {
        logging::log_warning("RDB calendar load: failed to read string buffer");
        return null_mut();
    };

    let bytes: &[u8] = buffer.as_ref();

    // Layer 1: Try new envelope format
    let calendar = match bincode::deserialize::<RDBCalendarDump>(bytes) {
        Ok(envelope) => load_from_envelope(envelope),

        Err(_) => {
            // Layer 2: Try legacy bare RDBCalendar
            logging::log_notice("RDB calendar load: not current format, trying legacy");
            load_legacy(bytes)
        }
    };

    Box::into_raw(Box::new(calendar)).cast::<c_void>()
}
```

### Fast Path With catch_unwind
```rust
fn load_from_envelope(envelope: RDBCalendarDump) -> Calendar {
    let version_matches = match (BUILD_VERSION, envelope.version.as_deref()) {
        (Some(current), Some(saved)) => current == saved,
        _ => false,
    };

    if version_matches {
        let raw_dump = envelope.raw_dump;

        let fast_result = catch_unwind(AssertUnwindSafe(|| -> Result<Calendar, String> {
            let mut calendar: Calendar = bincode::deserialize(&raw_dump)
                .map_err(|e| format!("{e}"))?;

            calendar.rebuild_indexes()
                .map_err(|e| format!("{e}"))?;

            Ok(calendar)
        }));

        match fast_result {
            Ok(Ok(calendar)) => {
                logging::log_debug("RDB load: fast path OK");
                return calendar;
            }

            Ok(Err(error)) => {
                logging::log_warning(
                    &format!("RDB load: fast path failed ({error}), using iCal fallback")
                );
            }

            Err(panic_payload) => {
                let message = extract_panic_message(&panic_payload);
                logging::log_warning(
                    &format!("RDB load: fast path panicked (payload: '{message}'), using iCal fallback")
                );
            }
        }
    } else {
        let current = BUILD_VERSION.unwrap_or("None");
        let saved = envelope.version.as_deref().unwrap_or("None");
        logging::log_warning(
            &format!("RDB load: fast path skipped (version build digest mismatch: {saved} vs {current}), using iCal fallback")
        );
    }

    // iCal fallback from envelope.dump
    Calendar::try_from(&envelope.dump).unwrap_or_else(|error| {
        panic!("RDB load: iCal fallback failed: {error}");
    })
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bare RDBCalendar bincode blob | RDBCalendarDump envelope with dual representation | This phase | Fast path for same-version, safe fallback for mismatches |
| No panic safety in rdb_load | catch_unwind on fast path | This phase | Redis process survives corrupt/mismatched binary data |
| No version tracking in RDB | GIT_SHA embedded in serialized data | This phase | Version-gated fast path avoids deserializing stale bincode |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo test (Rust built-in) |
| Config file | Cargo.toml (workspace) |
| Quick run command | `cargo test -p redical_redis -- --test-threads=1` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| RDB-01 | RDBCalendarDump struct exists with correct fields | unit | `cargo test -p redical_redis rdb_data -- --test-threads=1` | Wave 0 |
| RDB-02 | rdb_save produces RDBCalendarDump bytes | unit | `cargo test -p redical_redis rdb_save -- --test-threads=1` | Wave 0 |
| RDB-03 | rdb_load three-layer dispatch | unit | `cargo test -p redical_redis rdb_load -- --test-threads=1` | Wave 0 |
| RDB-04 | catch_unwind catches fast-path panic | unit | `cargo test -p redical_redis catch_unwind -- --test-threads=1` | Wave 0 |
| RDB-05 | rebuild_indexes called after fast-path deser | unit | `cargo test -p redical_redis rebuild -- --test-threads=1` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p redical_redis -- --test-threads=1`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before /gsd:verify-work

### Wave 0 Gaps
- [ ] RDBCalendarDump round-trip test (serialize envelope, deserialize, verify fields)
- [ ] Version mismatch falls back to iCal path (unit test)
- [ ] Existing tests in rdb_data.rs pass with new struct added

Note: Phase 4 handles integration test fixtures (TEST-01 through TEST-06). Phase 3 tests focus on unit-level correctness of the new code paths.

## Open Questions

1. **Logging API: `raw::log_warning` vs `logging::log_warning`**
   - What we know: CONTEXT.md says "use `raw::log_warning` directly" but this function doesn't exist in redis-module 2.0.4. The actual API is `redis_module::logging::log_warning()`.
   - Recommendation: Use `redis_module::logging::log_warning()` / `log_debug()` / `log_notice()`. The user intent was "no helper abstraction" which is honored -- these are direct calls.

2. **Redis "info" log level mapping**
   - What we know: CONTEXT.md says "log at info level when falling through to legacy path." Redis log levels are: debug, verbose, notice, warning. No "info" level.
   - Recommendation: Map "info" to `log_notice()` (closest equivalent).

## Sources

### Primary (HIGH confidence)
- redis-module 2.0.4 source at ~/.cargo/registry -- logging API, raw::save_slice, raw::load_string_buffer verified
- redical_redis/src/datatype/mod.rs -- current rdb_load/rdb_save implementation
- redical_redis/src/datatype/rdb_data.rs -- existing RDBCalendar struct and TryFrom impls
- redical_redis/build.rs -- GIT_SHA env var setup confirmed
- Rust stdlib std::panic::catch_unwind -- AssertUnwindSafe usage (HIGH confidence, compiler-enforced)

### Secondary (MEDIUM confidence)
- bincode 1.3.3 panic behavior on malformed input -- documented in prior project research (.planning/research/STACK.md)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all dependencies already in workspace, verified in source
- Architecture: HIGH -- dispatch pattern well-defined in CONTEXT.md, code structure inspected
- Pitfalls: HIGH -- catch_unwind scoping and FFI boundary concerns are well-documented in Rust ecosystem and prior project research
- Logging API: HIGH -- verified directly in redis-module 2.0.4 source code

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain, no moving targets)
