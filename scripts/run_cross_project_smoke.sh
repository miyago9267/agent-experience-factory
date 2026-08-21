#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
output_root="${factory_root}/results/cross-project"
context_task=agent-benchmark-waza
context_cwd="${factory_root}"
runtime_list=(claude codex gemini grok)

usage() {
  printf '%s\n' 'usage: run_cross_project_smoke.sh [--output-dir PATH] [--context-task TASK_ID] [--context-cwd PATH]'
}

while (($#)); do
  case "$1" in
    --output-dir) output_root="${2:?missing value for --output-dir}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

prompt_file="${factory_root}/fixtures/runtime-parity/cross-project-inventory.txt"
summary="${output_root}/summary.yaml"
mkdir -p "$output_root"
{
  printf '%s\n' 'version: 1' 'kind: cross_project_runtime_summary' "context_task: $context_task" 'results:'
} > "$summary"

for runtime in "${runtime_list[@]}"; do
  result="${output_root}/${runtime}.yaml"
  log="${result}.log"
  set +e
  MIYAGO_PLUGIN_ID="$runtime" "${factory_root}/plugins/runtime-adapter.sh" run \
    --prompt-file "$prompt_file" \
    --cwd "$context_cwd" \
    --context-task "$context_task" \
    --context-cwd "$context_cwd" \
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
  } >> "$summary"
  printf '%s exit=%s status=%s\n' "$runtime" "$rc" "$status"
done

printf 'summary_path: %s\n' "$summary"
