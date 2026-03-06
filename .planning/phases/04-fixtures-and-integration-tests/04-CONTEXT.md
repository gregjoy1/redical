# Phase 4: Fixtures and Integration Tests - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Commit pre-generated binary fixtures (legacy RDBCalendar and mismatched-version RDBCalendarDump) and cover all RDB dispatch paths with tests. No changes to production code — this phase is test-only.

</domain>

<decisions>
## Implementation Decisions

### Fixture data richness
- Minimal Calendar: 1 event with RRULE + 1 event occurrence override
- Enhances the existing `build_test_calendar()` to include an override (currently has event only)
- Both fixtures (legacy + mismatch) serialize the same Calendar data — assertions compare against one expected Calendar
- Full `PartialEq` assertions via `assert_eq!` with `pretty_assertions_sorted` (existing pattern)

### Fast-path test strategy
- `BUILD_VERSION` is `None` in tests — fast path unreachable via normal dispatch
- Test internals directly: existing `test_calendar_bincode_round_trip` in `rdb_data.rs` covers the fast-path data path (serialize + deserialize + rebuild_indexes)
- Add envelope round-trip test: build `RDBCalendarDump` manually, serialize, deserialize, call `load_from_envelope` — exercises dispatch logic (falls through to iCal path since version won't match)
- Keep both: bincode round-trip (data path) + envelope round-trip (dispatch path)

### Test file organization
- `#[ignore]`-gated fixture generator: in `rdb_data.rs` test module (per requirements)
- Fixture-loading dispatch tests: extend existing `load_tests` module in `mod.rs`
- Envelope round-trip test: alongside fixture loading tests in `mod.rs` `load_tests`
- Shared `build_test_calendar()`: extract to a `#[cfg(test)]` helper within `redical_redis` that both `rdb_data.rs` and `mod.rs` can import
- Fixture path: `tests/fixtures/` at workspace root, located via `env!("CARGO_MANIFEST_DIR")` navigating up to workspace

### Claude's Discretion
- Exact module structure for shared test helper (new file vs inline module)
- Whether `build_test_calendar` returns just Calendar or also pre-built RDBCalendar/RDBCalendarDump
- Fixture generator test naming and exact file-writing implementation

</decisions>

<specifics>
## Specific Ideas

- Enhance `build_test_calendar()` with an override to exercise `EventOccurrenceOverride` in the fixture path
- Generator test should be runnable independently to regenerate fixtures without touching test logic

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `build_test_calendar()` in `mod.rs:221-238` — builds Calendar with 1 event + RRULE, needs override added
- `load_from_envelope()` and `load_legacy()` — `pub(crate)` helpers, directly callable from tests
- `test_calendar_rdb_entity` in `rdb_data.rs:267` — builds a Calendar with override, can reference for override construction
- `pretty_assertions_sorted::assert_eq` — already a workspace dependency
- `bincode::serialize` / `bincode::deserialize` — already used throughout tests

### Established Patterns
- Unit tests co-located in `#[cfg(test)] mod tests` / `mod load_tests` at bottom of source files
- `build_event_from_ical()` and `build_event_override_from_ical()` in `redical_core/src/testing/utils.rs`
- `Event::parse_ical(uid, ical_string)` for inline test data construction
- `calendar.rebuild_indexes().unwrap()` after any deserialization

### Integration Points
- `redical_redis/src/datatype/mod.rs` `load_tests` module — extend with fixture loading tests
- `redical_redis/src/datatype/rdb_data.rs` test module — add `#[ignore]` fixture generator
- `tests/fixtures/` at workspace root — new directory for binary fixtures

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 04-fixtures-and-integration-tests*
*Context gathered: 2026-03-06*
