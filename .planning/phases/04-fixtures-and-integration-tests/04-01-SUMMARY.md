---
phase: 04-fixtures-and-integration-tests
plan: 01
subsystem: testing
tags: [bincode, fixtures, test-helpers, rdb]

requires:
  - phase: 03-rdb-format
    provides: "RDBCalendar, RDBCalendarDump types and load_from_envelope/load_legacy helpers"
provides:
  - "Shared build_test_calendar() with EventOccurrenceOverride for both test modules"
  - "fixture_path() helper for locating workspace-root test fixtures"
  - "rdb_calendar_legacy.bin binary fixture"
  - "rdb_calendar_dump_mismatch.bin binary fixture"
  - "#[ignore]-gated fixture generator test"
affects: [04-02]

tech-stack:
  added: []
  patterns: ["shared #[cfg(test)] test_helpers module across datatype submodules"]

key-files:
  created:
    - redical_redis/src/datatype/test_helpers.rs
    - tests/fixtures/rdb_calendar_legacy.bin
    - tests/fixtures/rdb_calendar_dump_mismatch.bin
  modified:
    - redical_redis/src/datatype/mod.rs
    - redical_redis/src/datatype/rdb_data.rs

key-decisions:
  - "Override-enriched calendar as shared test data -- both fixture and load_tests use identical Calendar"
  - "fixture_path via CARGO_MANIFEST_DIR parent -- locates workspace-root tests/fixtures from any subcrate"

patterns-established:
  - "test_helpers.rs: shared test builders as pub(crate) #[cfg(test)] module"
  - "#[ignore]-gated fixture generators: cargo test --ignored to regenerate"

requirements-completed: [TEST-01, TEST-02, TEST-03]

duration: 3min
completed: 2026-03-06
---

# Phase 4 Plan 1: Test Helpers and Binary Fixtures Summary

**Shared override-enriched Calendar builder, #[ignore]-gated fixture generator, and two committed binary fixtures (legacy + mismatch)**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T16:53:57Z
- **Completed:** 2026-03-06T16:57:00Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Extracted shared `build_test_calendar()` with EventOccurrenceOverride to `test_helpers.rs`
- Created `fixture_path()` helper for locating workspace-root fixtures from subcrate tests
- Added `#[ignore]`-gated `generate_fixtures` test producing both binary fixtures
- All 13 existing tests + 3 load_tests pass with enriched calendar

## Task Commits

Each task was committed atomically:

1. **Task 1: Extract shared test helper and update mod.rs** - `f404cfd`
2. **Task 2: Create fixture generator and generate binary fixtures** - `2bc92d5`

## Files Created/Modified
- `redical_redis/src/datatype/test_helpers.rs` - Shared build_test_calendar() and fixture_path()
- `redical_redis/src/datatype/mod.rs` - Added #[cfg(test)] mod test_helpers, load_tests uses shared import
- `redical_redis/src/datatype/rdb_data.rs` - Added #[ignore] generate_fixtures test
- `tests/fixtures/rdb_calendar_legacy.bin` - Bare RDBCalendar bincode bytes (480 bytes)
- `tests/fixtures/rdb_calendar_dump_mismatch.bin` - RDBCalendarDump with mismatched version (1030 bytes)

## Decisions Made
- Override-enriched calendar as shared test data -- single Calendar used by both fixture generation and load_tests assertions
- fixture_path navigates via CARGO_MANIFEST_DIR parent to workspace-root tests/fixtures/

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Binary fixtures committed, ready for plan 02 fixture-loading dispatch tests
- test_helpers::build_test_calendar and fixture_path importable from any test in redical_redis

---
*Phase: 04-fixtures-and-integration-tests*
*Completed: 2026-03-06*

## Self-Check: PASSED
