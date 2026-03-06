# Testing Patterns

**Analysis Date:** 2026-03-06

## Test Framework

**Runner:**
- Rust's built-in `cargo test`
- No external test runner

**Assertion Library:**
- `pretty_assertions_sorted` (workspace dependency) — provides `assert_eq!`, `assert_eq_sorted!`, `assert_ne!` with coloured diffs
- Standard `assert!` for boolean checks

**Run Commands:**
```bash
cargo test --all                      # Run all tests (unit + integration)
cargo test --all integration          # Run only integration tests
cargo test -p redical_ical            # Run tests for a specific crate
cargo test -p redical_core            # Run core crate tests only
```

## Test File Organization

**Location:**
- Unit tests are co-located: `#[cfg(test)] mod tests { ... }` block at the bottom of each source file
- Integration tests: `tests/integration.rs` at workspace root — runs against a live Redis instance
- Integration test helpers split into `tests/macros.rs` and `tests/utils.rs`
- Core test utilities: `redical_core/src/testing/` — `macros.rs` and `utils.rs`
- Fuzz regression tests: `redical_ical/tests/fuzzing_hang_tests.rs` (marked `#[ignore]`)

**Naming:**
- Test functions named after the method under test: `fn parse_ical()`, `fn render_ical()`, `fn date_time_parse_ical_error()`
- Multiple tests per method grouped by scenario suffix: `parse_ical`, `parse_ical_error`, `parse_ical_with_terminated_property_lookahead`, `render_ical_with_context_tz_override`

**Structure:**
```
redical_ical/src/values/date_time.rs     # Unit tests at bottom of file
redical_ical/src/properties/event/dtstart.rs
redical_core/src/testing/macros.rs       # Shared test macros for core
redical_core/src/testing/utils.rs        # Shared test helpers for core
tests/integration.rs                     # Workspace-level integration tests
tests/macros.rs                          # Shared macros for integration tests
tests/utils.rs                           # Shared utils for integration tests
```

## Test Structure

**Suite Organization:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    use chrono::{NaiveDate, NaiveTime, NaiveDateTime};
    use crate::tests::{assert_parser_output, assert_parser_error};

    #[test]
    fn parse_ical() {
        assert_parser_output!(
            DTStartProperty::parse_ical("DTSTART:19960401T150000Z DESCRIPTION:Description text".into()),
            (
                " DESCRIPTION:Description text",
                DTStartProperty { ... },
            ),
        );
    }

    #[test]
    fn parse_ical_error() {
        assert_parser_error!(
            DTStartProperty::parse_ical("...".into()),
            nom::Err::Failure(
                span: "...",
                message: "expected ...",
                context: ["DTSTART"],
            ),
        );
    }

    #[test]
    fn render_ical() {
        assert_eq!(
            MyType { ... }.render_ical(),
            String::from("EXPECTED:VALUE"),
        );
    }
}
```

**Patterns:**
- Each `impl ICalendarEntity` type has at minimum: `parse_ical`, `render_ical` test functions
- Error cases test both `nom::Err::Error` (soft fail / backtrackable) and `nom::Err::Failure` (hard fail after `cut()`)
- Happy-path tests use `assert_parser_output!`; error tests use `assert_parser_error!`
- Render tests use plain `assert_eq!` comparing `.render_ical()` to a `String::from("...")` literal

## Mocking

**Framework:** None — no mock library used.

**Patterns:**
- No mocking of external dependencies; tests either use real instances or build minimal structs directly
- Integration tests spin up a real Redis server instance on port 6480 using the module binary
- The `run_all_integration_tests_sequentially!` macro runs all integration test functions sequentially through a single Redis connection (avoids port conflicts and state bleed)

**What to Mock:**
- Not applicable — the codebase does not use mocking.

**What NOT to Mock:**
- Redis connection — integration tests use real Redis. Do not introduce mock Redis clients.

## Fixtures and Factories

**Test Data:**
```rust
// Building typed values inline in test assertions
DateTime::UtcDateTime(
    NaiveDateTime::new(
        NaiveDate::from_ymd_opt(1998_i32, 1_u32, 18_u32).unwrap(),
        NaiveTime::from_hms_opt(23_u32, 0_u32, 0_u32).unwrap(),
    )
)
```

**Core test utilities** at `redical_core/src/testing/utils.rs`:
```rust
// Build an Event from iCal property string slices
build_event_from_ical(event_uid, vec![
    "DTSTART:20201231T160000Z",
    "RRULE:FREQ=WEEKLY;BYDAY=MO",
]);

// Build event with overrides
build_event_and_overrides_from_ical(uid, ical_parts, overrides);

// Build a single event override
build_event_override_from_ical(dtstart_date_string, override_ical_parts);
```

**`build_property_from_ical!` macro** at `redical_core/src/testing/macros.rs`:
```rust
let property = build_property_from_ical!(DTStartProperty, "DTSTART:19960401T150000Z");
```

**Location:** No separate fixtures directory — all test data is inline within test functions.

## Coverage

**Requirements:** None enforced — no coverage configuration found.

**View Coverage:**
```bash
# Install cargo-tarpaulin then:
cargo tarpaulin --all
```

## Test Types

**Unit Tests:**
- Scope: individual parser functions, entity types, value types, property types
- Location: `#[cfg(test)] mod tests` within each `.rs` file
- Cover: `parse_ical` (success cases, error cases, edge cases), `render_ical` (round-trip), `validate`, type conversions (`From`, `with_timezone`, etc.)

**Integration Tests:**
- Location: `tests/integration.rs`
- Scope: full Redis command round-trips — set, get, list, delete, query, prune operations
- Require a running Redis server with the `redical` module loaded
- Run sequentially via `run_all_integration_tests_sequentially!` macro — each sub-test flushes the DB after completion

**Fuzz/Regression Tests:**
- Location: `redical_ical/tests/fuzzing_hang_tests.rs`
- Scope: hang regression for previously discovered AFL fuzzing inputs
- Marked `#[ignore]` by default — run manually when testing parser performance

## Common Patterns

**Parser success assertion:**
```rust
assert_parser_output!(
    SomeType::parse_ical("INPUT remaining".into()),
    (
        " remaining",          // expected remaining input
        SomeType { ... },      // expected parsed value
    ),
);
```

**Parser error assertion:**
```rust
assert_parser_error!(
    SomeType::parse_ical(":".into()),
    nom::Err::Error(           // or nom::Err::Failure for hard failures
        span: ":",
        message: "expected iCalendar RFC-5545 ...",
        context: ["OUTER_CONTEXT", "INNER_CONTEXT"],
    ),
);
```

**Timeout-bounded test:**
```rust
assert_finishes_within_duration!(
    1000,  // milliseconds
    SomeType::parse_ical(potentially_slow_input.into()),
);
```

**Integration test helper macros** (defined in `tests/macros.rs`):
- `set_and_assert_calendar!(connection, uid)` — set + verify calendar exists
- `set_and_assert_event!(connection, cal_uid, event_uid, [ical_properties...])` — set + verify event
- `set_and_assert_event_override!(connection, cal_uid, event_uid, dtstart, [properties...])` — set + verify override
- `list_and_assert_matching_events!(connection, cal_uid, [[event_props...], ...])` — list + compare
- `query_calendar_and_assert_matching_event_instances!(connection, cal_uid, [query_props...], [results...])` — query + compare
- `assert_keyspace_events_published!(message_queue, event, keyname)` — Redis pub/sub assertions
- `assert_error_returned!(connection, "expected error", "COMMAND", arg1, arg2)` — error case validation

---

*Testing analysis: 2026-03-06*
