#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${AGENT_FACTORY_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
dotfile_root="${AGENT_FACTORY_DOTFILE_ROOT:-$HOME/dotfile}"
tmp_root="$(mktemp -d "${TMPDIR:-/tmp}/agent-experience-workflow.XXXXXX")"
trap 'rm -rf "$tmp_root"' EXIT

AGENT_FACTORY_WORKSPACE_ROOT="$workspace_root" AGENT_FACTORY_DOTFILE_ROOT="$dotfile_root" \
MIYAGO_OBSERVATION_ROOT="$tmp_root/data" AGENT_OBSERVATION_ROOT="$tmp_root/data" \
  "$factory_root/scripts/agent-workflow" experience observe \
  --task personal-experience-autocapture \
  --kind explicit_preference \
  --runtime codex \
  --scope "$workspace_root" \
  --scope "$dotfile_root" \
  --source user_message \
  --summary 'Factory facade should capture experience without exposing the candidate id.' >/dev/null

output="$(cd "$dotfile_root" && AGENT_FACTORY_WORKSPACE_ROOT="$workspace_root" \
  AGENT_FACTORY_DOTFILE_ROOT="$dotfile_root" MIYAGO_OBSERVATION_ROOT="$tmp_root/data" \
  AGENT_OBSERVATION_ROOT="$tmp_root/data" "$factory_root/scripts/agent-workflow" experience sync \
  --task personal-experience-autocapture \
  --experience-task personal-experience-autocapture \
  --runtime codex \
  --cwd "$workspace_root")"

printf '%s\n' "$output" | grep -Fq 'auto_confirmed: 1'
bundle="$(printf '%s\n' "$output" | sed -n 's/^experience_bundle_path: //p')"
[[ -s "$bundle" ]]
grep -Fq 'Factory facade should capture experience without exposing the candidate id.' "$bundle"

AGENT_CONTEXT_HARNESS_BIN="${AGENT_CONTEXT_HARNESS_BIN:-$HOME/.local/bin/miyago-context-harness}"
usage_output="$($AGENT_CONTEXT_HARNESS_BIN 2>&1 || true)"
printf '%s\n' "$usage_output" | grep -Fq 'handoff'

fresh_workspace="$tmp_root/fresh-workspace"
fresh_project="$tmp_root/fresh-project"
mkdir -p "$fresh_workspace"
cp -R "$workspace_root/routing.yaml" "$workspace_root/records" "$fresh_workspace/"
mkdir -p "$fresh_project"
git -C "$fresh_project" init -q
fresh_output="$(AGENT_FACTORY_WORKSPACE_ROOT="$fresh_workspace" \
  AGENT_FACTORY_DOTFILE_ROOT="$dotfile_root" "$factory_root/scripts/agent-workflow" \
  session-start --runtime codex --cwd "$fresh_project")"
printf '%s\n' "$fresh_output" | grep -Fq 'selected_task:'

printf '%s\n' 'factory_workflow: OK'
