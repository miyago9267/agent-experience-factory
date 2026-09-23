---
id: spec-context-harness-factory-ownership
title: Context Harness 核心封裝與本機資料邊界
status: complete
created: 2026-09-23
updated: 2026-09-23
approved_by:
tags: [packaging, portability, privacy]
priority: high
---

# Context Harness 核心封裝與本機資料邊界

## Requirements
- Factory repo owns the reusable Context Harness source and builds it from the checked-out Factory version.
- The local workspace remains the sole owner of routing, task records, Personal Model, project maps, and private experience data.
- Public source and tests contain no user-specific absolute paths, task IDs, or personal data fixtures.
- Installing/updating Factory must not copy, publish, or rewrite local workspace data; missing local configuration remains an explicit setup error.
- A clean checkout can run the documented tests and build both `agent-workflow` and Context Harness without reading any user's local workspace or dotfile.
- Existing local runtime adapters continue to invoke the installed Context Harness after source ownership changes.

## Architecture / Plan
### Decisions
- **Decision:** Move only the reusable Rust crate and its generic tests into `agent-experience-factory`.
  - **Reason:** The public Factory release must contain the implementation it installs; personal workspace data must remain local.
- **Decision:** Keep `routing.yaml`, `records/`, `personal-model/`, project knowledge, and experience stores outside the public Factory repo.
  - **Reason:** These are instance data and settings, not reusable tool code.
- **Decision:** Replace embedded user paths and task fixtures in the public core/tests with explicit configuration and temporary fixtures.
  - **Reason:** A clean installation must be portable and must not leak or depend on user-specific state.
- **Decision:** Preserve import compatibility for existing experience packs while new exports use the generic pack identifier.
  - **Reason:** Portability must not strand the user's already-exported data.

## Tasks
- [x] Phase 0: Confirm Factory GitHub target and local-only data boundary.
- [x] Phase 1: Move the reusable Rust crate and core tests into the Factory repository.
- [x] Phase 2: Make installer build from Factory source and keep workspace paths as runtime data inputs only.
- [x] Phase 3: Remove user-specific defaults and private fixtures from public tool source; retain safe import compatibility.
- [x] Phase 4: Make Factory tests self-contained and verify a clean-HOME install/build.
- [x] Phase 5: Update release metadata and docs, audit the public file set, and pass acceptance checks.

## Files
- `Cargo.toml`, `Cargo.lock`, `src/` - reusable Context Harness crate.
- `install.sh`, `INSTALL.md`, `scripts/agent-workflow` - install/build/runtime entrypoints.
- `factory.yaml`, `README.md`, `docs/SPEC.md` - ownership and release documentation.
- `scripts/test_factory*.sh`, Rust unit tests - portable regression coverage.
- `docs/specs/context-harness-factory-ownership/SPEC.md` - this migration tracker.

## Notes
The previous core commit `a8e8b2d` remains in the local workspace history. Its private task record and unrelated workspace WIP are not part of this migration.
Release publication is verified through the `v0.2.0` Git tag and remote ref; the source spec does not duplicate that mutable hosting state.
