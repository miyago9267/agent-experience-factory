#!/usr/bin/env bash
set -Eeuo pipefail

factory_root="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd -P)"
runtime_list=(claude codex gemini grok)
scenario_list=(architecture operations conversation)
trials=2
output_root="$factory_root/results/runtime-parity"
context_task="${AGENT_FACTORY_DEFAULT_TASK_ID:-}"
context_cwd="$PWD"
scope_paths=()

usage() {
  printf '%s\n' 'usage: run_runtime_parity.sh --context-task TASK_ID --scope PATH [--scope PATH ...] [--runtime RUNTIME] [--trials N] [--output-dir PATH] [--context-cwd PATH]'
}

while (($#)); do
  case "$1" in
    --runtime) runtime_list=("${2:?missing value for --runtime}"); shift 2 ;;
    --trials) trials="${2:?missing value for --trials}"; shift 2 ;;
    --output-dir) output_root="${2:?missing value for --output-dir}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
    --scope) scope_paths+=("${2:?missing value for --scope}"); shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

[[ -n "$context_task" ]] || { printf '%s\n' '--context-task is required; Factory does not choose a local task for you' >&2; exit 2; }
[[ "$trials" =~ ^[1-9][0-9]*$ ]] || { printf 'trials must be positive\n' >&2; exit 2; }
(("${#scope_paths[@]}" > 0)) || { printf '%s\n' 'at least one --scope PATH is required' >&2; exit 2; }
[[ -d "$context_cwd" ]] || { printf 'context cwd is not a directory: %s\n' "$context_cwd" >&2; exit 2; }
for scope_path in "${scope_paths[@]}"; do
  [[ -d "$scope_path" ]] || { printf 'scope is not a directory: %s\n' "$scope_path" >&2; exit 2; }
done

mkdir -p "$output_root"
summary="$output_root/summary.yaml"
{
  printf '%s\n' 'version: 1' 'kind: runtime_parity_summary'
  printf 'context_task: %s\ntrials: %s\n' "$context_task" "$trials"
  printf '%s\n' 'results:'
} > "$summary"

for runtime in "${runtime_list[@]}"; do
  for scenario in "${scenario_list[@]}"; do
    prompt_file="$factory_root/fixtures/runtime-parity/$scenario.txt"
    case "$scenario" in
      architecture) expected_terms=(重疊 風險 驗證) ;;
      operations) expected_terms=(事實 假設 停止) ;;
      conversation) expected_terms=(狀態 例子) ;;
    esac
    expect_args=()
    for term in "${expected_terms[@]}"; do
      expect_args+=(--expect "$term")
    done
    scope_args=()
    for scope_path in "${scope_paths[@]}"; do
      scope_args+=(--scope "$scope_path")
    done
    for ((trial = 1; trial <= trials; trial++)); do
      result="$output_root/${runtime}-${scenario}-${trial}.yaml"
      log="$result.log"
      set +e
      AGENT_PLUGIN_ID="$runtime" "$factory_root/plugins/runtime-adapter.sh" run \
        --prompt-file "$prompt_file" \
        --cwd "$context_cwd" \
        --context-task "$context_task" \
        --context-cwd "$context_cwd" \
        "${scope_args[@]}" \
        "${expect_args[@]}" \
        --output "$result" >"$log" 2>&1
      rc=$?
      set -e
      result_status="$(awk '$1 == "status:" { print $2; exit }' "$result" 2>/dev/null || true)"
      [[ -n "$result_status" ]] || result_status=error
      {
        printf '  - runtime: %s\n' "$runtime"
        printf '    scenario: %s\n' "$scenario"
        printf '    trial: %s\n' "$trial"
        printf '    status: %s\n' "$result_status"
        printf '    exit_status: %s\n' "$rc"
        printf '    result_path: %s\n' "$result"
        printf '    final_output_path: %s\n' "$result.final"
      } >> "$summary"
      printf '%s/%s trial %s: status=%s exit=%s\n' "$runtime" "$scenario" "$trial" "$result_status" "$rc"
    done
  done
done

printf 'summary_path: %s\n' "$summary"
