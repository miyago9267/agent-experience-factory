#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"
export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"
test_root="$(mktemp -d "${TMPDIR:-/tmp}/agent-factory-test.XXXXXX")"
test_root="$(CDPATH='' cd -- "$test_root" && pwd -P)"
workspace_root="$test_root/workspace"
binary="$factory_root/target/debug/agent-context-harness"
experience_task_id=experience-test
export AGENT_EXPERIENCE_DATA_ROOT="$test_root/experience-data"
trap 'rm -rf "$test_root"' EXIT

mkdir -p "$workspace_root/records/tasks" "$workspace_root/records/contexts" "$workspace_root/secondary"
git -C "$workspace_root" -c init.defaultBranch=main init -q
cat >"$workspace_root/routing.yaml" <<'YAML'
version: 1
excluded: []
knowledge_sources: []
YAML
printf 'version: 1\ntasks:\n' >"$workspace_root/records/tasks/index.yaml"

register_test_task() {
  local task_id="$1"
  local profile="${2:-implementation}"
  local project="${3:-$workspace_root}"
  cat >"$workspace_root/records/tasks/$task_id.yaml" <<YAML
task_id: $task_id
profile: $profile
goal: Exercise the generic experience workflow.
scope:
  projects:
    - $project
  services: []
  repositories:
    - $project
  environments: [local]
excluded_scope: []
stop_condition: The focused integration check passes.
context_policy:
  layers: [task_core, current_state, direct_evidence, background_experience]
  max_items: 20
  max_chars: 10000
YAML
  cat >>"$workspace_root/records/tasks/index.yaml" <<YAML
  - task_id: $task_id
    status: active
    task_path: records/tasks/$task_id.yaml
    pack_path: records/contexts/$task_id-context-pack.md
    projects:
      - $project
    preferred_runtimes: [codex, claude]
YAML
}

register_test_task "$experience_task_id"

build_test_binary() {
  cargo build --quiet --locked --manifest-path "$factory_root/Cargo.toml"
}

test_mode() {
  if stat -c '%a' "$1" >/dev/null 2>&1; then
    stat -c '%a' "$1"
  else
    stat -f '%Lp' "$1"
  fi
}

test_sha256() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  else
    shasum -a 256 "$1" | awk '{print $1}'
  fi
}
