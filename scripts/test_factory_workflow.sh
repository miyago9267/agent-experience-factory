#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/miyago-factory-workflow.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

MIYAGO_OBSERVATION_ROOT="$tmp_root/data" "$factory_root/scripts/agent-workflow" experience observe \
  --task personal-experience-autocapture \
  --kind explicit_preference \
  --runtime codex \
  --scope "$workspace_root" \
  --scope "$HOME/dotfile" \
  --source user_message \
  --summary 'Factory facade should capture experience without exposing the candidate id.' >/dev/null

output="$(MIYAGO_OBSERVATION_ROOT="$tmp_root/data" "$factory_root/scripts/agent-workflow" experience sync \
  --task personal-experience-autocapture \
  --experience-task personal-experience-autocapture \
  --runtime codex \
  --cwd "$workspace_root")"

printf '%s\n' "$output" | grep -Fq 'auto_confirmed: 1'
bundle="$(printf '%s\n' "$output" | sed -n 's/^experience_bundle_path: //p')"
[[ -s "$bundle" ]]
grep -Fq 'Factory facade should capture experience without exposing the candidate id.' "$bundle"

MIYAGO_CONTEXT_HARNESS_BIN="${MIYAGO_CONTEXT_HARNESS_BIN:-$HOME/.local/bin/miyago-context-harness}"
usage_output="$($MIYAGO_CONTEXT_HARNESS_BIN 2>&1 || true)"
printf '%s\n' "$usage_output" | grep -Fq 'handoff'
printf '%s\n' 'factory_workflow: OK'
