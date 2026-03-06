---
phase: 1
slug: safety-fixes
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in (`cargo test`) |
| **Config file** | none (uses Cargo defaults) |
| **Quick run command** | `cargo build --package redical_redis 2>&1 \| grep -E "^error"` |
| **Full suite command** | `cargo test` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo build --package redical_redis 2>&1 | grep -E "^error"` (must be empty)
- **After every plan wave:** Run `cargo test`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** ~30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 1-01-01 | 01 | 1 | SAFE-01 | smoke | `cargo build && ! grep -r 'todo!' redical_redis/src/datatype/mod.rs` | N/A — compile-time | ⬜ pending |
| 1-01-02 | 01 | 1 | SAFE-02 | smoke | `cargo build && grep -c 'from_utf8_unchecked' redical_redis/src/datatype/mod.rs \| grep -q '^0$'` | N/A — compile-time | ⬜ pending |
| 1-01-03 | 01 | 1 | UPGR-01 | smoke | `grep 'redis-module' redical_redis/Cargo.toml \| grep '2.0.4'` | redical_redis/Cargo.toml | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Existing infrastructure covers all phase requirements. No new test files needed — all three requirements are verified by `cargo build` succeeding plus grep checks.

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
