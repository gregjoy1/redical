---
phase: 02-serde-derive-chain
plan: 02
subsystem: serialization
tags: [serde, derive, bincode, calendar, event, skip]

requires:
  - phase: 02-serde-derive-chain
    provides: Serialize + Deserialize on all redical_ical types in Calendar field graph
provides:
  - Serialize + Deserialize on all 8 redical_core types
  - serde(skip) on 11 computed/index fields across Calendar and Event
  - Bincode round-trip smoke tests proving full serialize chain
affects: [03-rdb-format, redical_redis rdb_save/rdb_load]

tech-stack:
  added: []
  patterns: [serde skip for computed/index fields rebuilt post-deserialization]

key-files:
  created: []
  modified:
    - redical_core/src/calendar.rs
    - redical_core/src/event.rs
    - redical_core/src/event_occurrence_override.rs
    - redical_core/src/utils.rs
    - redical_core/src/geo_index.rs
    - redical_redis/src/datatype/rdb_data.rs

key-decisions:
  - "indexes_active kept serialized (source state, not computed)"
  - "InvertedEventIndex, InvertedCalendarIndex, GeoSpatialCalendarIndex excluded from serde (rebuilt post-load)"

patterns-established:
  - "serde(skip) + rebuild_indexes() pattern: skip computed fields, rebuild after deserialize"

requirements-completed: [SERD-03, SERD-04]

duration: 3min
completed: 2026-03-06
---

# Phase 2 Plan 2: redical_core Serde Derives Summary

**Serde derives on 8 redical_core types with 11 serde(skip) annotations and bincode round-trip smoke tests**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-06T15:09:30Z
- **Completed:** 2026-03-06T15:12:38Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- All 8 redical_core types (Calendar, Event, ScheduleProperties, IndexedProperties, PassiveProperties, EventOccurrenceOverride, KeyValuePair, GeoPoint) derive Serialize + Deserialize
- 11 computed/index fields annotated with #[serde(skip)] and explanatory comments
- `indexes_active` correctly NOT skipped (source state)
- Bincode serialize/deserialize round-trip verified end-to-end with rebuild_indexes()
- All 75 existing workspace tests pass unchanged

## Task Commits

1. **Task 1: Derive serde on redical_core types with skip annotations** - `3bd50c2` (feat)
2. **Task 2: Bincode round-trip smoke test** - `eecc7f6` (test)

## Files Created/Modified
- `redical_core/src/utils.rs` - KeyValuePair derives Serialize, Deserialize
- `redical_core/src/geo_index.rs` - GeoPoint derives Serialize, Deserialize
- `redical_core/src/event_occurrence_override.rs` - EventOccurrenceOverride derives serde
- `redical_core/src/event.rs` - ScheduleProperties, IndexedProperties, PassiveProperties, Event derive serde; 6 fields skipped
- `redical_core/src/calendar.rs` - Calendar derives serde; 5 index fields skipped
- `redical_redis/src/datatype/rdb_data.rs` - Bincode round-trip smoke tests added

## Decisions Made
- `indexes_active` kept serialized as it is source state, not a computed field
- InvertedEventIndex, InvertedCalendarIndex, GeoSpatialCalendarIndex excluded from serde derives (rebuilt by rebuild_indexes() post-load)

## Deviations from Plan

None - plan executed exactly as written.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness
- Full serde derive chain complete: `bincode::serialize(&calendar)` and `bincode::deserialize::<Calendar>(bytes)` both work
- Ready for Phase 3 RDB format work to use bincode serialization in rdb_save/rdb_load

---
*Phase: 02-serde-derive-chain*
*Completed: 2026-03-06*
