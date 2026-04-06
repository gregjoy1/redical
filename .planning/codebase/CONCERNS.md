# Codebase Concerns

**Analysis Date:** 2026-03-06

## Tech Debt

**Duplicated query layer (`EventQuery` vs `EventInstanceQuery`):**
- Issue: `event_query.rs` (1346 lines) and `event_instance_query.rs` (1315 lines) are near-identical. Both have the same `IndexAccessor` structs, the same `search_*_index` / `search_not_*_index` methods, and parallel test suites. The only meaningful difference is the output type (`Event` vs `EventInstance`).
- Files: `redical_core/src/queries/event_query.rs`, `redical_core/src/queries/event_instance_query.rs`
- Impact: Every bug fix, index search change, or new property filter must be applied twice. Divergence between the two is likely over time.
- Fix approach: Extract a shared generic query executor parameterised on output type, or use a trait to unify the index accessor logic.

**`insert_new_where_conditional` marked for cleanup:**
- Issue: The `// TODO: Clean this up!` comment at the trait method in `query.rs` flags this as a known rough spot in the query builder API.
- Files: `redical_core/src/queries/query.rs:64`
- Impact: Query construction is harder to follow and extend correctly.
- Fix approach: Refactor the `insert_new_where_conditional` method into a cleaner builder pattern.

**Duplicate serialization path in `rdcl_evt_query` and `rdcl_evi_query`:**
- Issue: Both command handlers contain `// TODO: Clean up and properly serialize this griminess` around the result serialization to `RedisValue::Array`.
- Files: `redical_redis/src/commands/rdcl_evt_query.rs:114`, `redical_redis/src/commands/rdcl_evi_query.rs:114`
- Impact: Fragile output format that is hard to change consistently.
- Fix approach: Extract a shared `serialize_query_results` helper function.

**`timestamp_from_date_string` helper not promoted:**
- Issue: `// TODO: make this a helper` comment exists for a date-string-to-timestamp conversion in `rdcl_evt_prune.rs`. The same pattern is repeated in `rdcl_evo_prune.rs`.
- Files: `redical_redis/src/commands/rdcl_evt_prune.rs:70`, `redical_redis/src/commands/rdcl_evo_prune.rs`
- Impact: Inconsistency risk if parsing logic diverges between prune commands.
- Fix approach: Move to a shared `commands/utils.rs` module.

**`inverted_index.rs` merge efficiency notes:**
- Issue: `merge_and` and `merge_or` both carry `// TODO: * Iterate on the smallest HashMap for efficiency` and `// TODO: clone()/borrowing etc` comments. These are hot paths executed on every indexed query.
- Files: `redical_core/src/inverted_index.rs:54`, `redical_core/src/inverted_index.rs:81`
- Impact: Unnecessary allocations and suboptimal iteration order on large calendars.
- Fix approach: Iterate over the smaller map in `merge_and`; reduce `clone()` calls by working with references where possible.

**`inverted_index.rs` lacks tests for merge operations:**
- Issue: `// TODO: Add tests...` appears at lines 303 and 335 for the merge operations.
- Files: `redical_core/src/inverted_index.rs:303`, `redical_core/src/inverted_index.rs:335`
- Impact: Core index merge logic is untested; regressions would be invisible.
- Fix approach: Add unit tests covering `merge_and` / `merge_or` edge cases including empty sets, overlapping conclusions, and exception lists.

**`rebuild_indexed_geo` and `rebuild_indexed_class` lack tests:**
- Issue: Both methods have `// TODO: Add tests...` comments.
- Files: `redical_core/src/event.rs:577`, `redical_core/src/event.rs:584`
- Impact: Index rebuild correctness for geo and class properties is unverified.

## Known Bugs

**UID index query with unrecognised term returns wrong result:**
- Symptoms: Querying for a UID that is not in the index returns an `Include` result containing that UID rather than an empty set. Two separate `// TODO: fix this. Should return an empty event set` comments confirm this is a known incorrect behaviour.
- Files: `redical_core/src/queries/indexed_property_filters.rs:731`, `redical_core/src/queries/indexed_property_filters.rs:1065`, `redical_core/src/queries/event_instance_query.rs:522`, `redical_core/src/queries/event_query.rs:572`
- Trigger: Execute a WHERE UID = "NONEXISTENT_UID" query against a calendar that does not contain that event.
- Workaround: Callers must filter the result set against the events map to discard phantom UIDs.

**Missing indexed event silently skipped during query execution:**
- Symptoms: When an inverted index contains a UID for which no `Event` object exists in `calendar.events`, the query silently continues rather than surfacing an error. Three separate `// TODO: handle missing indexed event...` comments mark these sites.
- Files: `redical_core/src/queries/event_instance_query.rs:277`, `redical_core/src/queries/event_instance_query.rs:409`, `redical_core/src/queries/event_query.rs:361`, `redical_core/src/queries/event_query.rs:495`
- Trigger: Index/data inconsistency after a failed partial update.
- Workaround: None; the event is silently dropped from results.

**RECURRENCE-ID `RANGE` parameter not implemented:**
- Symptoms: The `RANGE` parameter on `RECURRENCE-ID` (used to modify `THISANDFUTURE` occurrences) is silently ignored.
- Files: `redical_ical/src/properties/recurrence_id.rs:22`
- Impact: Clients relying on `RANGE=THISANDFUTURE` semantics will have overrides applied only to the single specified occurrence.

**`RDATE` PERIOD value type not implemented:**
- Symptoms: Parsing `RDATE` with `VALUE=PERIOD` is not supported; the `// TODO: Implement PERIOD VALUE type.` comment confirms this.
- Files: `redical_ical/src/properties/event/rdate.rs:152`
- Impact: iCalendar feeds using period-typed RDATEs will fail to parse.

## Security Considerations

**`from_utf8_unchecked` on RDB serialized bytes:**
- Risk: `rdb_save` serializes a `Calendar` to bincode bytes and then calls `std::str::from_utf8_unchecked` before passing the result to `raw::save_string`. Bincode output is arbitrary binary; if it contains non-UTF-8 sequences this is undefined behaviour.
- Files: `redical_redis/src/datatype/mod.rs:80`
- Current mitigation: None. The comment acknowledges this is a workaround for missing `save_string_buffer` in the redis-module crate.
- Recommendations: Track redis-module crate for `save_string_buffer` addition; alternatively encode bytes as base64 before saving.

**RDB files committed to repository:**
- Risk: `dodgey_dump.rdb`, `dump.rdb`, and `test_dump.rdb` are present at the project root and not consistently excluded from git (`.gitignore` does exclude `**/*.rdb` but these files appear to have been committed at some point).
- Files: `/dodgey_dump.rdb`, `/dump.rdb`, `/test_dump.rdb`
- Current mitigation: `.gitignore` covers `**/*.rdb` so they should not be staged going forward.
- Recommendations: Confirm these files are not tracked in git history; remove if so.

## Performance Bottlenecks

**`rdcl.evo_prune` collects all event UIDs before iterating:**
- Problem: Full key collection via `calendar.events.keys().map(String::from).collect()` creates an allocation proportional to the number of events before any pruning begins.
- Files: `redical_redis/src/commands/rdcl_evo_prune.rs:192`
- Cause: Required to avoid borrowing `calendar` mutably while iterating its map.
- Improvement path: Collect only UIDs whose events have overrides in the target range; use a cursor-based approach for very large calendars.

**Inverted index merge does not iterate smallest map first:**
- Problem: `merge_and` always iterates `events_a` regardless of size; for large calendars with sparse index overlap this wastes work.
- Files: `redical_core/src/inverted_index.rs:54`
- Cause: Acknowledged in TODO comment; not yet fixed.
- Improvement path: Swap iteration to use the smaller of `events_a` / `events_b`.

**`mem_usage` always returns zero:**
- Problem: The Redis module's `mem_usage` callback always returns `0`, so Redis cannot accurately report memory used by calendar data types via `MEMORY USAGE` or enforce `maxmemory` policies against this data.
- Files: `redical_redis/src/datatype/mod.rs:93`
- Cause: Stub implementation.
- Improvement path: Implement using `std::mem::size_of_val` recursively or a custom `MemoryUsage` trait.

## Fragile Areas

**`aof_rewrite` is a hard `todo!()`:**
- Files: `redical_redis/src/datatype/mod.rs:90`
- Why fragile: Calling this function (which Redis may invoke during AOF rewrite) will panic the Redis process.
- Safe modification: Implement AOF rewrite or explicitly return an error; do not leave as `todo!()` in production.
- Test coverage: None.

**`rdb_load` / `rdb_save` panic on error instead of propagating:**
- Files: `redical_redis/src/datatype/mod.rs:57-74`
- Why fragile: A corrupted or schema-mismatched RDB dump will crash the Redis process on startup via `panic!`.
- Safe modification: Surface errors through the Redis module API (return `null_mut()` with a logged error) rather than panicking.

**`DateTime::from_utc_timestamp` panics on invalid timestamp:**
- Files: `redical_ical/src/values/date_time.rs:314-320`
- Why fragile: Any code path that produces an out-of-range or ambiguous UTC timestamp will crash at runtime. The comments acknowledge this is not handled well.
- Safe modification: Return `Result<DateTime, String>` and propagate errors upward.

**`QueryResultOrdering::partial_cmp` panics on mismatched variants:**
- Files: `redical_core/src/queries/results_ordering.rs:292`
- Why fragile: If query result ordering is ever constructed inconsistently (e.g., mixing geo-distance and dtstart orderings in the same result set), a sort will panic.
- Safe modification: Return `None` from `partial_cmp` on mismatched variants instead of panicking.

**`event_instance.rs` test panics on missing override:**
- Files: `redical_core/src/event_instance.rs:513`
- Why fragile: `panic!("Expected event to have an occurrence...")` inside a test provides a poor failure message and masks the actual assertion failure.

## Scaling Limits

**In-memory calendar model:**
- Current capacity: All events and overrides for a calendar key are held in a single `Calendar` struct in Redis memory. No pagination of the data structure itself.
- Limit: Very large calendars (millions of events/overrides) will consume substantial Redis memory with no eviction strategy.
- Scaling path: Consider sharding calendars or implementing a tiered storage strategy.

**Fuzz-discovered hang inputs not fully addressed:**
- Current state: 75 hang inputs are stored in `redical_ical/tests/fuzz_finds/hangs/` but the hang test (`parse_ical_fuzzing_hang_test`) is marked `#[ignore]` and will panic if any input takes more than 1 second.
- Files: `redical_ical/tests/fuzz_finds/hangs/`, `redical_ical/tests/fuzzing_hang_tests.rs`
- Risk: Maliciously crafted iCalendar input can trigger parser hangs (CPU spin), acting as a denial-of-service vector against the Redis module.
- Scaling path: Fix the underlying parser backtracking issue that causes hangs; un-ignore the test suite to prevent regressions.

## Dependencies at Risk

**`chrono-tz` DST gap handling pending upstream PR:**
- Risk: DST transition gap validation relies on a workaround because the proper fix (chrono-tz PR #188) has not been released. Datetimes in DST gaps are currently rejected entirely rather than adjusted.
- Files: `redical_ical/src/values/tzid.rs:68`
- Impact: Valid iCalendar datetimes that fall in DST transition windows are rejected with an error.
- Migration plan: Watch chrono-tz for PR #188 release; implement proper gap handling once available.

## Test Coverage Gaps

**`rebuild_indexed_geo` and `rebuild_indexed_class` untested:**
- What's not tested: Index rebuild logic for `GEO` and `CLASS` properties.
- Files: `redical_core/src/event.rs:578`, `redical_core/src/event.rs:585`
- Risk: Incorrect index state after event mutation would silently affect query results.
- Priority: Medium

**`inverted_index.rs` merge operations untested:**
- What's not tested: `merge_and` / `merge_or` correctness with edge cases.
- Files: `redical_core/src/inverted_index.rs`
- Risk: Index query results are incorrect; difficult to diagnose because failures surface in query output not in index logic.
- Priority: High

**`convert_error` function stubbed:**
- What's not tested: The `// TODO: Implement this...` comment at `lib.rs:99` means error conversion is incomplete; error messages returned to Redis clients may be incomplete or misleading.
- Files: `redical_ical/src/lib.rs:99`
- Risk: Users receive poor error feedback for malformed iCalendar input.
- Priority: Low

**`x_geo` context handling missing:**
- What's not tested: `// TODO: handle context.` at `x_geo.rs:77` means geo query property parsing does not respect rendering context.
- Files: `redical_ical/src/properties/query/x_geo.rs:77`
- Priority: Low

---

*Concerns audit: 2026-03-06*
