#!/usr/bin/env bash
set -Eeuo pipefail

runtime="${AGENT_PLUGIN_ID:-${MIYAGO_PLUGIN_ID:-unknown}}"
mode=health
output=''
prompt_file=''
cwd="$PWD"
context_task=''
context_cwd="$PWD"
model=''
failure_gap=''
failure_reason=''
expected_terms=()
scope_paths=()

usage() {
  printf '%s\n' 'usage: adapter.sh <health|plan|run> [--prompt-file PATH] [--cwd PATH] [--context-task TASK_ID] [--context-cwd PATH] [--scope PATH] [--output PATH] [--model MODEL]'
}

yaml_quote() { jq -Rn --arg value "$1" '$value'; }

write_scope() {
  if ((${#scope_paths[@]} == 0)); then
    printf '%s\n' 'scope: []'
    return
  fi
  printf '%s\n' 'scope:'
  local path
  for path in "${scope_paths[@]}"; do
    printf '  - %s\n' "$(yaml_quote "$path")"
  done
}

context_binary() {
  if [[ -n "${AGENT_CONTEXT_HARNESS_BIN:-${MIYAGO_CONTEXT_HARNESS_BIN:-}}" ]]; then
    printf '%s\n' "${AGENT_CONTEXT_HARNESS_BIN:-$MIYAGO_CONTEXT_HARNESS_BIN}"
  elif command -v miyago-context-harness >/dev/null 2>&1; then
    command -v miyago-context-harness
  else
    printf '%s\n' "${HOME}/.local/bin/miyago-context-harness"
  fi
}

write_result() {
  local status="$1" summary="$2" gap="$3" exit_status="$4"
  [[ -n "$output" ]] || { printf 'status: %s\n' "$status"; return; }
  mkdir -p "$(dirname -- "$output")"
  {
    printf '%s\n' 'schema_version: "1"' 'kind: plugin_result'
    printf 'plugin_id: %s\nplugin_version: 0.1.0\nstatus: %s\n' "$runtime" "$status"
    printf '%s\n' 'task_id: provider-neutral-runtime-task' "runtime: $runtime" "model: ${model:-unknown}"
    printf '%s\n' 'scenario: external-session' 'attempt: 1' "exit_status: $exit_status" 'duration_ms: null'
    write_scope
    printf '%s\n' 'output_ref: null' 'tool_summary: []' 'graders: []' 'tasks: []'
    printf '%s\n' 'eval:' '  name: runtime-adapter-compatibility' '  version: 0.1.0' '  executor: external' '  trials_per_task: 1'
    printf 'grader_summary: %s\nevidence:\n  - %s\n' "$(yaml_quote "$summary")" "$(yaml_quote "$context_task")"
    if [[ -n "$gap" ]]; then
      printf 'capability_gaps:\n  - %s\n' "$(yaml_quote "$gap")"
    else
      printf '%s\n' 'capability_gaps: []'
    fi
    printf '%s\n' 'errors: []'
  } > "$output"
  printf 'result_path: %s\nstatus: %s\n' "$output" "$status"
}

validate_context() {
  [[ -n "$context_task" ]] || { write_result capability_gap 'runtime execution requires --context-task' 'context task was not supplied' 2; exit 2; }
  local binary
  binary="$(context_binary)"
  [[ -x "$binary" ]] || { write_result capability_gap "Context Harness binary is unavailable: $binary" 'context harness binary unavailable' 1; exit 1; }
  local root="${AGENT_FACTORY_WORKSPACE_ROOT:-${MIYAGO_AGENT_WORKSPACE_ROOT:-${HOME}/agent-workspace}}"
  "$binary" plan --workspace-root "$root" --task "$context_task" --cwd "$context_cwd" >/dev/null || {
    write_result error 'Context Harness rejected the task scope' 'context task plan rejected' 1
    exit 1
  }
}

validate_provider_output() {
  failure_gap=''
  failure_reason=''
  [[ -s "$final_output" ]] || { failure_reason='provider returned empty output'; return 1; }
  case "$runtime" in
    claude)
      if rg -q '^Error:|Error:|session limit|api_error_status":429|rate limit|Rate limit' "$raw_output"; then
        failure_gap='claude provider session limit, rate limit, authentication, or CLI input error'
        return 1
      fi
      ;;
    gemini)
      if rg -q 'IneligibleTierError|Error authenticating|critical error' "$raw_output"; then
        failure_gap='gemini CLI authentication or account tier is unavailable'
        return 1
      fi
      ;;
    opencode)
      if rg -q 'ProviderAuthError|API key is missing|Authentication' "$raw_output"; then
        failure_gap='OpenCode provider authentication or API credential is unavailable'
        return 1
      fi
      ;;
  esac
  if ((${#expected_terms[@]})); then
    local term
    for term in "${expected_terms[@]}"; do
      if ! rg -F -q -- "$term" "$final_output"; then
        failure_reason="deterministic grader missing expected term: $term"
        return 1
      fi
    done
  fi
}

run_provider() {
  local prompt="$1" raw="$2" final="$3"
  case "$runtime" in
    claude)
      local args=(--print --output-format json --no-session-persistence --permission-mode plan)
      [[ -n "$model" ]] && args+=(--model "$model")
      (cd "$cwd" && claude "${args[@]}" "$prompt") >"$raw" 2>&1
      jq -r '.result // .content // .' "$raw" >"$final" 2>/dev/null || cp "$raw" "$final"
      ;;
    codex)
      local args=(exec --json --ephemeral --sandbox read-only --cd "$cwd" -o "$final")
      [[ -n "$model" ]] && args+=(--model "$model")
      codex "${args[@]}" - <"$prompt_file" >"$raw" 2>&1
      [[ -s "$final" ]] || cp "$raw" "$final"
      ;;
    gemini)
      local args=(--prompt "$prompt" --output-format json --approval-mode plan --skip-trust)
      [[ -n "$model" ]] && args+=(--model "$model")
      (cd "$cwd" && gemini "${args[@]}") >"$raw" 2>&1
      jq -r '.response // .result // .text // .' "$raw" >"$final" 2>/dev/null || cp "$raw" "$final"
      ;;
    grok)
      local args=(--single "$prompt" --output-format json --permission-mode plan --no-subagents --no-memory --cwd "$cwd")
      [[ -n "$model" ]] && args+=(--model "$model")
      grok "${args[@]}" >"$raw" 2>&1
      jq -r '.response // .result // .content // .' "$raw" >"$final" 2>/dev/null || cp "$raw" "$final"
      ;;
    opencode)
      local args=(run --format json --pure --dir "$cwd" --agent quick-explorer)
      [[ -n "$model" ]] && args+=(--model "$model")
      (cd "$cwd" && opencode "${args[@]}" "$prompt") >"$raw" 2>&1
      jq -r 'select(.type == "text" or .type == "assistant") | (.part.text // .text // .message // empty)' \
        "$raw" >"$final" 2>/dev/null || cp "$raw" "$final"
      ;;
    *)
      write_result capability_gap "unsupported runtime: $runtime" 'runtime is not registered' 2
      return 2
      ;;
  esac
}

while (($#)); do
  case "$1" in
    --health) mode=health; shift ;;
    plan) mode=plan; shift ;;
    run) mode=run; shift ;;
    --prompt-file) prompt_file="${2:?missing value for --prompt-file}"; shift 2 ;;
    --cwd) cwd="${2:?missing value for --cwd}"; shift 2 ;;
    --context-task) context_task="${2:?missing value for --context-task}"; shift 2 ;;
    --context-cwd) context_cwd="${2:?missing value for --context-cwd}"; shift 2 ;;
    --output) output="${2:?missing value for --output}"; shift 2 ;;
    --model) model="${2:?missing value for --model}"; shift 2 ;;
    --expect) expected_terms+=("${2:?missing value for --expect}"); shift 2 ;;
    --scope) scope_paths+=("${2:?missing value for --scope}"); shift 2 ;;
    -h|--help) usage; exit 0 ;;
    *) printf 'unknown option: %s\n' "$1" >&2; usage >&2; exit 2 ;;
  esac
done

case "$mode" in
  health)
    printf 'plugin_id: %s\nplugin_type: runtime_adapter\nmanifest: OK\nbridge: external-session\n' "$runtime"
    command -v "$runtime" >/dev/null 2>&1 && printf 'provider_cli: available\n' || printf 'provider_cli: not-installed\n'
    ;;
  plan)
    printf 'plugin_id: %s\nplugin_type: runtime_adapter\nroute: external-session\nresult: plugin_result\nexecution: read-only headless CLI\n' "$runtime"
    ;;
  run)
    [[ -f "$prompt_file" ]] || { write_result error 'prompt file is required' 'prompt file missing' 2; exit 2; }
    [[ -n "$output" ]] || { write_result error '--output is required for durable result' 'output path missing' 2; exit 2; }
    validate_context
    command -v "$runtime" >/dev/null 2>&1 || { write_result capability_gap "$runtime CLI is not installed" 'provider CLI unavailable' 1; exit 1; }
    raw_output="${output}.raw"
    final_output="${output}.final"
    prompt="$(<"$prompt_file")"
    started_at="$(date +%s)"
    if run_provider "$prompt" "$raw_output" "$final_output" && validate_provider_output; then
      finished_at="$(date +%s)"
      duration=$((finished_at - started_at))
      {
        printf '%s\n' 'schema_version: "1"' 'kind: plugin_result'
        printf 'plugin_id: %s\nplugin_version: 0.1.0\nstatus: pass\n' "$runtime"
        printf '%s\n' 'task_id: provider-neutral-runtime-task' "runtime: $runtime" "model: ${model:-unknown}"
        printf '%s\n' 'scenario: external-session' 'attempt: 1' 'exit_status: 0' "duration_ms: $((duration * 1000))"
        write_scope
        printf 'output_ref: %s\n' "$(yaml_quote "$final_output")"
        printf '%s\n' 'tool_summary: []'
        if ((${#expected_terms[@]})); then
          printf '%s\n' 'graders:' '  - name: deterministic-output' '    passed: true' '    score: 1' '    feedback: expected output terms found'
        else
          printf '%s\n' 'graders: []'
        fi
        printf '%s\n' 'tasks: []'
        printf '%s\n' 'eval:' '  name: runtime-adapter-compatibility' '  version: 0.1.0' '  executor: external' '  trials_per_task: 1'
        printf 'grader_summary: %s\nevidence:\n  - %s\ncapability_gaps: []\nerrors: []\n' "$(yaml_quote 'external session completed')" "$(yaml_quote "$context_task")"
      } >"$output"
      printf 'result_path: %s\nstatus: pass\nraw_output: %s\nfinal_output: %s\n' "$output" "$raw_output" "$final_output"
    else
      if [[ -n "$failure_gap" ]]; then
        write_result capability_gap 'provider CLI returned a usable process but no usable session result' "$failure_gap" 1
      else
        write_result fail 'provider output did not satisfy deterministic grader' "${failure_reason:-provider CLI returned non-zero exit status or empty output}" 1
      fi
      exit 1
    fi
    ;;
esac
