---
status: testing
phase: 04-fixtures-and-integration-tests
source: [04-01-SUMMARY.md, 04-02-SUMMARY.md]
started: 2026-03-06T17:10:00Z
updated: 2026-03-06T17:10:00Z
---

## Current Test

number: 1
name: Binary fixtures exist and are committed
expected: |
  Both `tests/fixtures/rdb_calendar_legacy.bin` and `tests/fixtures/rdb_calendar_dump_mismatch.bin` exist at workspace root and are tracked by git.
  Run: `ls -la tests/fixtures/` and `git ls-files tests/fixtures/`
  Both files should appear in both listings.
awaiting: user response

## Tests

### 1. Binary fixtures exist and are committed
expected: Both `tests/fixtures/rdb_calendar_legacy.bin` and `tests/fixtures/rdb_calendar_dump_mismatch.bin` exist at workspace root and are tracked by git. Run: `ls -la tests/fixtures/` and `git ls-files tests/fixtures/`. Both files appear in both listings.
result: [pending]

### 2. Fixture generator regenerates fixtures
expected: Running `cargo test -p redical_redis --lib -- generate_fixtures --ignored` succeeds and overwrites the fixture files. Run the command — it should complete with "1 passed" and no errors.
result: [pending]

### 3. All existing tests still pass
expected: Running `cargo test -p redical_redis --lib -- datatype` passes all tests (existing unit tests + new integration tests). No regressions from the shared test helper extraction.
result: [pending]

### 4. Legacy fixture loads correctly
expected: Running `cargo test -p redical_redis --lib -- load_legacy_fixture_produces_correct_calendar` passes. The bare RDBCalendar bincode fixture deserializes and matches the expected Calendar.
result: [pending]

### 5. Mismatch fixture falls back to iCal path
expected: Running `cargo test -p redical_redis --lib -- load_mismatch_fixture_falls_back_to_ical` passes. The mismatched-version RDBCalendarDump falls through to iCal deserialization and produces the correct Calendar.
result: [pending]

### 6. Envelope round-trip produces identical Calendar
expected: Running `cargo test -p redical_redis --lib -- envelope_round_trip_produces_correct_calendar` passes. A manually-built RDBCalendarDump survives serialize/deserialize/dispatch and matches the original Calendar.
result: [pending]

## Summary

total: 6
passed: 0
issues: 0
pending: 6
skipped: 0

## Gaps

[none yet]
