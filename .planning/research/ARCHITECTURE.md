# Architecture Patterns

**Domain:** RDB serialization versioning — Rust Redis module (redical)
**Researched:** 2026-03-06

## Recommended Architecture

### Overview

`RDBCalendarDump` is a new envelope struct that wraps the existing `RDBCalendar`
(iCal string-based serialization) and adds a raw bincode blob of `Calendar` for
same-version fast-path loads. It lives in `redical_redis/src/datatype/rdb_data.rs`
alongside `RDBCalendar`, `RDBEvent`, and `RDBEventOccurrenceOverride`.

The fast-path works on an exact `GIT_SHA` match. When the version token is absent
or mismatches, the load falls through to the existing `RDBCalendar`-based iCal
parse path, which is already known-good.

---

### Component Boundaries

| Component | Crate | Responsibility | Changes Required |
|-----------|-------|---------------|-----------------|
| `RDBCalendarDump` | `redical_redis` | Envelope struct: `version`, `raw_dump`, `dump` fields; serde + bincode | New struct in `rdb_data.rs` |
| `rdb_save` | `redical_redis` | Serialize `Calendar` twice: raw bincode (`raw_dump`) + existing `RDBCalendar` (`dump`); wrap in `RDBCalendarDump`; write single blob | Replace body in `datatype/mod.rs` |
| `rdb_load` | `redical_redis` | Attempt `RDBCalendarDump` deserialization first; on success branch to fast or slow path; fall back to legacy `RDBCalendar` path on any bincode error | Replace body in `datatype/mod.rs` |
| `aof_rewrite` | `redical_redis` | No-op stub (remove `todo!()`) | One-line change in `datatype/mod.rs` |
| `Calendar` + nested types | `redical_core` | Add `#[derive(Serialize, Deserialize)]` across all fields reachable from `Calendar` | Multiple files in `redical_core/src/` |
| Fixture generator | `redical_redis` | Test-only binary/test that writes both legacy and new binary fixture files | New test or build script |
| Integration fixtures | workspace root | Pre-generated binary blobs committed to repo | New files in `tests/fixtures/` |
| Integration tests | workspace root | Load both fixture files; assert correct `Calendar` rehydration | New tests in `tests/integration.rs` |

---

### Data Flow — `rdb_save`

```
Calendar (in Redis memory)
  │
  ├─► bincode::serialize(&calendar)       → raw_dump: Vec<u8>
  │                                           (fast-path blob, Calendar + serde derives required)
  │
  ├─► RDBCalendar::try_from(&calendar)    → dump: RDBCalendar
  │       └─ iCal content-line render for each event + override
  │
  └─► RDBCalendarDump {
          version: option_env!("GIT_SHA").map(str::to_owned),
          raw_dump,
          dump,
      }
        │
        └─► bincode::serialize(&rdb_calendar_dump)
              │
              └─► raw::save_string(rdb, ...)
```

---

### Data Flow — `rdb_load`

```
bytes from Redis RDB stream
  │
  ├─ bincode::deserialize::<RDBCalendarDump>(bytes)
  │     ├─ Ok(dump_wrapper)
  │     │     │
  │     │     ├─ version matches GIT_SHA at current build?
  │     │     │     YES → std::panic::catch_unwind({
  │     │     │               bincode::deserialize::<Calendar>(&dump_wrapper.raw_dump)
  │     │     │           })
  │     │     │           ├─ Ok(Ok(calendar)) → return calendar   [FAST PATH]
  │     │     │           └─ _ (panic or Err) → fall through to slow path
  │     │     │
  │     │     └─ version absent or mismatch → fall through to slow path
  │     │           │
  │     │           └─ Calendar::try_from(&dump_wrapper.dump)     [SLOW PATH — iCal re-parse]
  │     │
  │     └─ Err(_) (legacy format — raw RDBCalendar bytes)
  │           │
  │           └─ bincode::deserialize::<RDBCalendar>(bytes)
  │                 └─ Calendar::try_from(&rdb_calendar)          [LEGACY PATH]
```

**Key invariant:** the legacy path is the unchanged existing path. It is reached
when the outer `bincode::deserialize::<RDBCalendarDump>` fails because the bytes
were written by an older build that only saved a bare `RDBCalendar`.

---

### Serde Derive Chain — Which Types Need `Serialize + Deserialize`

`bincode::serialize(&calendar)` on the raw fast-path requires `Serialize +
Deserialize` on `Calendar` and every type reachable from its fields.

#### `redical_core` crate — currently zero serde derives

Types requiring `#[derive(Serialize, Deserialize)]`:

| Type | File | Notes |
|------|------|-------|
| `Calendar` | `redical_core/src/calendar.rs` | Root — currently only `Debug, PartialEq, Clone` |
| `Event` | `redical_core/src/event.rs` | Stored in `Calendar.events: BTreeMap<String, Box<Event>>` |
| `ScheduleProperties` | `redical_core/src/event.rs` | Field of `Event`; contains `Option<rrule::RRuleSet>` |
| `IndexedProperties` | `redical_core/src/event.rs` | Field of `Event` |
| `PassiveProperties` | `redical_core/src/event.rs` | Field of `Event` |
| `EventOccurrenceOverride` | `redical_core/src/event_occurrence_override.rs` | Stored in `Event.overrides` |
| `InvertedCalendarIndex<K>` | `redical_core/src/inverted_index.rs` | Multiple typed fields on `Calendar` |
| `InvertedCalendarIndexTerm` | `redical_core/src/inverted_index.rs` | Inner type of `InvertedCalendarIndex` |
| `InvertedEventIndex<K>` | `redical_core/src/inverted_index.rs` | Fields on `Event` and `EventOccurrenceOverride` |
| `IndexedConclusion` | `redical_core/src/inverted_index.rs` | Enum: `Include(Option<HashSet<i64>>)` / `Exclude(...)` |
| `GeoSpatialCalendarIndex` | `redical_core/src/geo_index.rs` | Contains `RTree<GeomWithData<GeoPoint, ...>>` |
| `GeoPoint` | `redical_core/src/geo_index.rs` | `{lat: f64, long: f64}` — straightforward |

#### `redical_ical` crate — currently no serde dep

Every property type used in `Event`, `EventOccurrenceOverride`, `ScheduleProperties`,
`IndexedProperties`, and `PassiveProperties` also needs serde derives because those
structs own them directly (not via iCal string intermediaries on the fast path).

This covers property types in `redical_ical/src/properties/event/` and their
underlying value types in `redical_ical/src/values/`. The exact set must be
determined by following the compiler errors after adding the top-level derives —
this is the right approach since the set is large and attempting to enumerate all
leaf types upfront risks misses.

`redical_ical` currently has **no serde dependency at all**. Adding serde derives
to its types requires:

1. Adding `serde = { workspace = true }` to `redical_ical/Cargo.toml`
2. Adding `#[derive(Serialize, Deserialize)]` to all property and value types
   that appear as owned fields in the fast-path type graph

#### Third-party types — already have serde feature flags enabled

| Type | Crate | Feature flag | Status |
|------|-------|-------------|--------|
| `rrule::RRuleSet` | `rrule 0.10` | `features = ["serde"]` | Already in workspace Cargo.toml |
| `rstar::RTree<_>` | `rstar 0.11` | `features = ["serde"]` | Already in workspace Cargo.toml |
| `geo::Point<_>` (inside `GeomWithData`) | `geo 0.26` | `features = ["use-serde"]` | Already in workspace Cargo.toml |
| `chrono` types | `chrono 0.4` | Part of chrono feature set | Verify `serde` feature is included |

These are high-confidence (Cargo.toml is authoritative). The serde feature flags
are already present, so third-party types will derive without further config changes.

---

### Fixture File Placement

```
tests/
└── fixtures/
    ├── rdb_calendar_legacy.bin      # Raw bincode of RDBCalendar (old format)
    └── rdb_calendar_dump.bin        # Raw bincode of RDBCalendarDump (new format)
```

**Rationale:**

- Parallels the existing `tests/` integration test directory structure
- Not inside any crate's `src/` — these are not unit test concerns; they test the
  Redis module's load boundary
- Analogous to `redical_ical/tests/fuzz_finds/` — committed regression artifacts
  that cannot be regenerated at test time
- The generator (a `#[test]` or binary gated on `#[cfg(feature = "...")]`) writes
  to this path; the integration test reads from it

**Generator placement:** A `#[test]` function in `redical_redis/src/datatype/rdb_data.rs`
(gated behind `#[ignore]` so it does not run in CI automatically) that serializes a
known `Calendar` to both formats and writes the bytes to `tests/fixtures/`. Run once
locally to regenerate fixtures; commit the output.

---

### Patterns to Follow

#### Pattern 1: Dual-format envelope with version discriminator

**What:** `RDBCalendarDump` holds `version: Option<String>` (from `option_env!("GIT_SHA")`),
`raw_dump: Vec<u8>` (fast-path bincode of `Calendar`), and `dump: RDBCalendar`
(safe-path iCal string tree). The outer struct is what bincode actually serializes
to disk.

**When:** Every `rdb_save` call constructs this wrapper. The `raw_dump` is always
written (it is the speculative fast-path). The `dump` is always written (it is the
unconditional fallback). No flags gate the save — both blobs are always persisted.

#### Pattern 2: Layered deserialization fallback

**What:** `rdb_load` attempts to deserialize the outer envelope first. On success,
it inspects the version token. If the version matches the current build's `GIT_SHA`,
it attempts fast-path deserialization of `raw_dump` inside `catch_unwind`. Any
failure at any layer falls to the next layer rather than panicking.

**When:** Bincode is not self-describing; deserializing the wrong type layout can
produce garbage or panic. `catch_unwind` is the correct containment boundary because
bincode can trigger index-out-of-bounds panics on malformed data — it cannot be made
`Result`-returning for all failure modes.

#### Pattern 3: `option_env!` for build-time version token

**What:** `option_env!("GIT_SHA")` evaluated at compile time produces
`Option<&'static str>`. Map to `Option<String>` for storage. When absent (e.g.,
sandboxed CI environments), store `None` and always skip the fast path.

**When:** `GIT_SHA` is set by `redical_redis/build.rs` via `git rev-parse --short HEAD`.
The short SHA is sufficient — the fast path exists only for same-binary round-trips,
not cross-version upgrades.

---

### Anti-Patterns to Avoid

#### Anti-Pattern 1: Deriving serde on `Calendar` without auditing the full field graph

**What goes wrong:** Compiles only if every transitively-owned type also derives
`Serialize + Deserialize`. Missing a leaf type (e.g., a property struct in
`redical_ical`) produces a compile error on `Calendar`'s derive, not on the leaf —
the error message points at the wrong location and is confusing.

**Instead:** Add the derive to `Calendar` first, then let the compiler enumerate
missing derives bottom-up. Fix them in crate order: `redical_ical` → `redical_core`
→ compile. Do not guess the full set upfront.

#### Anti-Pattern 2: Using `unwrap()` inside `rdb_load` on the fast path

**What goes wrong:** A corrupt or version-mismatched `raw_dump` will panic the Redis
module process (taking down the Redis server).

**Instead:** Wrap fast-path deserialization in `std::panic::catch_unwind`. Log any
panic/failure at warning level and fall through to the slow path.

#### Anti-Pattern 3: Generating fixture bytes at test runtime

**What goes wrong:** If `Calendar`'s serde representation changes, a test that
generates its own fixture bytes will always agree with itself. The fixture becomes
meaningless as a backward-compat guard.

**Instead:** Commit pre-generated binary fixtures. The generator is a separate
`#[ignore]`-gated test run manually before committing a format change. CI then loads
the committed bytes, which will fail if the format drifts.

#### Anti-Pattern 4: Storing `raw_dump` bytes as the top-level RDB blob

**What goes wrong:** If bincode's representation of `Calendar` changes between Rust
or library versions, the old bytes are unreadable and there is no fallback.

**Instead:** Always wrap in `RDBCalendarDump` so the outer deserialization can
succeed even if `raw_dump` is stale, allowing fallback to `dump`.

---

### Build Order (what must be done before what)

```
Step 1 — Cargo.toml changes
    Add serde dependency to redical_ical/Cargo.toml
    (redical_core already has serde; redical_redis already has serde + bincode)

Step 2 — serde derives on redical_ical types
    All property and value types that appear in the Calendar field graph
    Compile redical_ical alone to verify

Step 3 — serde derives on redical_core types
    Calendar, Event, ScheduleProperties, IndexedProperties, PassiveProperties,
    EventOccurrenceOverride, InvertedCalendarIndex, InvertedCalendarIndexTerm,
    InvertedEventIndex, IndexedConclusion, GeoSpatialCalendarIndex, GeoPoint
    Compile redical_core alone to verify

Step 4 — RDBCalendarDump struct + updated rdb_save / rdb_load
    New struct in redical_redis/src/datatype/rdb_data.rs
    Updated hooks in redical_redis/src/datatype/mod.rs
    aof_rewrite stub (remove todo!())
    Compile redical_redis to verify

Step 5 — Fixture generator
    #[ignore]-gated test in rdb_data.rs that writes tests/fixtures/*.bin
    Run locally, commit fixture files

Step 6 — Integration tests
    Tests in tests/integration.rs that load both fixture files and assert
    correct Calendar rehydration
    Must run after fixture files are committed
```

---

### Scalability Considerations

The fast-path's safety properties are version-scoped. The bincode layout of
`Calendar` is not stable across library updates (rrule, rstar, chrono may change
their serde output). The `GIT_SHA` discriminator provides exact binary identity but
has a narrow scope: it is safe only for same-binary RDB round-trips within a single
Redis instance lifetime. Cross-version RDB migrations always use the `dump` (iCal)
path, which is stable by design.

The fixture format issue: binary fixtures committed to the repo will diverge from
the live format as soon as any serde-derived type changes its representation. The
`#[ignore]`-gated regenerator addresses this. Document in the test file that fixtures
must be regenerated whenever the fast-path serialization surface changes.

---

## Sources

- `redical_redis/src/datatype/rdb_data.rs` — existing `RDBCalendar`, `RDBEvent`, `RDBEventOccurrenceOverride` (read 2026-03-06)
- `redical_redis/src/datatype/mod.rs` — current `rdb_load`, `rdb_save`, `aof_rewrite` hooks (read 2026-03-06)
- `redical_core/src/calendar.rs` — `Calendar` struct fields (read 2026-03-06)
- `redical_core/src/event.rs` — `Event`, `ScheduleProperties`, `IndexedProperties`, `PassiveProperties` (read 2026-03-06)
- `redical_core/src/inverted_index.rs` — `IndexedConclusion`, `InvertedCalendarIndex`, `InvertedEventIndex` (read 2026-03-06)
- `redical_core/src/geo_index.rs` — `GeoSpatialCalendarIndex`, `GeoPoint` (read 2026-03-06)
- `Cargo.toml` (workspace) — confirms `rrule` serde feature, `rstar` serde feature, `geo` use-serde feature all present (read 2026-03-06)
- `redical_redis/build.rs` — confirms `GIT_SHA` set via `git rev-parse --short HEAD` (read 2026-03-06)
- `.planning/PROJECT.md` — project requirements and constraints (read 2026-03-06)
