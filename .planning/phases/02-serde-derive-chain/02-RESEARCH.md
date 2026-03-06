# Phase 2: Serde Derive Chain - Research

**Researched:** 2026-03-06
**Domain:** Rust serde derive propagation across multi-crate type graph
**Confidence:** HIGH

## Summary

Phase 2 adds `Serialize`/`Deserialize` derives to all types reachable from `Calendar` so that `bincode::serialize(&calendar)` compiles. The work spans two crates: `redical_ical` (property/value types) and `redical_core` (Calendar, Event, and supporting structs). The main complexity is the sheer number of types (~40+ in redical_ical, ~8 in redical_core) and three special cases: (1) `Tzid` wrapping `chrono_tz::Tz` which lacks serde, (2) the `build_ical_param!` macro generating structs without serde derives, and (3) computed/index fields needing `#[serde(skip)]`.

The approach is mechanical: add `serde = { workspace = true }` to `redical_ical/Cargo.toml`, add chrono's serde feature to workspace, then iteratively add `#[derive(Serialize, Deserialize)]` guided by compiler errors. The only non-trivial code is the custom serde impl for `Tzid`.

**Primary recommendation:** Use compiler-driven discovery -- add derives to leaf types first (values), then properties, then redical_core types, fixing errors as they surface.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Only types reachable from Calendar's field graph get serde derives -- query-only types do NOT
- PassiveProperty and ContentLine data get serde derives
- All EventProperty enum variants get serde derives
- Value types serialize their parsed Rust representation, NOT raw iCal strings
- KeyValuePair (redical_core/src/utils.rs) gets serde derives
- Keep chrono pinned at 0.4.19, add serde feature: `chrono = { version = "0.4.19", features = ["serde"] }`
- Tzid: custom Serialize/Deserialize impl (serialize as timezone string name, deserialize by parsing back)
- All computed/index fields get `#[serde(skip)]` with code comments explaining skip rationale and rebuild_indexes() requirement
- Calendar-level skipped: indexed_categories, indexed_location_type, indexed_related_to, indexed_geo, indexed_class
- Event-level skipped: indexed_categories, indexed_location_type, indexed_related_to, indexed_geo, indexed_class
- ScheduleProperties skipped: parsed_rrule_set
- Phase 2 includes bincode round-trip smoke test

### Claude's Discretion
- Plan splitting strategy (one plan vs multiple)
- Exact order of type discovery (compiler-driven is fine)
- Where to place the smoke test (redical_core or redical_redis)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|-----------------|
| SERD-01 | `serde` dependency added to `redical_ical/Cargo.toml` | Workspace already defines `serde = { version = "1.0.162", features = ["derive"] }` -- just add `serde = { workspace = true }` |
| SERD-02 | Derive Serialize/Deserialize on all `redical_ical` property types in Calendar's field graph | ~40+ types identified across values/, properties/event/, content_line.rs; includes macro-generated types from `build_ical_param!` |
| SERD-03 | Derive Serialize/Deserialize on `redical_core` types | Calendar, Event, EventOccurrenceOverride, ScheduleProperties, IndexedProperties, PassiveProperties, KeyValuePair, GeoPoint |
| SERD-04 | `#[serde(skip)]` on all computed/index fields | 5 fields on Calendar, 5 on Event, 1 on ScheduleProperties; all default to None/Default already |
| SERD-05 | chrono serde feature enabled in workspace | Currently `chrono = "0.4.19"` without serde feature; needs `chrono = { version = "0.4.19", features = ["serde"] }` |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde | 1.0.162 | Serialization framework | Already in workspace with `derive` feature |
| bincode | 1.3.3 | Binary serialization format | Already in redical_redis; used for RDB fast-path |
| chrono | 0.4.19 | Date/time types | Already in workspace; needs `serde` feature added |

### No New Dependencies
This phase adds zero new crate dependencies. It only:
- Adds `serde = { workspace = true }` to `redical_ical/Cargo.toml`
- Adds `features = ["serde"]` to chrono in workspace `Cargo.toml`

## Architecture Patterns

### Type Graph Discovery Order

The Calendar field graph forms a dependency tree. Serde derives must be added bottom-up (leaf types first):

```
Layer 1 (leaf values):
  redical_ical/src/values/
    text.rs         -> Text(String)
    integer.rs      -> Integer(i64)
    float.rs        -> Float(f64)
    date.rs         -> Date { year, month, day }
    time.rs         -> Time { hour, min, sec }
    duration.rs     -> Duration, PositiveNegative (in grammar.rs)
    class.rs        -> ClassValue enum
    reltype.rs      -> Reltype enum
    tzid.rs         -> Tzid(Tz)  [CUSTOM IMPL]
    date_time.rs    -> DateTime enum, ValueType enum
    list.rs         -> List<T> (generic)
    recur.rs        -> Recur, Frequency, WeekDay, WeekDayNum,
                       + 14 macro-generated *Param types

Layer 2 (content line):
  redical_ical/src/content_line.rs
    ContentLineParam(String, String)
    ContentLineParams(Vec<ContentLineParam>)
    ContentLine(String, ContentLineParams, String)

Layer 3 (properties + params):
  redical_ical/src/properties/
    uid.rs          -> UIDProperty, UIDPropertyParams
    last_modified.rs -> LastModifiedProperty, LastModifiedPropertyParams
    event/dtstart.rs -> DTStartProperty, DTStartPropertyParams
    event/dtend.rs   -> DTEndProperty, DTEndPropertyParams
    event/duration.rs -> DurationProperty, DurationPropertyParams
    event/rrule.rs   -> RRuleProperty, RRulePropertyParams
    event/exrule.rs  -> ExRuleProperty, ExRulePropertyParams
    event/rdate.rs   -> RDateProperty, RDatePropertyParams
    event/exdate.rs  -> ExDateProperty, ExDatePropertyParams
    event/categories.rs -> CategoriesProperty, CategoriesPropertyParams
    event/location_type.rs -> LocationTypeProperty, LocationTypePropertyParams
    event/class.rs   -> ClassProperty, ClassPropertyParams
    event/geo.rs     -> GeoProperty, GeoPropertyParams
    event/related_to.rs -> RelatedToProperty, RelatedToPropertyParams
    event/passive.rs -> PassiveProperty enum (40+ variants)
    event/mod.rs     -> EventProperty enum, EventProperties
    calendar.rs      -> CalendarProperty enum

Layer 4 (core types):
  redical_core/src/
    utils.rs         -> KeyValuePair
    geo_index.rs     -> GeoPoint
    event.rs         -> ScheduleProperties, IndexedProperties,
                        PassiveProperties, Event
    event_occurrence_override.rs -> EventOccurrenceOverride
    calendar.rs      -> Calendar
```

### Pattern: Adding Derive to Existing Structs

Most types follow the same pattern -- add Serialize, Deserialize to the existing derive list:

```rust
// Before
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SomeProperty { ... }

// After
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SomeProperty { ... }
```

### Pattern: Custom Serde for Tzid

`chrono_tz::Tz` has no serde support at version 0.6.1 (used by this project). The `Tzid` newtype needs manual impl:

```rust
use serde::{Serialize, Deserialize, Serializer, Deserializer};

impl Serialize for Tzid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Tzid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let tz: Tz = s.parse().map_err(serde::de::Error::custom)?;
        Ok(Tzid(tz))
    }
}
```

### Pattern: Modifying build_ical_param! Macro

The `build_ical_param!` macro in `recur.rs` generates 14 param structs (FreqParam, UntilParam, CountParam, etc.) without serde derives. The macro must be updated:

```rust
// Before (line 20)
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct $struct_name(pub $value_type);

// After
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct $struct_name(pub $value_type);
```

This requires `use serde::{Serialize, Deserialize};` in scope at macro expansion sites. Since the macro is `#[macro_export]`, callers must import serde themselves. Currently the macro is only invoked in `recur.rs` itself, so adding the serde use to `recur.rs` is sufficient.

### Pattern: serde(skip) with Comments

```rust
// Computed index field -- rebuilt by rebuild_indexes() after deserialization.
// Not serialized because it's derived from indexed_properties, not source data.
#[serde(skip)]
pub indexed_categories: Option<InvertedEventIndex<String>>,
```

### Anti-Patterns to Avoid
- **Deriving on index types:** InvertedCalendarIndex, InvertedEventIndex, GeoSpatialCalendarIndex should NOT get serde derives. They are always rebuilt post-load.
- **Adding serde to query-only types:** Types in `properties/query/` and `values/where_*.rs` are NOT in Calendar's field graph.
- **Forgetting the macro:** The `build_ical_param!` macro silently generates structs -- missing it causes cryptic "doesn't implement Serialize" errors on Recur fields.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| NaiveDate/NaiveDateTime serialization | Custom date string handling | chrono's serde feature | chrono 0.4.19 serde feature handles all chrono types correctly |
| Enum variant serialization | Manual match-based serialization | serde derive on enums | Serde handles Rust enums natively with bincode |
| HashSet serialization | Manual iterator-based serialization | serde derive (HashSet has native serde support) | Hash-based collections serialize/deserialize automatically |
| BTreeMap/BTreeSet serialization | Manual sorted output | serde derive | Ordered collections have native serde support |

## Common Pitfalls

### Pitfall 1: chrono_tz::Tz Has No Serde
**What goes wrong:** Adding `#[derive(Serialize, Deserialize)]` to types containing `Tzid` fails because `chrono_tz::Tz` doesn't implement serde traits at v0.6.1 without serde feature.
**Why it happens:** chrono_tz 0.6.1 has a `serde` feature but this project doesn't enable it. Decision is to use custom impl on Tzid instead.
**How to avoid:** Write custom Serialize/Deserialize for Tzid before deriving on types that contain it (DTStartPropertyParams, DTEndPropertyParams, RDatePropertyParams, ExDatePropertyParams).
**Warning signs:** Compiler error mentioning `Tz` not implementing `Serialize`.

### Pitfall 2: Macro-Generated Types Missing Derives
**What goes wrong:** `Recur` struct contains 14 fields whose types are generated by `build_ical_param!`. Compiler errors point to Recur but the actual missing derives are in the macro output.
**Why it happens:** Macro-generated code is invisible in source -- easy to miss during manual derive addition.
**How to avoid:** Modify the `build_ical_param!` macro itself to include Serialize, Deserialize in derives.
**Warning signs:** Error on Recur struct saying FreqParam/CountParam etc. don't implement Serialize.

### Pitfall 3: Float(f64) Serde Compatibility
**What goes wrong:** `Float` wraps `f64` which does implement Serialize/Deserialize, but Float has manual `Eq` impl (f64 is not Eq). Serde derive still works fine -- Eq is not required for serde.
**How to avoid:** Just add the derive. No special handling needed.

### Pitfall 4: PositiveNegative in grammar.rs
**What goes wrong:** `Duration` struct contains `Option<PositiveNegative>`. This enum is defined in `grammar.rs`, not in the values module. Easy to miss.
**How to avoid:** The compiler will flag it. Add Serialize, Deserialize derive to PositiveNegative in grammar.rs.
**Warning signs:** Error on Duration saying PositiveNegative doesn't implement Serialize.

### Pitfall 5: Skipped Fields Without Default
**What goes wrong:** `#[serde(skip)]` requires the field type to implement Default for deserialization.
**Why it happens:** Serde needs to populate skipped fields with some value during deserialization.
**How to avoid:** All skipped fields already implement Default: `Option<T>` defaults to None, `InvertedCalendarIndex` and `GeoSpatialCalendarIndex` have Default impls. Calendar.indexes_active (bool) defaults to false -- but this field is NOT skipped, it's serialized.
**Warning signs:** None expected -- all skipped types already have Default.

### Pitfall 6: indexes_active Field on Calendar
**What goes wrong:** `Calendar.indexes_active: bool` is NOT an index field but controls whether indexing is active. It MUST be serialized (not skipped).
**How to avoid:** Only skip the five named index fields. indexes_active is source state, not computed.

## Code Examples

### Cargo.toml Changes

**Workspace root Cargo.toml:**
```toml
# Before
chrono = "0.4.19"

# After
chrono = { version = "0.4.19", features = ["serde"] }
```

**redical_ical/Cargo.toml:**
```toml
[dependencies]
serde = { workspace = true }
# ... existing deps unchanged
```

### Custom Tzid Serde (verified pattern)

```rust
// In redical_ical/src/values/tzid.rs
use serde::{Serialize, Deserialize, Serializer, Deserializer};

impl Serialize for Tzid {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for Tzid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        let tz: Tz = s.parse().map_err(serde::de::Error::custom)?;

        Ok(Tzid(tz))
    }
}
```

### Calendar Struct with Skip Annotations

```rust
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Calendar {
    pub uid: UIDProperty,
    pub events: BTreeMap<String, Box<Event>>,
    pub indexes_active: bool,

    // Computed index field -- rebuilt by rebuild_indexes() after deserialization.
    // Not serialized because it's derived from event properties, not source data.
    #[serde(skip)]
    pub indexed_categories: InvertedCalendarIndex<String>,

    // Computed index field -- rebuilt by rebuild_indexes() after deserialization.
    #[serde(skip)]
    pub indexed_location_type: InvertedCalendarIndex<String>,

    // Computed index field -- rebuilt by rebuild_indexes() after deserialization.
    #[serde(skip)]
    pub indexed_related_to: InvertedCalendarIndex<KeyValuePair>,

    // Computed index field -- rebuilt by rebuild_indexes() after deserialization.
    #[serde(skip)]
    pub indexed_geo: GeoSpatialCalendarIndex,

    // Computed index field -- rebuilt by rebuild_indexes() after deserialization.
    #[serde(skip)]
    pub indexed_class: InvertedCalendarIndex<String>,
}
```

### Bincode Smoke Test (recommended location: redical_redis)

```rust
#[cfg(test)]
mod serde_smoke_test {
    use super::*;

    #[test]
    fn test_calendar_bincode_round_trip() {
        let mut calendar = Calendar::new(String::from("TEST_UID"));

        let event = Event::parse_ical(
            "EVENT_UID",
            "RRULE:FREQ=WEEKLY;UNTIL=19700101T000500Z;INTERVAL=1 \
             CLASS:PUBLIC CATEGORIES:CATEGORY_ONE \
             DTSTART:19700101T000500Z \
             LAST-MODIFIED:19700101T010500Z",
        ).unwrap();

        calendar.insert_event(event);
        calendar.rebuild_indexes().unwrap();

        let bytes = bincode::serialize(&calendar).unwrap();
        let mut deserialized: Calendar = bincode::deserialize(&bytes).unwrap();
        deserialized.rebuild_indexes().unwrap();

        assert_eq!(calendar, deserialized);
    }
}
```

**Note:** Place in `redical_redis` since that crate already depends on bincode. Alternatively could add bincode as dev-dependency to redical_core.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| RDBCalendar iCal string round-trip | Direct bincode of Calendar struct | This phase | Enables fast-path serialization in Phase 3 |
| No serde in redical_ical | serde derives on all field-graph types | This phase | Foundation for binary serialization |

## Complete Type Inventory

### redical_ical Types Needing Derives (~42 types)

**Values (14 types + 14 macro types):**
- Text, Integer, Float, Date, Time, Duration, ClassValue, Reltype, ValueType, DateTime, List\<T\>
- Tzid (CUSTOM impl, no derive)
- PositiveNegative (in grammar.rs)
- Frequency, WeekDay, WeekDayNum, Recur
- 14 macro-generated Param types via build_ical_param!: FreqParam, UntilParam, CountParam, IntervalParam, BysecondParam, ByminuteParam, ByhourParam, BydayParam, BymonthdayParam, ByyeardayParam, ByweeknoParam, BymonthParam, BysetposParam, WkstParam

**Content Line (3 types):**
- ContentLineParam, ContentLineParams, ContentLine

**Properties (28 types: 14 property structs + 14 param structs):**
- UIDProperty + UIDPropertyParams
- LastModifiedProperty + LastModifiedPropertyParams
- DTStartProperty + DTStartPropertyParams
- DTEndProperty + DTEndPropertyParams
- DurationProperty + DurationPropertyParams
- RRuleProperty + RRulePropertyParams
- ExRuleProperty + ExRulePropertyParams
- RDateProperty + RDatePropertyParams
- ExDateProperty + ExDatePropertyParams
- CategoriesProperty + CategoriesPropertyParams
- LocationTypeProperty + LocationTypePropertyParams
- ClassProperty + ClassPropertyParams
- GeoProperty + GeoPropertyParams
- RelatedToProperty + RelatedToPropertyParams

**Enums (3 types):**
- PassiveProperty, EventProperty, CalendarProperty

**Collection wrapper (1 type):**
- EventProperties

### redical_core Types Needing Derives (8 types)

- Calendar (with 5 skip fields)
- Event (with 5 skip fields)
- EventOccurrenceOverride
- ScheduleProperties (with 1 skip field)
- IndexedProperties
- PassiveProperties
- KeyValuePair
- GeoPoint

### Types NOT Getting Derives

- InvertedCalendarIndex\<K\>, InvertedCalendarIndexTerm, InvertedEventIndex\<K\> (rebuilt post-load)
- GeoSpatialCalendarIndex (rebuilt post-load)
- IndexedConclusion (only in index types)
- All query types in properties/query/
- WhereOperator, WhereFromRangeOperator, WhereUntilRangeOperator, WhereRangeProperty (query-only values)
- RecurrenceIdProperty (not in Calendar's field graph -- used for override key parsing only)

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test + pretty_assertions_sorted |
| Config file | Cargo.toml (per-crate test sections) |
| Quick run command | `cargo test -p redical_redis serde_smoke_test` |
| Full suite command | `cargo test --workspace` |

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| SERD-01 | serde dependency compiles in redical_ical | compilation | `cargo check -p redical_ical` | N/A (compile check) |
| SERD-02 | All redical_ical types derive Serialize/Deserialize | compilation | `cargo check -p redical_ical` | N/A (compile check) |
| SERD-03 | All redical_core types derive Serialize/Deserialize | compilation | `cargo check -p redical_core` | N/A (compile check) |
| SERD-04 | Skip fields default correctly on deserialize | unit | `cargo test -p redical_redis serde_smoke_test` | Wave 0 |
| SERD-05 | chrono serde feature works | compilation | `cargo check -p redical_ical` | N/A (compile check) |
| SMOKE | bincode round-trip produces identical Calendar | unit | `cargo test -p redical_redis serde_smoke_test` | Wave 0 |

### Sampling Rate
- **Per task commit:** `cargo test --workspace`
- **Per wave merge:** `cargo test --workspace`
- **Phase gate:** Full suite green before `/gsd:verify-work`

### Wave 0 Gaps
- [ ] Bincode smoke test in redical_redis (or redical_core with bincode dev-dep)
- Existing test infrastructure covers compilation checks -- no new framework needed

## Open Questions

1. **Smoke test location**
   - What we know: bincode is already a dependency of redical_redis but not redical_core
   - Options: (a) Add test in redical_redis near rdb_data.rs tests, (b) Add bincode as dev-dep to redical_core
   - Recommendation: Place in redical_redis since bincode is already there and tests can reuse existing test Calendar construction patterns from rdb_data.rs tests

## Sources

### Primary (HIGH confidence)
- Codebase inspection: all type definitions, derive patterns, and field structures verified by reading source files
- `Cargo.toml` files: verified serde workspace definition, chrono version, chrono-tz version, bincode dependency location

### Secondary (MEDIUM confidence)
- [chrono-tz serde feature](https://docs.rs/chrono-tz/0.6.0/chrono_tz/) - confirmed serde feature exists but project chose custom Tzid impl per CONTEXT.md decision

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies already in workspace, just wiring
- Architecture: HIGH - type graph fully mapped from source inspection
- Pitfalls: HIGH - all edge cases (Tzid, macro, Float) verified in source

**Research date:** 2026-03-06
**Valid until:** 2026-04-06 (stable domain, no moving parts)
