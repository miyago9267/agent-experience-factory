#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"

dotfile_root="$test_root/dotfile"
runtime_config_dir="$test_root/runtime-config"
install_dir="$test_root/home/.local/bin"
config_dir="$test_root/home/.config/agent-experience"
mkdir -p "$dotfile_root/config/ai/generated" "$dotfile_root/config/opencode" \
  "$dotfile_root/script/common" "$runtime_config_dir/personal-model" \
  "$install_dir" "$test_root/home/.config"
printf '%s\n' 'Local test entry.' >"$dotfile_root/config/ai/AGENT-ENTRY.md"
printf '%s\n' 'Generic test rules.' >"$dotfile_root/config/ai/AGENTS.md"
printf '%s\n' '{}' >"$dotfile_root/config/opencode/opencode.json"
printf '%s\n' 'Generic local profile.' >"$runtime_config_dir/personal-model/PROFILE.md"
printf '%s\n' 'Generic shared rules.' >"$runtime_config_dir/AGENTS.md"
for runtime_file in \
  "$dotfile_root/config/ai/generated/claude/AGENTS.md" \
  "$dotfile_root/config/ai/generated/codex/AGENTS.md" \
  "$dotfile_root/config/ai/generated/gemini/GEMINI.md" \
  "$dotfile_root/config/ai/generated/grok/AGENTS.md"; do
  mkdir -p "$(dirname -- "$runtime_file")"
  printf '%s\n' 'Generated test entry.' >"$runtime_file"
done

bash -n "$factory_root/install.sh" "$factory_root/scripts/agent-workflow"
dry_run_output=$(HOME="$test_root/home" XDG_CONFIG_HOME="$test_root/home/.config" \
  XDG_DATA_HOME="$test_root/home/.local/share" \
  "$factory_root/install.sh" --dry-run --workspace-root "$workspace_root" \
    --dotfile-root "$dotfile_root" --install-dir "$install_dir" --config-dir "$config_dir" \
    --runtime-config-dir "$runtime_config_dir" --non-entry-root "$test_root/non-entry" \
    --data-root "$test_root/home/.local/share/agent-experience" \
    --compat-binary-alias "$install_dir/dry-run-legacy" --ref test)
printf '%s\n' "$dry_run_output" | grep -Fq 'context_harness: build_required'
printf '%s\n' "$dry_run_output" | grep -Fq "context_harness_compat_alias: planned ($install_dir/dry-run-legacy)"
[[ ! -e "$install_dir/dry-run-legacy" && ! -L "$install_dir/dry-run-legacy" ]]

HOME="$test_root/home" XDG_CONFIG_HOME="$test_root/home/.config" \
XDG_DATA_HOME="$test_root/home/.local/share" \
  "$factory_root/install.sh" --workspace-root "$workspace_root" \
    --dotfile-root "$dotfile_root" --install-dir "$install_dir" --config-dir "$config_dir" \
    --runtime-config-dir "$runtime_config_dir" --non-entry-root "$test_root/non-entry" \
    --data-root "$test_root/home/.local/share/agent-experience" \
    --experience-task-id "$experience_task_id" \
    --compat-binary-alias "$install_dir/legacy-context-harness" \
    --jev-experience-retention on >/dev/null
[[ -x "$install_dir/context-harness" ]]
[[ -L "$install_dir/legacy-context-harness" ]]
[[ -f "$config_dir/factory.env" ]]
grep -Fq 'AGENT_JEV_EXPERIENCE_RETENTION=on' "$config_dir/factory.env"
grep -Fq "AGENT_FACTORY_EXPERIENCE_TASK_ID=$experience_task_id" "$config_dir/factory.env"

factory_bin="$install_dir/agent-workflow"
doctor_output=$(HOME="$test_root/home" XDG_CONFIG_HOME="$test_root/home/.config" \
  XDG_DATA_HOME="$test_root/home/.local/share" "$factory_bin" doctor)
printf '%s\n' "$doctor_output" | grep -Fq 'status: OK'
printf '%s\n' "$doctor_output" | grep -Fq 'jev_experience_retention: enabled'
version_output=$("$factory_bin" version)
printf '%s\n' "$version_output" | grep -Fq 'agent-experience-factory 0.2.0'
runtime_list=$("$factory_bin" runtime list)
printf '%s\n' "$runtime_list" | grep -Fxq codex
printf '%s\n' "$runtime_list" | grep -Fxq grok
plan_output=$(HOME="$test_root/home" XDG_CONFIG_HOME="$test_root/home/.config" \
  XDG_DATA_HOME="$test_root/home/.local/share" "$factory_bin" plan --cwd "$workspace_root")
printf '%s\n' "$plan_output" | grep -Fq 'model: coding-reasoning'
printf '%s\n' "$plan_output" | grep -Fq 'delegation: bounded-parallel'
status_output=$(HOME="$test_root/home" XDG_CONFIG_HOME="$test_root/home/.config" \
  XDG_DATA_HOME="$test_root/home/.local/share" "$factory_bin" status --cwd "$workspace_root")
printf '%s\n' "$status_output" | grep -Fq "task_id: $experience_task_id"

printf '%s\n' 'factory: OK'
