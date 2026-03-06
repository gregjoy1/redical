---
phase: 3
slug: rdb-format
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test (Rust built-in) |
| **Config file** | Cargo.toml (workspace) |
| **Quick run command** | `cargo test -p redical_redis -- --test-threads=1` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test -p redical_redis -- --test-threads=1`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 03-01-01 | 01 | 1 | RDB-01 | unit | `cargo test -p redical_redis rdb_data -- --test-threads=1` | Wave 0 | ⬜ pending |
| 03-01-02 | 01 | 1 | RDB-02 | unit | `cargo test -p redical_redis rdb_save -- --test-threads=1` | Wave 0 | ⬜ pending |
| 03-01-03 | 01 | 1 | RDB-03 | unit | `cargo test -p redical_redis rdb_load -- --test-threads=1` | Wave 0 | ⬜ pending |
| 03-01-04 | 01 | 1 | RDB-04 | unit | `cargo test -p redical_redis catch_unwind -- --test-threads=1` | Wave 0 | ⬜ pending |
| 03-01-05 | 01 | 1 | RDB-05 | unit | `cargo test -p redical_redis rebuild -- --test-threads=1` | Wave 0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] RDBCalendarDump round-trip test (serialize envelope, deserialize, verify fields)
- [ ] Version mismatch falls back to iCal path (unit test)
- [ ] Existing tests in rdb_data.rs pass with new struct added

*Note: Phase 4 handles integration test fixtures (TEST-01 through TEST-06). Phase 3 tests focus on unit-level correctness of new code paths.*

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| catch_unwind prevents Redis crash | RDB-04 | Panic in FFI is UB in real Redis; unit test uses safe wrapper | Verify no crash in redis-server with intentionally corrupted bincode data |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
