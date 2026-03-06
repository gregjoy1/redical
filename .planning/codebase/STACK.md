# Technology Stack

**Analysis Date:** 2026-03-06

## Languages

**Primary:**
- Rust (edition 2021) - all crates: `redical_core`, `redical_ical`, `redical_redis`, `redical_ical_afl_fuzz_targets`

## Runtime

**Environment:**
- Native compiled binary (cdylib shared object: `libredical.so` / `libredical.dylib`)
- Loaded as a Redis module into a running Redis server process

**Package Manager:**
- Cargo (Rust toolchain `stable`)
- Lockfile: `Cargo.lock` present and committed

## Frameworks

**Core:**
- `redis-module` 2.0.2 - Redis module SDK; exposes Rust code as Redis commands and custom data types
- `redis-module-macros` 2.0.2 - Macros companion for `redis-module`

**Testing:**
- Rust built-in test harness (`cargo test`) - unit and integration tests
- `pretty_assertions_sorted` 1.2.3 - enhanced diff output in test failures

**Build/Dev:**
- `make` - build orchestration (`Makefile`)
- Docker - multi-stage build (`Dockerfile`); builds release `.so` and embeds into `redis:7.0` image
- `ramp-packer` (Python, pip) - packages module for Redis Enterprise (`ramp.yml`)

## Key Dependencies

**Critical:**
- `redis-module` 2.0.2 - entire Redis integration layer; `redical_redis/Cargo.toml`
- `nom` 6.0 (core) / 7.1.3 + `nom_locate` 4.2.0 (ical) - parser combinator for iCalendar grammar; `redical_core/Cargo.toml`, `redical_ical/Cargo.toml`
- `rrule` 0.10 (features: `serde`, `exrule`) - RFC 5545 recurrence rule evaluation; `redical_core/Cargo.toml`
- `chrono` 0.4.19 + `chrono-tz` 0.6.1 - datetime handling with timezone support; all crates
- `rstar` 0.11.0 (features: `serde`) - R*-tree spatial index for geographic queries; `redical_core/Cargo.toml`
- `geo` 0.26.0 + `geohash` 0.13.0 - geometric types and geohash encoding for geo indexing; `redical_core/Cargo.toml`
- `rayon` 1.10.0 - data parallelism for query execution; `redical_redis/Cargo.toml`
- `bincode` 1.3.3 - binary serialisation for Redis RDB persistence; `redical_redis/Cargo.toml`
- `serde` 1.0.162 (features: `derive`) - serialisation throughout; all crates

**Infrastructure:**
- `redis` 0.23 - Rust Redis client used in integration tests; dev-dependency in workspace root and `redical_core`, `redical_redis`
- `libc` 0.2 - FFI types for Redis allocator interop; `redical_redis/Cargo.toml`
- `lazy_static` 1.4.0 - static initialisation; `redical_core`, `redical_redis`
- `itertools` 0.12.1 - iterator utilities; `redical_ical`
- `unicode-segmentation` 1.10.1 - Unicode-aware string splitting; `redical_core`, `redical_ical`
- `regex` 1.5.5 (features: `perf`, `std`, no default) - pattern matching; `redical_core`
- `num` 0.4.1 - numeric traits; `redical_core`
- `afl` (any) - American Fuzzy Lop fuzzing harness; `redical_ical_afl_fuzz_targets`
- `anyhow` 1 - error handling in tests; dev-dependency

## Configuration

**Environment:**
- No application-level environment variables; the module is configured entirely through Redis config commands
- `REDICAL.ICAL-PARSER-TIMEOUT-MS` - iCal parser timeout (default 500ms, range 1–60000ms); set via `CONFIG SET` / `redis.conf`
- Build-time env vars injected by `redical_redis/build.rs`: `GIT_SHA`, `GIT_TAG`, `MODULE_VERSION`, `BUILD_DATE_STRING`

**Build:**
- `Cargo.toml` (workspace root) - workspace-level dependency versions
- `Makefile` - build targets: `all`, `run`, `test`, `clean`, `distclean`, `deps`, `pack`
- `rustfmt.toml` - Rust formatting (edition = "2021")
- `Dockerfile` - multi-stage: `rust:bookworm` builder + `redis:7.0` base + `debian:bookworm` runtime

## Platform Requirements

**Development:**
- Rust stable toolchain
- Redis 7.0+ server for integration tests
- Python 3 + `ramp-packer` pip package for packaging only
- `clang` (required by Dockerfile apt install during build)

**Production:**
- Redis 7.0.0 minimum (declared in `ramp.yml`)
- Redis Enterprise 6.2.18 minimum pack version
- Deployed as `libredical.so` loaded via `redis-server --loadmodule`
- Docker image published to Docker Hub as `gregjoy/redical`
- Compiled targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`

---

*Stack analysis: 2026-03-06*
