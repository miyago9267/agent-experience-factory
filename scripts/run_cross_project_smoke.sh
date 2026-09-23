#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
output_root="$factory_root/results/cross-project"
context_task="${AGENT_FACTORY_DEFAULT_TASK_ID:-}"
context_cwd="$PWD"
runtime_list=(claude codex gemini grok)
scope_paths=()
exclude_paths=()
prompt_file="$factory_root/fixtures/runtime-parity/cross-project-inventory.txt"

usage() {
  printf '%s\n' 'usage: run_cross_project_smoke.sh --context-task TASK_ID --scope PATH --scope PATH [--exclude PATH ...] [--prompt-file PATH] [--output-dir PATH] [--context-cwd PATH]'
}

while (($#)); do
  case "$1" in
    --prompt-file) prompt_file="${2:?missing value for --prompt-file}"; shift 2 ;;
    --output-dir) output_root="${2:?missing value for --output-dir}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
    --scope) scope_paths+=("${2:?missing value for --scope}"); shift 2 ;;
    --exclude) exclude_paths+=("${2:?missing value for --exclude}"); shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$context_task" ]] || { printf '%s\n' '--context-task is required' >&2; exit 2; }
(("${#scope_paths[@]}" >= 2)) || { printf '%s\n' 'at least two --scope PATH values are required' >&2; exit 2; }
[[ -f "$prompt_file" ]] || { printf 'prompt file does not exist: %s\n' "$prompt_file" >&2; exit 2; }
[[ -d "$context_cwd" ]] || { printf 'context cwd is not a directory: %s\n' "$context_cwd" >&2; exit 2; }
for scope_path in "${scope_paths[@]}"; do
  [[ -d "$scope_path" ]] || { printf 'scope is not a directory: %s\n' "$scope_path" >&2; exit 2; }
done

yaml_quote() { jq -Rn --arg value "$1" '$value'; }

summary="$output_root/summary.yaml"
summary_tmp="$summary.tmp.$$"
trap 'rm -f "$summary_tmp"' EXIT
mkdir -p "$output_root"
{
  printf '%s\n' 'version: 1' 'kind: cross_project_runtime_summary'
  printf 'context_task: %s\n' "$(yaml_quote "$context_task")"
  printf '%s\n' 'scope:'
  for scope_path in "${scope_paths[@]}"; do
    printf '  - %s\n' "$(yaml_quote "$scope_path")"
  done
  if (("${#exclude_paths[@]}" == 0)); then
    printf '%s\n' 'excluded_scope: []'
  else
    printf '%s\n' 'excluded_scope:'
    for exclude_path in "${exclude_paths[@]}"; do
      printf '  - %s\n' "$(yaml_quote "$exclude_path")"
    done
  fi
  printf '%s\n' 'results:'
} > "$summary_tmp"

scope_args=()
for scope_path in "${scope_paths[@]}"; do
  scope_args+=(--scope "$scope_path")
done
for runtime in "${runtime_list[@]}"; do
  result="$output_root/$runtime.yaml"
  log="$result.log"
  set +e
  AGENT_PLUGIN_ID="$runtime" "$factory_root/plugins/runtime-adapter.sh" run \
    --prompt-file "$prompt_file" \
    --cwd "$context_cwd" \
    --context-task "$context_task" \
    --context-cwd "$context_cwd" \
    "${scope_args[@]}" \
    --output "$result" >"$log" 2>&1
  rc=$?
  set -e
  status="$(awk '$1 == "status:" { print $2; exit }' "$result" 2>/dev/null || true)"
  [[ -n "$status" ]] || status=error
  {
    printf '  - runtime: %s\n' "$runtime"
    printf '    status: %s\n' "$status"
    printf '    exit_status: %s\n' "$rc"
    printf '    result_path: %s\n' "$result"
    printf '    final_output_path: %s\n' "$result.final"
  } >> "$summary_tmp"
  printf '%s exit=%s status=%s\n' "$runtime" "$rc" "$status"
done

mv "$summary_tmp" "$summary"
printf 'summary_path: %s\n' "$summary"
