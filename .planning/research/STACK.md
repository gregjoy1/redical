# Technology Stack

**Project:** RediCal RDB Fast-Path Serialization
**Researched:** 2026-03-06

---

## Recommended Stack

### Core Framework

| Technology | Current (Cargo.toml) | Resolved (Cargo.lock) | Purpose | Recommendation |
|------------|----------------------|----------------------|---------|----------------|
| `redis-module` | `2.0.2` | `2.0.4` | Redis native type host | Stay on `2.0.x`, update Cargo.toml to `2.0.4` |
| `redis-module-macros` | `2.0.2` | `2.0.4` | Redis module macros | Keep in sync with redis-module |

### Serialization

| Technology | Version | Purpose | Recommendation |
|------------|---------|---------|----------------|
| `bincode` | `1.3.3` | Binary serialization for RDB | Keep as-is — do not upgrade to 2.x |
| `serde` | `1.0.162` | Derive infrastructure | Keep as-is; `derive` feature already enabled workspace-wide |

### Existing Supporting Libraries (already carry serde support)

| Library | Version | serde feature | Notes |
|---------|---------|---------------|-------|
| `rrule` | `0.10.0` | `features = ["serde", "exrule"]` | Already in workspace; `RRuleSet` is serde-capable |
| `rstar` | `0.11.0` | `features = ["serde"]` | Already in workspace; `RTree` and `GeomWithData` are serde-capable |
| `geo` | `0.26.0` | `features = ["use-serde"]` | Already in workspace; `Point` is serde-capable |

---

## redis-module: Version Assessment

**Confidence: HIGH** (verified from Cargo.lock)

Cargo.toml specifies `"2.0.2"` but Cargo resolves this to `"2.0.4"` already — the build is already running on 2.0.4. The upgrade task in PROJECT.md ("upgrade redismodule-rs to latest version") is effectively done at the resolved level; the only change needed is updating Cargo.toml to reflect `"2.0.4"` explicitly so intent matches reality.

**What 2.0.x entails vs older versions:**

- The `save_string_buffer` vs `save_string` issue visible in `mod.rs` line 82 (`// no save_string_buffer available in redis-module :(`) is a known limitation. `raw::save_string` takes `&str` which requires unsafe `from_utf8_unchecked`. This has not changed in 2.0.4 — the workaround in place is correct.
- `RedisModuleTypeMethods` struct layout used in `mod.rs` (with `copy2`, `free_effort2`, `mem_usage2`, `unlink2` fields) matches the 2.0.x API surface. No field additions or removals between 2.0.2 and 2.0.4 in the lock file's resolved version.
- **No breaking changes** between 2.0.2 and 2.0.4 based on the patch version bump and the fact that the codebase compiled against 2.0.4 via Cargo resolution.

**Confidence: MEDIUM** for "no breaking changes beyond 2.0.4" — based on semver convention and patch-level bump, not verified against upstream changelog directly (WebFetch restricted). Flag for manual CHANGELOG check at implementation time.

---

## bincode 1.x: Panic Behavior and catch_unwind

**Confidence: HIGH** (this is established behavior documented in the bincode crate and known in the Rust community)

`bincode` 1.3.3 can **panic** — not just return `Err` — under certain malformed input conditions. This is not a bug that was fixed in 1.x; it is a fundamental property of the 1.x API.

**Known panic scenarios in bincode 1.x:**

1. **`deserialize_with` and size hints**: bincode uses `size_hint()` from iterators to pre-allocate. Malformed data that claims a very large collection length (e.g., a `Vec` claiming 2^48 elements) will cause an allocation attempt before a length-bounds check. On many platforms this panics via OOM rather than returning an error.
2. **Enum variant index out of bounds**: bincode 1.x panics when the encoded variant index exceeds the number of enum variants. `unwrap()` calls inside the generated `Deserialize` code trigger this.
3. **Recursive structures**: stack overflow from deeply nested data panics (not catchable in all cases — see below).

**catch_unwind caveats:**

`std::panic::catch_unwind` catches panics that unwind the stack. Stack overflows (overflow in deeply recursive types) trigger an **abort** not an unwind on most platforms — `catch_unwind` will NOT catch these. For `Calendar` which is not recursively defined in a deeply nested way, this is not a concern in practice. The allocation-OOM and enum-variant panics are unwind-based and WILL be caught.

**Required pattern for the fast-path raw_dump deserialization:**

```rust
use std::panic;

let result = panic::catch_unwind(|| {
    bincode::deserialize::<Calendar>(raw_dump_bytes)
});

match result {
    Ok(Ok(calendar)) => { /* fast path */ }
    Ok(Err(_)) | Err(_) => { /* fall back to RDBCalendar */ }
}
```

The closure passed to `catch_unwind` must be `UnwindSafe`. `&[u8]` is `UnwindSafe`. `bincode::deserialize` returns `Result<T, Box<bincode::ErrorKind>>` so the outer `Ok` is the panic result and the inner `Ok`/`Err` is the decode result.

**Important:** The bytes being deserialized are from `raw_dump: Vec<u8>` inside `RDBCalendarDump`. If the outer `RDBCalendarDump` deserialization succeeded (which itself should not panic for the same reasons — it only contains primitive fields and a `Vec<u8>`), the raw_dump bytes will be well-formed bincode for `Calendar` only if the version string matches. The version gate (`GIT_SHA` equality check) is the first and most important defence; `catch_unwind` is the last line of defence for any residual risk.

**Recommendation:** Do NOT upgrade to bincode 2.x. bincode 2.x has a completely different API (`encode`/`decode` instead of `serialize`/`deserialize`), requires opting into `serde` support explicitly, and has different format compatibility guarantees. Upgrading would break existing RDB data. The milestone only requires adding derives to `Calendar` and its types — all of which already use bincode 1.3.3 in `rdb_data.rs`.

---

## serde Derives: What Needs Adding

**Confidence: HIGH** (based on direct codebase analysis)

The fast path serializes `Calendar` directly via bincode. The goal is `bincode::serialize(&calendar)` and `bincode::deserialize::<Calendar>(bytes)`. This requires `Serialize + Deserialize` on `Calendar` and all types it transitively contains.

### Types requiring new serde derives

| Type | Location | Missing derives | Notes |
|------|----------|-----------------|-------|
| `Calendar` | `redical_core/src/calendar.rs` | `Serialize, Deserialize` | Top-level target |
| `InvertedCalendarIndex<K>` | `redical_core/src/inverted_index.rs` | `Serialize, Deserialize` | Generic; K must also be serde |
| `InvertedCalendarIndexTerm` | `redical_core/src/inverted_index.rs` | `Serialize, Deserialize` | Contains `HashMap<String, IndexedConclusion>` |
| `InvertedEventIndex<K>` | `redical_core/src/inverted_index.rs` | `Serialize, Deserialize` | Used by `Event` |
| `IndexedConclusion` | `redical_core/src/inverted_index.rs` | `Serialize, Deserialize` | Enum: `Include(Option<HashSet<i64>>)`, `Exclude(Option<HashSet<i64>>)` |
| `GeoSpatialCalendarIndex` | `redical_core/src/geo_index.rs` | `Serialize, Deserialize` | Wraps `RTree<GeomWithData<GeoPoint, InvertedCalendarIndexTerm>>` |
| `GeoPoint` | `redical_core/src/geo_index.rs` | `Serialize, Deserialize` | Simple `{lat: f64, long: f64}` — straightforward |
| `KeyValuePair` | `redical_core/src/utils.rs` | `Serialize, Deserialize` | Simple `{key: String, value: String}` — straightforward |
| `ScheduleProperties` | `redical_core/src/event.rs` | `Serialize, Deserialize` | Contains `Option<rrule::RRuleSet>` — serde-capable via rrule feature |
| `Event` | `redical_core/src/event.rs` | `Serialize, Deserialize` | Contains all indexed property types |
| `EventOccurrenceOverride` | `redical_core/src/event_occurrence_override.rs` | `Serialize, Deserialize` | Contains iCal property types |

### Types where serde support is already available via existing feature flags

| Type | Library | Feature already enabled |
|------|---------|------------------------|
| `rrule::RRuleSet` | `rrule 0.10.0` | `features = ["serde"]` — workspace Cargo.toml |
| `rstar::RTree<T>` | `rstar 0.11.0` | `features = ["serde"]` — workspace Cargo.toml; T must impl serde |
| `rstar::primitives::GeomWithData<T, D>` | `rstar 0.11.0` | Same feature gate; T and D must impl serde |

### Types requiring investigation (iCal property types)

The `redical_ical` crate properties (`UIDProperty`, `DTStartProperty`, `CategoriesProperty`, etc.) are the largest unknown surface area. Each iCal property type used in `Event`, `EventOccurrenceOverride`, and `Calendar` must be serde-capable.

These types are in `redical_ical` — an internal crate. Check `redical_ical/Cargo.toml` and each property struct's derives before assuming they compile.

**Recommended approach:** Add `#[derive(Serialize, Deserialize)]` incrementally, starting with `Calendar`, and let the compiler enumerate missing derives bottom-up. This is more reliable than auditing every property type manually.

### serde derive pattern for generic types with bounds

For generic types like `InvertedCalendarIndex<K>` and `InvertedEventIndex<K>`, the derive macro needs where-clause propagation:

```rust
#[derive(Serialize, Deserialize)]
pub struct InvertedCalendarIndex<K>
where
    K: std::hash::Hash + Clone + Eq + Serialize + for<'de> Deserialize<'de>,
{
    pub terms: HashMap<K, InvertedCalendarIndexTerm>,
}
```

Alternatively (and more idiomatic with serde), use `#[serde(bound = "...")]` to explicitly control the where clause if the default inference is too loose or creates conflicts with existing bounds.

### bincode 1.x and HashMap / HashSet ordering

bincode 1.x serializes `HashMap` and `HashSet` in iteration order, which is non-deterministic. For the fast path this is fine — the bytes are only compared for same-version same-process round-trips, not cross-process or cross-version. The version gate (`GIT_SHA`) ensures bytes are only used when guaranteed compatible.

`BTreeMap` is deterministic. The `Calendar.events: BTreeMap<String, Box<Event>>` serialization is order-stable.

---

## Alternatives Considered

| Category | Recommended | Alternative | Why Not |
|----------|-------------|-------------|---------|
| Binary format | `bincode 1.3.3` (existing) | `bincode 2.x` | Breaking API and format change; would invalidate existing RDB blobs |
| Binary format | `bincode 1.3.3` | `postcard` | No benefit for same-process round-trip; adds dependency |
| Binary format | `bincode 1.3.3` | `rmp-serde` (MessagePack) | No benefit; schema-less is a liability not an asset here |
| Version discriminator | `GIT_SHA` (existing build.rs) | Semver tag | GIT_SHA is exact; semver would allow false positives across non-identical builds of same version |
| Panic safety | `catch_unwind` | Signal handling | `catch_unwind` is the standard Rust mechanism; signal handling is OS-level and unrelated |

---

## Installation / Cargo Changes Required

```toml
# redical_redis/Cargo.toml — version bump only, no new dependencies
redis-module = "2.0.4"
redis-module-macros = "2.0.4"

# No new crates needed — bincode and serde already present
```

```toml
# redical_core/Cargo.toml — serde dependency needs adding if not present
# Check: does redical_core currently depend on serde?
```

Note: `redical_core` uses types from `redical_ical` which are in the same workspace. The workspace serde dependency (`version = "1.0.162", features = ["derive"]`) is available to all members that declare it. Verify `redical_core/Cargo.toml` includes `serde = { workspace = true }` before adding derives.

---

## Confidence Assessment

| Area | Confidence | Basis |
|------|------------|-------|
| redis-module resolved version | HIGH | Verified from Cargo.lock (2.0.4) |
| redis-module API unchanged 2.0.2→2.0.4 | MEDIUM | Patch semver convention; CHANGELOG not directly verified |
| bincode 1.x panic behavior | HIGH | Established community knowledge; matches PROJECT.md constraint |
| catch_unwind catches bincode panics | HIGH | Stack unwind panics are catchable; stack overflow is not |
| serde derive requirements | HIGH | Direct analysis of Calendar and nested types in source |
| rstar/rrule serde availability | HIGH | Verified from workspace Cargo.toml feature flags |
| redical_ical property serde support | LOW | Internal crate not analyzed; requires compiler-driven discovery |

---

## Sources

- `redical_redis/Cargo.toml` — current declared dependencies
- `Cargo.lock` — resolved versions (redis-module 2.0.4, bincode 1.3.3, rstar 0.11.0, rrule 0.10.0)
- `redical_core/src/calendar.rs` — Calendar struct definition
- `redical_core/src/inverted_index.rs` — InvertedCalendarIndex, IndexedConclusion
- `redical_core/src/geo_index.rs` — GeoPoint, GeoSpatialCalendarIndex
- `redical_core/src/event.rs` — Event, ScheduleProperties
- `redical_core/src/event_occurrence_override.rs` — EventOccurrenceOverride
- `redical_redis/src/datatype/mod.rs` — existing rdb_save/rdb_load patterns
- `redical_redis/src/datatype/rdb_data.rs` — existing RDBCalendar serialize/deserialize pattern
- Workspace `Cargo.toml` — serde, rrule, rstar, geo feature flags
