#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
dotfile_root="${MIYAGO_DOTFILE_ROOT:-$HOME/dotfile}"
install_dir="$(mktemp -d "${TMPDIR:-/tmp}/miyago-factory-bin.XXXXXX")"
install_dir="$(CDPATH='' cd -- "$install_dir" && pwd -P)"
tmp_home="$(mktemp -d "${TMPDIR:-/tmp}/miyago-factory-home.XXXXXX")"
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
  --dotfile-root "$dotfile_root" --install-dir "$install_dir" >/dev/null
[[ -x "$tmp_home/.local/bin/miyago-context-harness" ]]

factory_bin="$install_dir/agent-workflow"
doctor_output=$(MIYAGO_AGENT_WORKSPACE_ROOT="$workspace_root" \
  MIYAGO_DOTFILE_ROOT="$dotfile_root" \
  HOME="$tmp_home" \
  "$factory_bin" doctor)
printf '%s\n' "$doctor_output" | grep -Fq 'status: OK'
version_output=$("$factory_bin" version)
printf '%s\n' "$version_output" | grep -Fq 'agent-workflow-factory 0.1.0'
plan_output=$(MIYAGO_AGENT_WORKSPACE_ROOT="$workspace_root" \
  MIYAGO_DOTFILE_ROOT="$dotfile_root" \
  HOME="$tmp_home" \
  "$factory_bin" plan --cwd "$workspace_root")
printf '%s\n' "$plan_output" | grep -Fq 'model: coding-reasoning'
printf '%s\n' "$plan_output" | grep -Fq 'delegation: bounded-parallel'

printf '%s\n' 'factory: OK'
