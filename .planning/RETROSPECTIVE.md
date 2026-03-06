# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — RDB Fast-Path Serialization

**Shipped:** 2026-03-06
**Phases:** 4 | **Plans:** 7 | **Sessions:** 1

### What Was Built
- Eliminated `todo!()` crash and `from_utf8_unchecked` UB in RDB save path
- Serde derives across full Calendar type graph (~50 types) with custom Tzid impl
- Versioned `RDBCalendarDump` envelope with bincode fast-path + iCal fallback
- Three-layer `rdb_load` dispatch with `catch_unwind` panic safety
- Binary fixture regression suite covering all dispatch paths

### What Worked
- Single-day execution: 4 phases, 7 plans completed in one session
- Sequential phase dependencies worked well -- each phase cleanly built on the last
- Research agents identified pitfalls (CARGO_MANIFEST_DIR parent, rebuild_indexes pairing) before planning
- Plan verification loop caught nothing -- plans were clean on first pass every time
- Shared test helper extraction (test_helpers.rs) made phase 4 tests clean and DRY

### What Was Inefficient
- ROADMAP.md plan checkboxes not updated by executors (still showing `[ ]` after completion)
- Phase 4 ROADMAP progress table showed "0/2 Not started" even after completion
- Nyquist VALIDATION.md frontmatter never updated to `nyquist_compliant: true` post-execution

### Patterns Established
- `pub(crate)` helpers for testable dispatch paths (load_from_envelope, load_legacy)
- `#[cfg(test)] pub(crate) mod test_helpers` for shared test builders across submodules
- `#[ignore]`-gated fixture generators with `env!("CARGO_MANIFEST_DIR")` path resolution
- Thin log wrapper pattern for test-safe Redis module logging

### Key Lessons
1. Plan verification adds confidence but may be skippable for straightforward test-only phases
2. Pre-existing TODOs should be tracked separately -- they surface in every verification as noise
3. `catch_unwind` scope matters: must wrap `rebuild_indexes()` too, not just bincode deserialize

### Cost Observations
- Model mix: orchestrator on opus, researchers/executors/verifiers on sonnet
- Sessions: 1
- Notable: entire milestone completed in a single context window

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | 1 | 4 | Initial milestone -- established GSD workflow patterns |

### Cumulative Quality

| Milestone | Tests | Coverage | Zero-Dep Additions |
|-----------|-------|----------|-------------------|
| v1.0 | 248 | All dispatch paths | 0 new deps |

### Top Lessons (Verified Across Milestones)

1. (Pending additional milestones for cross-validation)
