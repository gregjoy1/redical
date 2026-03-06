---
phase: 03-rdb-format
plan: 02
subsystem: database
tags: [bincode, serde, rdb, redis-module, catch-unwind, panic-safety]

requires:
  - phase: 03-rdb-format
    provides: RDBCalendarDump envelope struct and rdb_save producing envelope format
provides:
  - Three-layer rdb_load dispatch (envelope -> legacy -> panic)
  - catch_unwind panic safety on fast-path bincode deserialization
  - Version-gated fast path (BUILD_VERSION match required)
  - Safe logging wrappers for test-mode compatibility
affects: [04-test-fixtures]

tech-stack:
  added: []
  patterns: [catch_unwind for panic-safe fast path, test-safe logging wrappers]

key-files:
  created: []
  modified:
    - redical_redis/src/datatype/mod.rs

key-decisions:
  - "Thin log wrapper module to no-op redis logging in test mode (upstream cfg!(test) only applies within redis-module crate)"
  - "load_from_envelope and load_legacy as pub(crate) helpers for direct unit testing without Redis IO handle"

patterns-established:
  - "Three-layer dispatch: envelope -> legacy -> panic in rdb_load"
  - "catch_unwind wrapping bincode deser + rebuild_indexes in single closure"
  - "Version match gating: both BUILD_VERSION and envelope.version must be Some and equal"

requirements-completed: [RDB-03, RDB-04, RDB-05]

duration: 6min
completed: 2026-03-06
---

# Phase 3 Plan 2: rdb_load Three-Layer Dispatch Summary

**Three-layer rdb_load with catch_unwind panic safety: envelope fast path (version-gated bincode) -> iCal fallback -> legacy RDBCalendar compat**

## Performance

- **Duration:** 6 min
- **Started:** 2026-03-06T16:08:53Z
- **Completed:** 2026-03-06T16:15:18Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- rdb_load rewritten with three-layer dispatch: RDBCalendarDump envelope first, legacy RDBCalendar fallback, panic on true corruption
- Fast path gated on BUILD_VERSION match, wrapped in catch_unwind covering both bincode deser and rebuild_indexes
- Panic payload extraction and logging on fast-path failure (downcast to &str, String, or "unknown panic")
- Three unit tests covering envelope iCal fallback, legacy path, and corrupted raw_dump fallback

## Task Commits

Each task was committed atomically:

1. **Task 1: Rewrite rdb_load with three-layer dispatch and catch_unwind** - `e11c080` (feat)
2. **Task 2: Unit tests for rdb_load dispatch paths** - `1e2e12d` (test)

## Files Created/Modified
- `redical_redis/src/datatype/mod.rs` - Three-layer rdb_load dispatch, load_from_envelope/load_legacy helpers, log wrapper module, 3 unit tests

## Decisions Made
- Added thin `log` wrapper module since redis-module's `cfg!(test)` guard only applies within its own crate, not dependents -- logging functions would panic on `unwrap()` of uninitialized `RedisModule_Log` function pointer
- Made `load_from_envelope` and `load_legacy` `pub(crate)` for direct unit testing without needing a Redis IO handle
- Event `validate()` call needed in test helper to match round-trip path (iCal reconstruction calls validate which populates parsed_rrule_set)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] redis_module::logging panics in test mode**
- **Found during:** Task 2
- **Issue:** `redis_module::logging::log_warning` et al. call `RedisModule_Log.unwrap()` which is None outside Redis -- the upstream `cfg!(test)` guard only applies when redis-module itself is the test target
- **Fix:** Added thin `log` wrapper module with `cfg!(test)` check at our crate level
- **Files modified:** redical_redis/src/datatype/mod.rs
- **Committed in:** 1e2e12d (Task 2 commit)

**2. [Rule 1 - Bug] Test calendar equality failure due to missing validate() call**
- **Found during:** Task 2
- **Issue:** `build_test_calendar()` didn't call `event.validate()` so `parsed_rrule_set` was None, but iCal round-trip path calls validate() populating it
- **Fix:** Added `event.validate().unwrap()` in test helper before inserting event
- **Files modified:** redical_redis/src/datatype/mod.rs
- **Committed in:** 1e2e12d (Task 2 commit)

---

**Total deviations:** 2 auto-fixed (2 bugs)
**Impact on plan:** Both fixes necessary for test correctness. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 3 complete: rdb_save writes RDBCalendarDump envelope, rdb_load reads it with three-layer dispatch
- Ready for Phase 4 test fixtures

---
*Phase: 03-rdb-format*
*Completed: 2026-03-06*

## Self-Check: PASSED
