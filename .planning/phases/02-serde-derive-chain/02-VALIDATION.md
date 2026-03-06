---
phase: 2
slug: serde-derive-chain
status: draft
nyquist_compliant: false
wave_0_complete: false
created: 2026-03-06
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust built-in test + pretty_assertions_sorted |
| **Config file** | Cargo.toml (per-crate test sections) |
| **Quick run command** | `cargo test -p redical_redis serde_smoke_test` |
| **Full suite command** | `cargo test --workspace` |
| **Estimated runtime** | ~30 seconds |

---

## Sampling Rate

- **After every task commit:** Run `cargo test --workspace`
- **After every plan wave:** Run `cargo test --workspace`
- **Before `/gsd:verify-work`:** Full suite must be green
- **Max feedback latency:** 30 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|-----------|-------------------|-------------|--------|
| 02-01-01 | 01 | 1 | SERD-01 | compilation | `cargo check -p redical_ical` | N/A | ⬜ pending |
| 02-01-02 | 01 | 1 | SERD-05 | compilation | `cargo check -p redical_ical` | N/A | ⬜ pending |
| 02-01-03 | 01 | 1 | SERD-02 | compilation | `cargo check -p redical_ical` | N/A | ⬜ pending |
| 02-01-04 | 01 | 1 | SERD-03 | compilation | `cargo check -p redical_core` | N/A | ⬜ pending |
| 02-01-05 | 01 | 1 | SERD-04 | compilation | `cargo check -p redical_core` | N/A | ⬜ pending |
| 02-01-06 | 01 | 1 | SMOKE | unit | `cargo test -p redical_redis serde_smoke_test` | ❌ W0 | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] Bincode round-trip smoke test in redical_redis (near rdb_data.rs tests)

*Existing infrastructure covers compilation checks — no new framework needed.*

---

## Manual-Only Verifications

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 30s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
