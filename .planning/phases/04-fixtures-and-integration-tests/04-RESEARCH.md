# Phase 4: Fixtures and Integration Tests - Research

**Researched:** 2026-03-06
**Domain:** Rust test infrastructure, binary fixtures, bincode serialization testing
**Confidence:** HIGH

## Summary

This phase is test-only -- no production code changes. The work involves: (1) enhancing `build_test_calendar()` to include an `EventOccurrenceOverride`, (2) creating a `#[ignore]`-gated fixture generator that writes two binary files, (3) writing integration tests that load those fixtures through the dispatch paths, and (4) an envelope round-trip test exercising the dispatch logic directly.

All building blocks exist. `load_from_envelope()` and `load_legacy()` are already `pub(crate)` and tested with in-memory data. The fixture tests add file-based coverage and commit known-good binaries for regression detection.

**Primary recommendation:** Extract `build_test_calendar()` to a shared `#[cfg(test)]` helper, add override to it, then build generator and loading tests in sequence.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Minimal Calendar: 1 event with RRULE + 1 event occurrence override
- Enhances the existing `build_test_calendar()` to include an override (currently has event only)
- Both fixtures (legacy + mismatch) serialize the same Calendar data -- assertions compare against one expected Calendar
- Full `PartialEq` assertions via `assert_eq!` with `pretty_assertions_sorted` (existing pattern)
- `BUILD_VERSION` is `None` in tests -- fast path unreachable via normal dispatch
- Test internals directly: existing `test_calendar_bincode_round_trip` in `rdb_data.rs` covers the fast-path data path
- Add envelope round-trip test: build `RDBCalendarDump` manually, serialize, deserialize, call `load_from_envelope`
- Keep both: bincode round-trip (data path) + envelope round-trip (dispatch path)
- `#[ignore]`-gated fixture generator: in `rdb_data.rs` test module
- Fixture-loading dispatch tests: extend existing `load_tests` module in `mod.rs`
- Envelope round-trip test: alongside fixture loading tests in `mod.rs` `load_tests`
- Shared `build_test_calendar()`: extract to a `#[cfg(test)]` helper within `redical_redis` that both `rdb_data.rs` and `mod.rs` can import
- Fixture path: `tests/fixtures/` at workspace root, located via `env!("CARGO_MANIFEST_DIR")` navigating up to workspace

### Claude's Discretion
- Exact module structure for shared test helper (new file vs inline module)
- Whether `build_test_calendar` returns just Calendar or also pre-built RDBCalendar/RDBCalendarDump
- Fixture generator test naming and exact file-writing implementation

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| TEST-01 | Pre-generated binary fixture `tests/fixtures/rdb_calendar_legacy.bin` committed -- bare `RDBCalendar` bincode bytes | Generator test serializes `RDBCalendar` via `bincode::serialize`, writes to file |
| TEST-02 | Pre-generated binary fixture `tests/fixtures/rdb_calendar_dump_mismatch.bin` committed -- `RDBCalendarDump` with mismatched version | Generator test serializes `RDBCalendarDump` with `version: Some("fixture_mismatch")`, writes to file |
| TEST-03 | `#[ignore]`-gated generator test in `rdb_data.rs` to regenerate fixtures | `#[test] #[ignore]` function using `std::fs::write` with path from `env!("CARGO_MANIFEST_DIR")` |
| TEST-04 | Loading `rdb_calendar_legacy.bin` via `rdb_load` logic produces correct Calendar | Read file bytes, call `load_legacy(&bytes)`, assert_eq against `build_test_calendar()` |
| TEST-05 | Loading `rdb_calendar_dump_mismatch.bin` falls back to iCal path and produces correct Calendar | Read file bytes, `bincode::deserialize::<RDBCalendarDump>`, call `load_from_envelope`, assert_eq |
| TEST-06 | In-process `rdb_save` -> `rdb_load` round-trip via fast path | Envelope round-trip: build RDBCalendarDump manually, serialize/deserialize, call `load_from_envelope`, assert_eq |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| bincode | 1.3.3 | Binary serialization for fixtures | Already used in rdb_save/rdb_load |
| pretty_assertions_sorted | 1.2.3 | Readable test diffs | Already a workspace dev-dependency |
| serde | 1.0.162 | Derive traits on test data | Already in workspace deps |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::fs | stdlib | Read/write fixture files | Generator writes, tests read |
| std::path::PathBuf | stdlib | Cross-platform path construction | Fixture path resolution |

No new dependencies required. Everything needed is already in the workspace.

## Architecture Patterns

### Test File Organization

```
redical_redis/src/datatype/
  mod.rs            # load_tests module (TEST-04, TEST-05, TEST-06)
  rdb_data.rs       # test module (TEST-03 generator + existing tests)
  test_helpers.rs   # NEW: #[cfg(test)] shared build_test_calendar()

tests/fixtures/     # At workspace root
  rdb_calendar_legacy.bin
  rdb_calendar_dump_mismatch.bin
```

### Pattern 1: Shared Test Helper Module

**What:** Extract `build_test_calendar()` to a separate file importable by both test modules.

**When to use:** When multiple test modules need the same test data constructor.

**Recommendation:** Create `redical_redis/src/datatype/test_helpers.rs` as a `#[cfg(test)] pub(crate) mod` declared in `mod.rs`. Both `load_tests` (in `mod.rs`) and `rdb_data::test` can then use `super::test_helpers::build_test_calendar()` or `crate::datatype::test_helpers::build_test_calendar()`.

```rust
// redical_redis/src/datatype/test_helpers.rs
use redical_core::{Calendar, Event, EventOccurrenceOverride};

pub fn build_test_calendar() -> Calendar {
    let mut calendar = Calendar::new(String::from("LOAD_TEST_UID"));

    let mut event = Event::parse_ical(
        "EVENT_UID",
        "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 \
         CLASS:PUBLIC CATEGORIES:CATEGORY_ONE \
         DTSTART:19700101T000500Z \
         LAST-MODIFIED:19700101T010500Z",
    ).unwrap();

    let event_override = EventOccurrenceOverride::parse_ical(
        "19700101T000500Z",
        "CLASS:PRIVATE CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO \
         LAST-MODIFIED:19700101T020500Z",
    ).unwrap();

    event.override_occurrence(&event_override, true).unwrap();
    event.validate().unwrap();

    calendar.insert_event(event);
    calendar.rebuild_indexes().unwrap();

    calendar
}
```

**Key detail:** The override construction mirrors `test_calendar_rdb_entity` in `rdb_data.rs:267-273`. The `true` flag on `override_occurrence` means "replace if exists".

### Pattern 2: Fixture Path Resolution

**What:** Locate workspace-root `tests/fixtures/` from within `redical_redis` crate tests.

**How:** `env!("CARGO_MANIFEST_DIR")` returns the crate's directory at compile time. Navigate up one level to workspace root.

```rust
fn fixture_path(filename: &str) -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    manifest_dir
        .parent()  // workspace root
        .unwrap()
        .join("tests")
        .join("fixtures")
        .join(filename)
}
```

This is a compile-time constant path, so it works reliably in CI and locally.

### Pattern 3: Ignored Fixture Generator

**What:** `#[test] #[ignore]` test that generates fixture files. Run manually via `cargo test -p redical_redis --lib -- --ignored generate_fixtures`.

```rust
#[test]
#[ignore]
fn generate_fixtures() {
    let calendar = build_test_calendar();

    // Legacy fixture: bare RDBCalendar
    let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();
    let legacy_bytes = bincode::serialize(&rdb_calendar).unwrap();

    let legacy_path = fixture_path("rdb_calendar_legacy.bin");
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(&legacy_path, &legacy_bytes).unwrap();

    // Mismatch fixture: RDBCalendarDump with non-matching version
    let raw_dump = bincode::serialize(&calendar).unwrap();
    let envelope = RDBCalendarDump {
        version:  Some(String::from("fixture_mismatch")),
        raw_dump,
        dump:     rdb_calendar,
    };
    let mismatch_bytes = bincode::serialize(&envelope).unwrap();

    let mismatch_path = fixture_path("rdb_calendar_dump_mismatch.bin");
    std::fs::write(&mismatch_path, &mismatch_bytes).unwrap();
}
```

### Pattern 4: Fixture Loading Tests

```rust
#[test]
fn load_legacy_fixture_produces_correct_calendar() {
    let expected = build_test_calendar();

    let bytes = std::fs::read(fixture_path("rdb_calendar_legacy.bin")).unwrap();
    let result = load_legacy(&bytes);

    assert_eq!(result, expected);
}

#[test]
fn load_mismatch_fixture_falls_back_to_ical() {
    let expected = build_test_calendar();

    let bytes = std::fs::read(fixture_path("rdb_calendar_dump_mismatch.bin")).unwrap();
    let envelope: RDBCalendarDump = bincode::deserialize(&bytes).unwrap();
    let result = load_from_envelope(envelope);

    assert_eq!(result, expected);
}
```

### Pattern 5: Envelope Round-Trip (TEST-06)

Since `BUILD_VERSION` is `None` in tests, a true fast-path can't fire via `load_from_envelope`. The requirement says "in-process rdb_save -> rdb_load round-trip produces identical Calendar via fast path." The existing `test_calendar_bincode_round_trip` already covers the data path (serialize Calendar, deserialize, rebuild_indexes). The envelope round-trip test exercises the dispatch logic:

```rust
#[test]
fn envelope_round_trip_produces_correct_calendar() {
    let calendar     = build_test_calendar();
    let rdb_calendar = RDBCalendar::try_from(&calendar).unwrap();
    let raw_dump     = bincode::serialize(&calendar).unwrap();

    let envelope = RDBCalendarDump {
        version:  None,
        raw_dump,
        dump:     rdb_calendar,
    };

    let bytes = bincode::serialize(&envelope).unwrap();
    let deserialized: RDBCalendarDump = bincode::deserialize(&bytes).unwrap();
    let result = load_from_envelope(deserialized);

    assert_eq!(result, calendar);
}
```

This covers the full serialize-deserialize-dispatch cycle. Version is `None` so it falls to iCal path, but the round-trip still proves data integrity through the envelope format.

### Anti-Patterns to Avoid
- **Runtime fixture generation:** Don't generate fixtures during normal test runs. The `#[ignore]` gate ensures fixtures are pre-committed artifacts.
- **Hardcoded absolute paths:** Always use `env!("CARGO_MANIFEST_DIR")` -- never hardcode `/Users/.../` paths.
- **Duplicating Calendar construction:** Don't copy-paste `build_test_calendar()` into multiple modules -- extract it once.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Binary serialization | Custom byte packing | `bincode::serialize`/`deserialize` | Already the project standard; deterministic for same input |
| Test diff output | Manual field-by-field assertions | `pretty_assertions_sorted::assert_eq!` | Shows clear diffs on Calendar structs |
| Path construction | String concatenation | `PathBuf::join` | Cross-platform, handles separators |

## Common Pitfalls

### Pitfall 1: Override Not Included in Test Calendar
**What goes wrong:** Tests pass with a Calendar that has no overrides, missing coverage of `EventOccurrenceOverride` serialization paths.
**How to avoid:** The enhanced `build_test_calendar()` must include at least one override. Verify by checking the generated `RDBCalendar` has a non-empty overrides vec.

### Pitfall 2: Fixture Staleness After Schema Changes
**What goes wrong:** Someone changes serde derives or struct fields but forgets to regenerate fixtures.
**How to avoid:** The generator test is `#[ignore]`-gated. Document in test comments that fixtures must be regenerated after any serde-affecting change. The loading tests will fail if fixtures are stale (deserialization error), which is the desired behavior -- it forces conscious regeneration.

### Pitfall 3: CARGO_MANIFEST_DIR Points to Crate, Not Workspace
**What goes wrong:** Code assumes `CARGO_MANIFEST_DIR` is the workspace root, but it's `redical_redis/`.
**How to avoid:** Always call `.parent()` to go up one level to workspace root before joining `tests/fixtures/`.

### Pitfall 4: Forgetting rebuild_indexes After Deserialization
**What goes wrong:** Calendar comparison fails because indexes are empty after bincode deserialization (indexes are `#[serde(skip)]`).
**How to avoid:** `load_legacy` and `load_from_envelope` already call `rebuild_indexes`. The `build_test_calendar()` helper also calls it. Just ensure any direct bincode deserialization in tests also rebuilds.

### Pitfall 5: Event.validate() Must Be Called Before insert_event
**What goes wrong:** Event without validation may have missing computed fields.
**How to avoid:** In `build_test_calendar()`, call `event.validate().unwrap()` before `calendar.insert_event(event)`. The existing `test_calendar_rdb_entity` pattern does this.

## Code Examples

### Building a Calendar with Override (from existing test patterns)
```rust
// Source: rdb_data.rs:267-291 (test_calendar_rdb_entity)
let event_override = EventOccurrenceOverride::parse_ical(
    "19700101T000500Z",
    "CLASS:PRIVATE CATEGORIES:\"CATEGORY THREE\",CATEGORY_ONE,CATEGORY_TWO \
     LAST-MODIFIED:19700101T020500Z",
).unwrap();

let mut event = Event::parse_ical(
    "EVENT_UID",
    "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 \
     CLASS:PUBLIC CATEGORIES:CATEGORY_ONE \
     DTSTART:19700101T000500Z \
     LAST-MODIFIED:19700101T010500Z",
).unwrap();

event.override_occurrence(&event_override, true).unwrap();
event.validate().unwrap();
```

### Reading Fixture Files
```rust
// Source: Rust std::fs
let bytes = std::fs::read(fixture_path("rdb_calendar_legacy.bin")).unwrap();
```

### Writing Fixture Files
```rust
// Source: Rust std::fs
std::fs::create_dir_all(path.parent().unwrap()).unwrap();
std::fs::write(&path, &bytes).unwrap();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Only in-memory test data | Pre-committed binary fixtures | This phase | Regression detection for format changes |
| `build_test_calendar()` without overrides | Enhanced with `EventOccurrenceOverride` | This phase | Full serialization coverage |
| Duplicated test Calendar construction | Shared `test_helpers` module | This phase | Single source of truth for test data |

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` + cargo test |
| Config file | Cargo.toml (workspace and crate-level) |
| Quick run command | `cargo test -p redical_redis --lib -- datatype` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| TEST-01 | Legacy fixture file exists | fixture + unit | `test -f tests/fixtures/rdb_calendar_legacy.bin` | No -- Wave 0 |
| TEST-02 | Mismatch fixture file exists | fixture + unit | `test -f tests/fixtures/rdb_calendar_dump_mismatch.bin` | No -- Wave 0 |
| TEST-03 | Generator test exists (ignored) | unit (ignored) | `cargo test -p redical_redis --lib -- generate_fixtures --ignored` | No -- Wave 0 |
| TEST-04 | Legacy fixture loads correctly | unit | `cargo test -p redical_redis --lib -- load_legacy_fixture` | No -- Wave 0 |
| TEST-05 | Mismatch fixture falls back to iCal | unit | `cargo test -p redical_redis --lib -- load_mismatch_fixture` | No -- Wave 0 |
| TEST-06 | Envelope round-trip produces correct Calendar | unit | `cargo test -p redical_redis --lib -- envelope_round_trip` | No -- Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test -p redical_redis --lib -- datatype`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] `redical_redis/src/datatype/test_helpers.rs` -- shared `build_test_calendar()` with override
- [ ] `tests/fixtures/` directory at workspace root -- created by generator
- [ ] `tests/fixtures/rdb_calendar_legacy.bin` -- generated by TEST-03
- [ ] `tests/fixtures/rdb_calendar_dump_mismatch.bin` -- generated by TEST-03

## Open Questions

1. **Should `build_test_calendar` also return RDBCalendar/RDBCalendarDump?**
   - What we know: Generator and loading tests both need RDBCalendar. The envelope round-trip test needs RDBCalendarDump.
   - Recommendation: Return just Calendar. Each test constructs the derived types it needs -- keeps the helper simple and its callers explicit.

2. **Should existing tests in mod.rs and rdb_data.rs be updated to use the shared helper?**
   - What we know: The existing `build_test_calendar()` in `mod.rs:221` builds a Calendar without overrides. Some existing tests depend on that exact shape.
   - Recommendation: Keep existing tests using their current inline data. Only new tests use the shared helper. The old `build_test_calendar` in `mod.rs` gets replaced by the import but must produce the same Calendar + override to avoid breaking existing tests. Actually, the existing tests only check that the result equals the input Calendar, so adding an override to the shared version is fine -- the assertions are `assert_eq!(result, calendar)` where `calendar` is built by the same function.

## Sources

### Primary (HIGH confidence)
- Project source code: `redical_redis/src/datatype/mod.rs`, `rdb_data.rs` -- read directly
- Existing test patterns: `rdb_data.rs` test module, `load_tests` module
- Rust stdlib docs for `std::fs`, `env!()` macro, `#[ignore]` attribute

### Secondary (MEDIUM confidence)
- `bincode` 1.3.3 deterministic serialization -- verified by existing round-trip tests in codebase

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all already in workspace
- Architecture: HIGH -- extending existing test modules with well-understood patterns
- Pitfalls: HIGH -- documented from direct code reading, not speculation

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain, no external dependencies changing)
