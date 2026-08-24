#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${AGENT_FACTORY_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
dotfile_root="${AGENT_FACTORY_DOTFILE_ROOT:-$HOME/dotfile}"
install_dir="$(mktemp -d "${TMPDIR:-/tmp}/agent-experience-bin.XXXXXX")"
install_dir="$(CDPATH='' cd -- "$install_dir" && pwd -P)"
tmp_home="$(mktemp -d "${TMPDIR:-/tmp}/agent-experience-home.XXXXXX")"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
trap 'rm -rf "$install_dir" "$tmp_home"' EXIT

bash -n "$factory_root/install.sh" "$factory_root/scripts/agent-workflow"
dry_run_output=$(HOME="$tmp_home" RUSTUP_HOME="$rustup_home" CARGO_HOME="$cargo_home" \
  "$factory_root/install.sh" --dry-run --workspace-root "$workspace_root" \
  --dotfile-root "$dotfile_root" --install-dir "$install_dir" --ref test
)
printf '%s\n' "$dry_run_output" | grep -Fq 'context_harness: build_required'
HOME="$tmp_home" RUSTUP_HOME="$rustup_home" CARGO_HOME="$cargo_home" \
  "$factory_root/install.sh" --workspace-root "$workspace_root" \
  --dotfile-root "$dotfile_root" --runtime-config-dir "$tmp_home/.config/miyago-agent" \
  --default-task-id agent-benchmark-waza \
  --install-dir "$install_dir" >/dev/null
[[ -x "$tmp_home/.local/bin/miyago-context-harness" ]]
[[ -f "$tmp_home/.config/agent-experience/factory.env" ]]
HOME="$tmp_home" XDG_CONFIG_HOME="$tmp_home/.config" \
  MIYAGO_DOTFILE_ROOT="$dotfile_root" MIYAGO_AGENT_WORKSPACE_ROOT="$workspace_root" \
  bash "$dotfile_root/script/common/setup_dotfiles.sh" >/dev/null
[[ -L "$tmp_home/.config/agent-experience/AGENTS.md" ]]
[[ -L "$tmp_home/.config/agent-experience/personal-model/PROFILE.md" ]]

factory_bin="$install_dir/agent-workflow"
doctor_output=$(HOME="$tmp_home" "$factory_bin" doctor)
printf '%s\n' "$doctor_output" | grep -Fq 'status: OK'
version_output=$("$factory_bin" version)
printf '%s\n' "$version_output" | grep -Fq 'agent-experience-factory 0.1.0'
runtime_list=$("$factory_bin" runtime list)
printf '%s\n' "$runtime_list" | grep -Fxq codex
printf '%s\n' "$runtime_list" | grep -Fxq grok
plan_output=$(HOME="$tmp_home" "$factory_bin" plan --cwd "$workspace_root")
printf '%s\n' "$plan_output" | grep -Fq 'model: coding-reasoning'
printf '%s\n' "$plan_output" | grep -Fq 'delegation: bounded-parallel'
status_output=$(HOME="$tmp_home" "$factory_bin" status --cwd "$tmp_home")
printf '%s\n' "$status_output" | grep -Fq 'task_id: agent-benchmark-waza'

printf '%s\n' 'factory: OK'
