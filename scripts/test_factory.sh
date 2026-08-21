#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
workspace_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
dotfile_root="${MIYAGO_DOTFILE_ROOT:-$HOME/dotfile}"
install_dir="$(mktemp -d "${TMPDIR:-/tmp}/miyago-factory-bin.XXXXXX")"
install_dir="$(CDPATH='' cd -- "$install_dir" && pwd -P)"

bash -n "$factory_root/install.sh" "$factory_root/scripts/agent-workflow"
"$factory_root/install.sh" --dry-run --workspace-root "$workspace_root" \
  --dotfile-root "$dotfile_root" --install-dir "$install_dir" --ref test
"$factory_root/install.sh" --workspace-root "$workspace_root" \
  --dotfile-root "$dotfile_root" --install-dir "$install_dir" >/dev/null

factory_bin="$install_dir/agent-workflow"
doctor_output=$(MIYAGO_AGENT_WORKSPACE_ROOT="$workspace_root" \
  MIYAGO_DOTFILE_ROOT="$dotfile_root" \
  MIYAGO_CONTEXT_HARNESS_BIN="${MIYAGO_CONTEXT_HARNESS_BIN:-$HOME/.local/bin/miyago-context-harness}" \
  "$factory_bin" doctor)
printf '%s\n' "$doctor_output" | grep -Fq 'status: OK'
version_output=$("$factory_bin" version)
printf '%s\n' "$version_output" | grep -Fq 'agent-workflow-factory 0.1.0'

printf '%s\n' 'factory: OK'
