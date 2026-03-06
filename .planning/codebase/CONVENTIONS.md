# Coding Conventions

**Analysis Date:** 2026-03-06

## Naming Patterns

**Files:**
- `snake_case` for all Rust source files: `date_time.rs`, `inverted_index.rs`, `rdcl_evt_set.rs`
- Redis command handler files are named after the command they implement: `rdcl_evt_set.rs`, `rdcl_evi_query.rs`
- Test helper files named `utils.rs` and `macros.rs` within a `testing/` subdirectory

**Functions:**
- `snake_case` for all functions: `parse_ical`, `render_ical_with_context`, `build_event_from_ical`
- Parser functions named after the grammar rule they parse: `date_time`, `escaped_char`, `tsafe_char`
- Predicate functions prefixed with `is_`: `is_tsafe_char`, `is_safe_char`, `is_utc`, `is_blank`, `is_present`
- Builder/constructor helpers prefixed with `build_`: `build_event_from_ical`, `build_parsed_rrule_set`
- Extraction helpers prefixed with `extract_`: `extract_all_category_strings`, `extract_geo_point`
- Getter methods prefixed with `get_`: `get_tzid`, `get_date_time`, `get_categories`, `get_utc_timestamp`

**Types (structs, enums, traits):**
- `PascalCase` for all type names: `DTStartProperty`, `XCategoriesProperty`, `InvertedCalendarIndexTerm`
- Property param structs named `{PropertyName}PropertyParams`: `DTStartPropertyParams`, `XCategoriesPropertyParams`
- Property structs named `{PropertyName}Property`: `DTStartProperty`, `XCategoriesProperty`
- Traits named as adjective/noun forms: `ICalendarEntity`, `ICalendarProperty`, `ICalendarComponent`, `QueryableEntity`

**Variables:**
- `snake_case` throughout, with descriptive full names: `event_occurrence_override`, `calendar_uid`, `context_adjusted_date_time`
- No abbreviations — `content_line_params` not `clp`, `parsed_event_uid` not `uid`
- Iterator/loop variables take the singular form of the collection name: `for (event_uid, indexed_conclusion) in events`

**Constants:**
- `SCREAMING_SNAKE_CASE`: `CALENDAR_DATA_TYPE`, `CONFIGURATION_ICAL_PARSER_TIMEOUT_MS`

## Code Style

**Formatting:**
- Rust edition 2021
- `rustfmt.toml` contains only `edition = "2021"` — default rustfmt settings otherwise
- Trailing commas used consistently in function args, struct literals, and macro invocations
- Blank lines used between logical groups within functions and between match arms containing multi-line expressions

**Whitespace:**
- Blank line after guard clauses / early returns before the next meaningful line
- Blank lines separate match arms that contain multi-line bodies
- Blank line between struct field groups when fields serve different conceptual purposes

**Alignment:**
- Match arm patterns with similar variants are vertically aligned when their values differ only in the variant:
  ```rust
  (ValueType::DateTime, DateTime::UtcDateTime(_))   => Ok(()),
  (ValueType::DateTime, DateTime::LocalDateTime(_)) => Ok(()),
  (ValueType::Date,     DateTime::LocalDate(_))     => Ok(()),
  ```

**Linting:**
- Clippy is used (evidenced by recent commit "Resolve Clippy infractions")
- Mismatched lifetime syntax warnings addressed (recent fix)

## Import Organization

**Order:**
1. Standard library (`use std::...`)
2. External crate imports (`use nom::...`, `use chrono::...`)
3. Blank line
4. Internal crate imports (`use crate::...`)

**Grouping:** Related imports from the same crate are grouped with multi-line `use` blocks rather than repeated single imports:
```rust
use nom::combinator::{recognize, map, map_res, opt, cut};
use nom::sequence::{pair, preceded};
```

**Path aliases:** None used — full paths throughout.

## Error Handling

**Parser errors:** Custom `ParserError` type in `redical_ical/src/lib.rs` wraps `nom` errors with structured span, message, and context fields.

**Patterns:**
- Parser functions return `ParserResult<T>` which is `nom::IResult<ParserInput, T, ParserError>`
- Hard failures use `cut()` after the initial identifying tag — prevents backtracking once committed to a parse branch
- Enriched error messages via the `map_err_message!` macro: clears context and sets a human-readable message
- Validation errors propagated via `Result<(), String>` — simple string errors for caller display
- `?` operator used for propagation; `unwrap()` only appears in test utilities and `panic!` in `From<i64>` where truly unrecoverable
- Redis command handlers map errors to `RedisError::String(...)` at the boundary

**Error formatting:**
- `convert_error` produces single-line error strings for Redis-friendliness: `"Error - {message} at \"{span}\" -- Context: {context_chain}"`

## Logging

**Framework:** Redis module context logging via `ctx.log_debug(...)` and `ctx.log_warning(...)`

**Patterns:**
- Debug logs at command entry with key arguments: `"rdcl.evt_set: key: {calendar_uid} event uid: {event_uid}"`
- Warning logs for timeout/unexpected conditions
- No logging in the `redical_core` or `redical_ical` layers — logging is a Redis command layer concern only

## Comments

**RFC compliance comments:** iCalendar property files include the RFC 5545 grammar notation as comments above the parser functions and struct definitions. This is the primary documentation format for the parsing layer.

**Doc comments (`///`):**
- Used for public utility functions and trait methods where behaviour needs elaboration: `/// Converts the DateTime to the provided timezone.`
- Includes doc-test examples in `///` blocks for public parser utilities (`map_err`, `terminated_lookahead`)
- `// TODO:` comments mark known incomplete work (see CONCERNS.md)

**Inline comments:** Brief explanatory notes on non-obvious logic, e.g. timezone fallback chains, match arm groupings.

## Traits and Implementations

**Core trait pattern:** `ICalendarEntity` is the central trait — all parseable/renderable types implement it:
```rust
pub trait ICalendarEntity {
    fn parse_ical(input: ParserInput) -> ParserResult<Self> where Self: Sized;
    fn render_ical_with_context(&self, context: Option<&RenderingContext>) -> String;
    fn render_ical(&self) -> String { self.render_ical_with_context(None) }
    fn validate(&self) -> Result<(), String> { Ok(()) }
}
```

**Macro-generated trait impls:** The `impl_icalendar_entity_traits!(TypeName)` macro generates `FromStr` and `Display` implementations for all entity types. Use this macro for every new iCalendar entity type.

**`define_property_params_ical_parser!` macro:** Used inside `ICalendarEntity` implementations for property params structs. Generates a `parse_ical` that iterates semicolon-separated params and dispatches to the provided handler closures.

## Module Design

**Exports:** Public items are re-exported from `mod.rs` using `pub use submodule::*;` — callers import from the module, not the file.

**Crate boundaries:**
- `redical_ical` — pure iCalendar parsing/rendering, no Redis or core business logic
- `redical_core` — calendar data model, indexing, querying; depends on `redical_ical`
- `redical_redis` — Redis module command handlers; depends on both above crates

**Test modules:**
- Unit tests are in `#[cfg(test)] mod tests { ... }` at the bottom of each source file
- Integration test helpers live in `redical_core/src/testing/` (re-exported under `#[cfg(test)]`)
- Integration tests against a live Redis server live in `tests/integration.rs` at the workspace root

---

*Convention analysis: 2026-03-06*
