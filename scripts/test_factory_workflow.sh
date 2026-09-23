#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary

dotfile_root="$test_root/dotfile"
config_file="$test_root/factory.env"
runtime_config_dir="$test_root/runtime-config"
data_root="$test_root/workflow-data"
mkdir -p "$dotfile_root" "$runtime_config_dir"
cat >"$config_file" <<EOF
AGENT_FACTORY_WORKSPACE_ROOT=$workspace_root
AGENT_FACTORY_DOTFILE_ROOT=$dotfile_root
AGENT_FACTORY_RUNTIME_CONFIG_DIR=$runtime_config_dir
AGENT_FACTORY_DATA_ROOT=$data_root
AGENT_FACTORY_NON_ENTRY_ROOT=$test_root/non-entry
AGENT_CONTEXT_HARNESS_BIN=$binary
AGENT_FACTORY_EXPERIENCE_TASK_ID=$experience_task_id
AGENT_JEV_EXPERIENCE_RETENTION=off
EOF

factory="$factory_root/scripts/agent-workflow"
AGENT_FACTORY_CONFIG_FILE="$config_file" "$factory" experience observe \
  --task "$experience_task_id" --kind explicit_preference --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'The facade should capture experience without requiring a candidate ID.' >/dev/null

output=$(AGENT_FACTORY_CONFIG_FILE="$config_file" "$factory" experience sync \
  --task "$experience_task_id" --experience-task "$experience_task_id" \
  --runtime codex --cwd "$workspace_root")
printf '%s\n' "$output" | grep -Fq 'auto_confirmed: 1'
printf '%s\n' "$output" | grep -Fq 'jev_status: disabled'
bundle=$(printf '%s\n' "$output" | sed -n 's/^experience_bundle_path: //p')
[[ -s "$bundle" ]]
grep -Fq 'capture experience without requiring a candidate ID' "$bundle"

enabled_config="$test_root/factory-jev-enabled.env"
cp "$config_file" "$enabled_config"
printf '%s\n' 'AGENT_JEV_EXPERIENCE_RETENTION=on' >>"$enabled_config"
AGENT_FACTORY_CONFIG_FILE="$enabled_config" "$factory" experience observe \
  --task "$experience_task_id" --kind correction --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'A correction candidate tests the unavailable-service fallback.' >/dev/null
jev_output=$(env -u JEV_EXPERIENCE_RETENTION TYPESAFE_API_KEY='' \
  AGENT_FACTORY_CONFIG_FILE="$enabled_config" "$factory" experience sync \
    --task "$experience_task_id" --experience-task "$experience_task_id" \
    --runtime codex --cwd "$workspace_root")
printf '%s\n' "$jev_output" | grep -Fq 'jev_status: enabled'
printf '%s\n' "$jev_output" | grep -Fq 'jev_skipped: 1'

usage_output=$("$binary" 2>&1 || true)
printf '%s\n' "$usage_output" | grep -Fq 'handoff'

fresh_workspace="$test_root/fresh-workspace"
fresh_project="$fresh_workspace/project"
mkdir -p "$fresh_project"
git -C "$fresh_project" -c init.defaultBranch=main init -q
mkdir -p "$fresh_workspace/records/tasks" "$fresh_workspace/records/contexts"
cp "$workspace_root/routing.yaml" "$fresh_workspace/routing.yaml"
printf 'version: 1\ntasks: []\n' >"$fresh_workspace/records/tasks/index.yaml"
fresh_config="$test_root/fresh-factory.env"
sed "s#AGENT_FACTORY_WORKSPACE_ROOT=.*#AGENT_FACTORY_WORKSPACE_ROOT=$fresh_workspace#" \
  "$config_file" | grep -v '^AGENT_FACTORY_EXPERIENCE_TASK_ID=' >"$fresh_config"
fresh_output=$(AGENT_FACTORY_CONFIG_FILE="$fresh_config" "$factory" \
  session-start --runtime codex --cwd "$fresh_project")
printf '%s\n' "$fresh_output" | grep -Fq 'selected_task: auto-'
[[ -f "$fresh_workspace/records/tasks/index.yaml" ]]

printf '%s\n' 'factory_workflow: OK'
