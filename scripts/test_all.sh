#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$factory_root"

printf '%s\n' '==> Rust formatting and unit tests'
cargo fmt --all -- --check
cargo test --locked

printf '%s\n' '==> Shell syntax'
bash -n install.sh scripts/*.sh plugins/*.sh plugins/waza/*.sh scripts/agent-workflow

printf '%s\n' '==> Integration tests'
for test_script in \
  scripts/test_experience_sync.sh \
  scripts/test_experience_organize.sh \
  scripts/test_experience_review.sh \
  scripts/test_experience_consume_feedback.sh \
  scripts/test_experience_portable_pack.sh \
  scripts/test_factory.sh \
  scripts/test_factory_workflow.sh \
  scripts/test_plugin_contract.sh; do
  bash "$test_script"
done

printf '%s\n' 'test_all: OK'
