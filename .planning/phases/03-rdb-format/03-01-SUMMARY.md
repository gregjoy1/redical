---
phase: 03-rdb-format
plan: 01
subsystem: database
tags: [bincode, serde, rdb, redis-module]

requires:
  - phase: 02-serde-derive-chain
    provides: Calendar serde derive chain for bincode serialization
provides:
  - RDBCalendarDump envelope struct with dual-representation fields
  - rdb_save producing envelope format (bincode raw_dump + iCal fallback)
  - BUILD_VERSION const from GIT_SHA
affects: [03-02, 04-test-fixtures]

tech-stack:
  added: []
  patterns: [envelope struct wrapping fast-path + fallback data]

key-files:
  created: []
  modified:
    - redical_redis/src/datatype/rdb_data.rs
    - redical_redis/src/datatype/mod.rs

key-decisions:
  - "Keep panics in rdb_save -- fundamentally broken state if in-memory Calendar fails to serialize"
  - "BUILD_VERSION as Option<&str> const from option_env!(GIT_SHA)"

patterns-established:
  - "RDBCalendarDump envelope: version + raw_dump (bincode) + dump (iCal-based RDBCalendar)"

requirements-completed: [RDB-01, RDB-02]

duration: 2min
completed: 2026-03-06
---

# Phase 3 Plan 1: RDBCalendarDump Envelope Summary

**RDBCalendarDump envelope struct carrying bincode raw_dump + iCal RDBCalendar fallback, rdb_save rewritten to produce envelope format**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-06T16:04:41Z
- **Completed:** 2026-03-06T16:06:43Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- RDBCalendarDump struct with version, raw_dump, dump fields in rdb_data.rs
- rdb_save rewritten to serialize Calendar via bincode (raw_dump) and iCal (RDBCalendar dump) into single envelope
- BUILD_VERSION const resolves GIT_SHA at compile time
- Two round-trip tests (with/without version) validate envelope serialization

## Task Commits

Each task was committed atomically:

1. **Task 1: Add RDBCalendarDump struct and envelope round-trip test** - `93bbe60` (feat+test, TDD)
2. **Task 2: Rewrite rdb_save to produce RDBCalendarDump envelope** - `4089dbd` (feat)

## Files Created/Modified
- `redical_redis/src/datatype/rdb_data.rs` - Added RDBCalendarDump struct + 2 round-trip tests
- `redical_redis/src/datatype/mod.rs` - BUILD_VERSION const, RDBCalendarDump import, rdb_save envelope format

## Decisions Made
- Kept panics in rdb_save per user decision -- serialization failure of in-memory Calendar is fundamentally broken state
- BUILD_VERSION as `Option<&str>` const from `option_env!("GIT_SHA")` for readability

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Envelope format established, rdb_save writes it
- rdb_load (Plan 03-02) needs three-layer dispatch: envelope -> legacy -> panic
- catch_unwind wrapping fast-path deserialization needed in 03-02

---
*Phase: 03-rdb-format*
*Completed: 2026-03-06*

## Self-Check: PASSED
