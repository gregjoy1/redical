# External Integrations

**Analysis Date:** 2026-03-06

## APIs & External Services

**Redis Module API:**
- Redis 7.0+ - the process host; RediCal runs inside the Redis server as a native module
  - SDK/Client: `redis-module` 2.0.2 (`redical_redis/Cargo.toml`)
  - Interface: `redis_module!` macro registers commands and data types with the Redis engine
  - Commands exposed: `RDCL.CAL_SET`, `RDCL.CAL_GET`, `RDCL.EVT_SET`, `RDCL.EVT_GET`, `RDCL.EVT_DEL`, `RDCL.EVT_LIST`, `RDCL.EVT_QUERY`, `RDCL.EVT_PRUNE`, `RDCL.EVO_SET`, `RDCL.EVO_GET`, `RDCL.EVO_DEL`, `RDCL.EVO_LIST`, `RDCL.EVO_PRUNE`, `RDCL.EVI_LIST`, `RDCL.EVI_QUERY`, `RDCL.CAL_IDX_DISABLE`, `RDCL.CAL_IDX_REBUILD`
  - Source: `redical_redis/src/commands/`

**iCalendar Standard:**
- RFC 5545 / iCalendar - the data format RediCal parses and serialises
  - Parser: custom `nom`-based combinator grammar in `redical_ical/src/grammar.rs` and `redical_ical/src/properties/`
  - Recurrence rules: delegated to `rrule` 0.10 crate

## Data Storage

**Databases:**
- Redis (embedded) - RediCal IS the storage; it extends Redis with a native `Calendar` data type
  - Connection: N/A (runs inside Redis process)
  - Client: `redis-module` SDK (`RedisGILGuard`, `Context`)
  - Persistence: RDB via `bincode` serialisation in `redical_redis/src/datatype/rdb_data.rs`; AOF supported per `ramp.yml` capabilities

**File Storage:**
- None (no external file storage)

**Caching:**
- Internal R*-tree and inverted index structures maintained in-memory within the `Calendar` data type (`redical_core/src/geo_index.rs`, `redical_core/src/inverted_index.rs`)

## Authentication & Identity

**Auth Provider:**
- None - authentication is entirely delegated to Redis's own ACL/auth system; the module adds no auth layer

## Monitoring & Observability

**Error Tracking:**
- None

**Logs:**
- Redis module logging API (`ctx.log_notice`, `ctx.log_warning`) used for startup banner and error reporting; logs flow through the Redis server log

## CI/CD & Deployment

**Hosting:**
- Docker Hub (`gregjoy/redical`) - container image published on push to `main` and on version tags
  - Workflow: `.github/workflows/` (docker-main and docker-tag workflows)
  - Secrets required: `DOCKERHUB_USERNAME`, `DOCKERHUB_PASSWORD`

- GitHub Releases - compiled `.tar.gz` archives uploaded on version tag push
  - Targets: `x86_64-unknown-linux-gnu`, `x86_64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin`
  - Workflow: `.github/workflows/` (build-release workflow)

**CI Pipeline:**
- GitHub Actions (`ubuntu-latest`)
  - `check` job: `cargo check`
  - `clippy` job: `cargo clippy -- -D warnings`
  - `unit_test` job: `cargo test`
  - `integration_test` job: installs Redis via `ppa:redislabs/redis`, builds module, runs `cargo test --all integration`
  - Trigger: every push and pull request

## Environment Configuration

**Required env vars:**
- None at runtime (module uses Redis `CONFIG SET`/`CONFIG GET`)
- CI secrets: `DOCKERHUB_USERNAME`, `DOCKERHUB_PASSWORD` (GitHub Actions secrets)

**Secrets location:**
- GitHub Actions encrypted secrets only; no `.env` files present in repo

## Webhooks & Callbacks

**Incoming:**
- Redis keyspace notifications emitted by RediCal commands; configured in test via `notify-keyspace-events Kegd` (`tests/redis_test_config.conf`)
- Clients subscribe to these through standard Redis pub/sub; RediCal does not consume them internally

**Outgoing:**
- None

## Fuzzing

**AFL (American Fuzzy Lop):**
- `redical_ical_afl_fuzz_targets/` - standalone crate with two fuzz targets (`event_properties_afl_fuzz_target`, `query_properties_afl_fuzz_target`)
- Uses `afl` crate; run via `redical_ical_afl_fuzz_targets/start_afl_fuzz.sh`
- Input seeds: `redical_ical_afl_fuzz_targets/input_seeds/`
- Not part of default workspace members or CI; opt-in fuzzing only

---

*Integration audit: 2026-03-06*
