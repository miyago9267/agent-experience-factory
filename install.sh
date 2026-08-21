#!/usr/bin/env bash
set -Eeuo pipefail

script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
install_dir="${MIYAGO_AGENT_WORKFLOW_INSTALL_DIR:-$HOME/.local/bin}"
config_dir="${MIYAGO_AGENT_WORKFLOW_CONFIG_DIR:-${XDG_CONFIG_HOME:-$HOME/.config}/agent-experience}"
workspace_root="${MIYAGO_AGENT_WORKSPACE_ROOT:-$HOME/Project/AI/agent-workspace}"
dotfile_root="${MIYAGO_DOTFILE_ROOT:-$HOME/dotfile}"
dry_run=0
ref='local'

usage() {
  cat <<'EOF'
usage: ./install.sh [--dry-run] [--ref REF]
  --workspace-root PATH  agent-workspace source
  --dotfile-root PATH    dotfile source
  --install-dir PATH     user-level binary destination
  --config-dir PATH      user-level factory source-root configuration
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
    --help|-h) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ "$ref" != *[/$'\n']* ]] || { printf 'unsafe ref\n' >&2; exit 2; }
[[ "$install_dir" = /* ]] || { printf 'install dir must be absolute\n' >&2; exit 2; }
[[ "$config_dir" = /* ]] || { printf 'config dir must be absolute\n' >&2; exit 2; }
[[ "$workspace_root" = /* && "$dotfile_root" = /* ]] || {
  printf 'source roots must be absolute\n' >&2; exit 2;
}

context_binary="${MIYAGO_CONTEXT_HARNESS_BIN:-$HOME/.local/bin/miyago-context-harness}"
context_binary_explicit=0
[[ -n "${MIYAGO_CONTEXT_HARNESS_BIN:-}" ]] && context_binary_explicit=1
required_sources=(
  "$workspace_root/routing.yaml"
  "$workspace_root/src/main.rs"
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
    cargo build --release --manifest-path "$workspace_root/Cargo.toml"
    mkdir -p "$(dirname -- "$context_binary")"
    if [[ -e "$context_binary" || -L "$context_binary" ]]; then
      printf 'refusing to replace non-executable Context Harness: %s\n' "$context_binary" >&2
      exit 1
    fi
    ln -s "$workspace_root/target/release/miyago-context-harness" "$context_binary"
    printf 'context_harness: installed (%s)\n' "$context_binary"
  fi
fi

target="$install_dir/agent-workflow"
config_file="$config_dir/factory.env"
printf 'factory_ref: %s\n' "$ref"
printf 'workspace_root: %s\ndotfile_root: %s\ntarget: %s\n' \
  "$workspace_root" "$dotfile_root" "$target"
printf 'config_file: %s\n' "$config_file"
if ((dry_run)); then
  printf '%s\n' 'dry_run: no files changed'
  exit 0
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
  printf 'MIYAGO_FACTORY_WORKSPACE_ROOT=%q\n' "$workspace_root"
  printf 'MIYAGO_FACTORY_DOTFILE_ROOT=%q\n' "$dotfile_root"
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
