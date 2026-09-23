#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
install_dir="${AGENT_FACTORY_INSTALL_DIR:-${MIYAGO_AGENT_WORKFLOW_INSTALL_DIR:-$HOME/.local/bin}}"
config_dir="${AGENT_FACTORY_CONFIG_DIR:-${MIYAGO_AGENT_WORKFLOW_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/agent-experience}}"
runtime_config_dir="${AGENT_FACTORY_RUNTIME_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/agent-experience}"
data_root="${AGENT_FACTORY_DATA_ROOT:-${XDG_DATA_HOME:-$HOME/.local/share}/agent-experience}"
non_entry_root="${AGENT_FACTORY_NON_ENTRY_ROOT:-${XDG_CONFIG_HOME:-$HOME/.config}/agent-experience/non-entry}"
default_task_id="${AGENT_FACTORY_DEFAULT_TASK_ID:-}"
experience_task_id="${AGENT_FACTORY_EXPERIENCE_TASK_ID:-${MIYAGO_EXPERIENCE_TASK_ID:-}}"
jev_experience_retention="${AGENT_JEV_EXPERIENCE_RETENTION:-off}"
workspace_root="${AGENT_FACTORY_WORKSPACE_ROOT:-${MIYAGO_AGENT_WORKSPACE_ROOT:-}}"
dotfile_root="${AGENT_FACTORY_DOTFILE_ROOT:-${MIYAGO_DOTFILE_ROOT:-}}"
personal_vault_root="${AGENT_PERSONAL_VAULT_ROOT:-${MIYAGO_PERSONAL_VAULT_ROOT:-}}"
sre_vault_root="${AGENT_SRE_VAULT_ROOT:-${MIYAGO_SRE_VAULT_ROOT:-}}"
compat_binary_alias="${AGENT_CONTEXT_HARNESS_COMPAT_ALIAS:-}"
dry_run=0
ref='local'

usage() {
  cat <<'EOF'
usage: ./install.sh [--dry-run] [--ref REF]
  --workspace-root PATH  Context Harness workspace source
  --dotfile-root PATH    runtime configuration source
  --install-dir PATH     user-level binary destination
  --config-dir PATH      user-level Factory source-root configuration
  --runtime-config-dir PATH  stable runtime-neutral instruction directory
  --data-root PATH       private experience data directory
  --non-entry-root PATH  explicitly excluded path
  --default-task-id ID   local fallback task when cwd has no unique match
  --experience-task-id ID local task that stores experience observations
  --personal-vault-root PATH  optional local knowledge vault source
  --sre-vault-root PATH      optional local SRE knowledge source
  --compat-binary-alias PATH optional local alias for an older runtime adapter
  --jev-experience-retention MODE  send filtered experience summaries to Jev: on|off (default: off)
  --ref REF              visible release/ref label (default: local)
EOF
}

while (($#)); do
  case "$1" in
    --dry-run) dry_run=1; shift ;;
    --ref) ref="${2:?missing value for --ref}"; shift 2 ;;
    --workspace-root) workspace_root="${2:?missing value for --workspace-root}"; shift 2 ;;
    --dotfile-root) dotfile_root="${2:?missing value for --dotfile-root}"; shift 2 ;;
    --install-dir) install_dir="${2:?missing value for --install-dir}"; shift 2 ;;
    --config-dir) config_dir="${2:?missing value for --config-dir}"; shift 2 ;;
    --runtime-config-dir) runtime_config_dir="${2:?missing value for --runtime-config-dir}"; shift 2 ;;
    --data-root) data_root="${2:?missing value for --data-root}"; shift 2 ;;
    --non-entry-root) non_entry_root="${2:?missing value for --non-entry-root}"; shift 2 ;;
    --default-task-id) default_task_id="${2:?missing value for --default-task-id}"; shift 2 ;;
    --experience-task-id) experience_task_id="${2:?missing value for --experience-task-id}"; shift 2 ;;
    --personal-vault-root) personal_vault_root="${2:?missing value for --personal-vault-root}"; shift 2 ;;
    --sre-vault-root) sre_vault_root="${2:?missing value for --sre-vault-root}"; shift 2 ;;
    --compat-binary-alias) compat_binary_alias="${2:?missing value for --compat-binary-alias}"; shift 2 ;;
    --jev-experience-retention) jev_experience_retention="${2:?missing value for --jev-experience-retention}"; shift 2 ;;
    --help|-h) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ "$ref" != *[/$'\n']* ]] || { printf 'unsafe ref\n' >&2; exit 2; }
[[ "$jev_experience_retention" == on || "$jev_experience_retention" == off ]] || {
  printf 'Jev experience retention must be on or off\n' >&2
  exit 2
}
[[ "$install_dir" = /* ]] || { printf 'install dir must be absolute\n' >&2; exit 2; }
[[ "$config_dir" = /* ]] || { printf 'config dir must be absolute\n' >&2; exit 2; }
[[ "$workspace_root" = /* && "$dotfile_root" = /* && "$runtime_config_dir" = /* && "$data_root" = /* && "$non_entry_root" = /* ]] || {
  printf 'source roots must be absolute\n' >&2; exit 2;
}
for optional_root in "$personal_vault_root" "$sre_vault_root" "$compat_binary_alias"; do
  [[ -z "$optional_root" || "$optional_root" = /* ]] || {
    printf 'optional roots and aliases must be absolute\n' >&2; exit 2;
  }
done

context_binary="${AGENT_CONTEXT_HARNESS_BIN:-${MIYAGO_CONTEXT_HARNESS_BIN:-$install_dir/context-harness}}"
context_binary_explicit=0
[[ -n "${AGENT_CONTEXT_HARNESS_BIN:-${MIYAGO_CONTEXT_HARNESS_BIN:-}}" ]] && context_binary_explicit=1
required_sources=(
  "$workspace_root/routing.yaml"
  "$workspace_root/records/tasks/index.yaml"
  "$dotfile_root/config/ai/AGENTS.md"
  "$dotfile_root/config/ai/generated/claude/AGENTS.md"
  "$dotfile_root/config/ai/generated/codex/AGENTS.md"
  "$dotfile_root/config/ai/generated/gemini/GEMINI.md"
  "$dotfile_root/config/ai/generated/grok/AGENTS.md"
)

for path in "${required_sources[@]}"; do
  [[ -e "$path" ]] || { printf 'missing source: %s\n' "$path" >&2; exit 1; }
done

if [[ ! -x "$context_binary" ]]; then
  if ((context_binary_explicit)); then
    printf 'missing executable Context Harness: %s\n' "$context_binary" >&2
    exit 1
  fi
  if ((dry_run)); then
    printf 'context_harness: build_required (%s)\n' "$context_binary"
  else
    command -v cargo >/dev/null 2>&1 || {
      printf 'cargo is required to build Context Harness\n' >&2
      exit 1
    }
    cargo build --release --locked --manifest-path "$script_dir/Cargo.toml"
    mkdir -p "$(dirname -- "$context_binary")"
    if [[ -e "$context_binary" || -L "$context_binary" ]]; then
      printf 'refusing to replace non-executable Context Harness: %s\n' "$context_binary" >&2
      exit 1
    fi
    ln -s "$script_dir/target/release/agent-context-harness" "$context_binary"
    printf 'context_harness: installed (%s)\n' "$context_binary"
  fi
fi
target="$install_dir/agent-workflow"
config_file="$config_dir/factory.env"
printf 'factory_ref: %s\n' "$ref"
printf 'workspace_root: %s\ndotfile_root: %s\ntarget: %s\n' \
  "$workspace_root" "$dotfile_root" "$target"
printf 'jev_experience_retention: %s\n' "$jev_experience_retention"
printf 'config_file: %s\n' "$config_file"
if ((dry_run)); then
  [[ -z "$compat_binary_alias" ]] || printf 'context_harness_compat_alias: planned (%s)\n' "$compat_binary_alias"
  printf '%s\n' 'dry_run: no files changed'
  exit 0
fi

if [[ -n "$compat_binary_alias" && "$compat_binary_alias" != "$context_binary" ]]; then
  mkdir -p "$(dirname -- "$compat_binary_alias")"
  if [[ -e "$compat_binary_alias" && ! -L "$compat_binary_alias" ]]; then
    backup="$compat_binary_alias.bak.$(date +%Y%m%d_%H%M%S)"
    mv "$compat_binary_alias" "$backup"
    printf 'backup: %s\n' "$backup"
  elif [[ -L "$compat_binary_alias" ]]; then
    rm -f "$compat_binary_alias"
  fi
  ln -s "$context_binary" "$compat_binary_alias"
  printf 'context_harness_compat_alias: %s\n' "$compat_binary_alias"
fi

mkdir -p "$install_dir"
mkdir -p "$config_dir"
if [[ -e "$config_file" && ! -L "$config_file" ]]; then
  backup="$config_file.bak.$(date +%Y%m%d_%H%M%S)"
  mv "$config_file" "$backup"
  printf 'backup: %s\n' "$backup"
elif [[ -L "$config_file" ]]; then
  rm -f "$config_file"
fi
{
  printf 'AGENT_FACTORY_WORKSPACE_ROOT=%q\n' "$workspace_root"
  printf 'AGENT_FACTORY_DOTFILE_ROOT=%q\n' "$dotfile_root"
  printf 'AGENT_FACTORY_ROOT=%q\n' "$script_dir"
  printf 'AGENT_FACTORY_RUNTIME_CONFIG_DIR=%q\n' "$runtime_config_dir"
  printf 'AGENT_FACTORY_DATA_ROOT=%q\n' "$data_root"
  printf 'AGENT_FACTORY_NON_ENTRY_ROOT=%q\n' "$non_entry_root"
  printf 'AGENT_CONTEXT_HARNESS_BIN=%q\n' "$context_binary"
  printf 'AGENT_JEV_EXPERIENCE_RETENTION=%q\n' "$jev_experience_retention"
  [[ -n "$default_task_id" ]] && printf 'AGENT_FACTORY_DEFAULT_TASK_ID=%q\n' "$default_task_id"
  [[ -n "$experience_task_id" ]] && printf 'AGENT_FACTORY_EXPERIENCE_TASK_ID=%q\n' "$experience_task_id"
  [[ -n "$personal_vault_root" ]] && printf 'AGENT_PERSONAL_VAULT_ROOT=%q\n' "$personal_vault_root"
  [[ -n "$sre_vault_root" ]] && printf 'AGENT_SRE_VAULT_ROOT=%q\n' "$sre_vault_root"
} > "$config_file"
if [[ -e "$target" && ! -L "$target" ]]; then
  backup="$target.bak.$(date +%Y%m%d_%H%M%S)"
  mv "$target" "$backup"
  printf 'backup: %s\n' "$backup"
elif [[ -L "$target" ]]; then
  rm -f "$target"
fi
ln -s "$script_dir/scripts/agent-workflow" "$target"
printf '%s\n' 'installed: OK'
