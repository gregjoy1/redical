# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-06)

**Core value:** Calendar RDB load/save must be fast for same-version deployments while never corrupting or losing data across version boundaries.
**Current focus:** Phase 1 — Safety Fixes

## Current Position

Phase: 1 of 4 (Safety Fixes)
Plan: 0 of ? in current phase
Status: Ready to plan
Last activity: 2026-03-06 — Roadmap created

Progress: [░░░░░░░░░░] 0%

## Performance Metrics

**Velocity:**
- Total plans completed: 0
- Average duration: -
- Total execution time: 0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**
- Last 5 plans: -
- Trend: -

*Updated after each plan completion*

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- GIT_SHA as version discriminator (build.rs already sets it; None safely disables fast path)
- `catch_unwind` must wrap full `rdb_load` body including `rebuild_indexes()`, not just the bincode call
- `RDBCalendar` kept as fallback inside `RDBCalendarDump` (single blob, both paths)
- `aof_rewrite` as empty stub (unblocks compilation; AOF rewrite deferred to v2)
- Pre-generated fixture files (not generated at test runtime)

### Pending Todos

None yet.

### Blockers/Concerns

- `redis-module` 2.0.4 API: `save_string_buffer` availability not verified — check changelog before implementing `from_utf8_unchecked` fix in Phase 1
- `chrono` serde feature: needs verification that `serde` feature is enabled in workspace before Phase 2
- `redical_ical` property/value type serde surface: exact scope unknown upfront — use compiler-driven discovery in Phase 2

## Session Continuity

Last session: 2026-03-06
Stopped at: Roadmap created, ready to plan Phase 1
Resume file: None
