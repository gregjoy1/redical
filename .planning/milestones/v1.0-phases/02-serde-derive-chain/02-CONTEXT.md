# Phase 2: Serde Derive Chain - Context

**Gathered:** 2026-03-06
**Status:** Ready for planning

<domain>
## Phase Boundary

Add `Serialize`/`Deserialize` derives across the full `Calendar` type graph so `bincode::serialize(&calendar)` compiles. Annotate computed/index fields with `#[serde(skip)]`. No RDB format changes — Phase 3 handles the envelope and load/save logic.

</domain>

<decisions>
## Implementation Decisions

### Derive scope in redical_ical
- Only types reachable from Calendar's field graph get serde derives — event properties, their params, value types, ContentLine
- Query-only types (XGeoProperty, XLocationTypeProperty, WHERE/ORDER/RANGE types) do NOT get derives
- Exception: if a shared value type is reachable from Calendar's graph AND used by query types, it still gets the derive (compiler-driven)
- PassiveProperty and its ContentLine data get serde derives — they're in Event's field graph
- All EventProperty enum variants get serde derives — all are needed for complete bincode round-trip
- Value types serialize their parsed Rust representation, NOT raw iCal strings — this is the whole point of the fast path (avoid re-parsing)
- KeyValuePair (redical_core/src/utils.rs) gets serde derives — it's in Calendar's field graph via indexed_related_to

### Chrono serde feature
- Keep chrono pinned at 0.4.19, add serde feature: `chrono = { version = "0.4.19", features = ["serde"] }`
- Minimal change, no version bump risk

### Custom serde for Tzid
- Tzid wraps chrono_tz::Tz which has no serde support
- Custom Serialize/Deserialize impl: serialize as timezone string name (e.g. "America/New_York"), deserialize by parsing back
- No new dependencies (serde_with not needed)

### Skipped fields strategy
- All computed/index fields get `#[serde(skip)]` with a code comment on each explaining:
  - Why it's skipped (computed/cached, not source data)
  - That `rebuild_indexes()` must be called after deserialization to repopulate
- Calendar-level skipped fields: `indexed_categories`, `indexed_location_type`, `indexed_related_to`, `indexed_geo`, `indexed_class`
- Event-level skipped fields: `indexed_categories`, `indexed_location_type`, `indexed_related_to`, `indexed_geo`, `indexed_class` (all `Option<InvertedEventIndex<T>>`)
- ScheduleProperties skipped field: `parsed_rrule_set: Option<RRuleSet>` — comment explains it's rebuilt from RRULE/EXRULE/RDATE/EXDATE properties
- All skipped types already implement Default — `Option<T>` defaults to None, `InvertedCalendarIndex` and `GeoSpatialCalendarIndex` have Default impls

### Bincode smoke test
- Phase 2 includes a basic round-trip smoke test: serialize Calendar -> deserialize -> rebuild_indexes() -> assert equality with original
- Verifies the full chain (derives + skip + rebuild) works before Phase 3 builds on it

### Claude's Discretion
- Plan splitting strategy (one plan vs multiple)
- Exact order of type discovery (compiler-driven is fine)
- Where to place the smoke test (redical_core or redical_redis)

</decisions>

<specifics>
## Specific Ideas

- Code comments are required on every `#[serde(skip)]` field explaining the skip rationale and rebuild_indexes() requirement
- Comment on `ScheduleProperties::parsed_rrule_set` specifically documenting it as a cached/computed field

</specifics>

<code_context>
## Existing Code Insights

### Reusable Assets
- `Calendar::rebuild_indexes()` at calendar.rs:113 — full clean rebuild: clears all indexes, iterates events, calls event.rebuild_indexes(), repopulates calendar-level inverted indexes
- `Event::rebuild_indexes()` at event.rs:492 — rebuilds all 5 event-level index fields from indexed_properties
- Workspace `serde = { version = "1.0.162", features = ["derive"] }` already defined — redical_ical just needs `serde = { workspace = true }`
- `rrule` crate has serde feature enabled — RRuleSet can serialize but we're skipping it
- `rstar` has serde feature enabled — but GeoSpatialCalendarIndex is skipped anyway
- `geo` has `use-serde` feature enabled

### Established Patterns
- All redical_ical types implement `ICalendarEntity` trait (parse + render) — serde derives are additive, no conflict
- `impl_icalendar_entity_traits!` macro generates FromStr/Display — orthogonal to serde
- Custom Hash impls exist on RDateProperty, ExDateProperty, CategoriesProperty, PassiveProperty, GeoPoint — no conflict with serde derives

### Integration Points
- `redical_ical/Cargo.toml` — needs `serde = { workspace = true }` added to dependencies
- `Cargo.toml` (workspace root) — chrono needs `features = ["serde"]` added
- ~40+ structs/enums in redical_ical need derives (properties, params, values, ContentLine)
- ~8 structs in redical_core need derives (Calendar, Event, EventOccurrenceOverride, ScheduleProperties, IndexedProperties, PassiveProperties, KeyValuePair, GeoPoint)
- GeoPoint (geo_index.rs) — needs serde derive added (currently only Debug, Clone + manual Hash/Eq/PartialEq)
- InvertedEventIndex<K> — already has Default impl; used as Option so skip defaults to None

### Verified No-Blockers
- InvertedEventIndex<K> Default: implemented for all K bounds (Hash + Clone + Eq)
- GeoPoint: plain f64 fields, trivial serde derive
- HashSet<PropertyType>: serde handles natively, Hash impls not involved in serialization

</code_context>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>

---

*Phase: 02-serde-derive-chain*
*Context gathered: 2026-03-06*
