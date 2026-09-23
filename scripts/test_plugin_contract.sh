#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary

test_home="$test_root/home"
mock_bin="$test_root/mock-bin"
mkdir -p "$test_home" "$mock_bin"
cat >"$mock_bin/opencode" <<'SH'
#!/bin/sh
printf '%s\n' 'ProviderAuthError: deterministic test fixture'
SH
chmod +x "$mock_bin/opencode"
entrypoint="$factory_root/scripts/agent-workflow"
result_path="$test_root/plugin-result.yaml"
opencode_result="$test_root/opencode-result.yaml"
prompt_file="$factory_root/fixtures/runtime-adapter-smoke/prompt.txt"

run_factory() {
  env HOME="$test_home" XDG_CONFIG_HOME="$test_home/.config" \
    AGENT_FACTORY_CONFIG_FILE="$test_root/no-local-config" \
    AGENT_FACTORY_WORKSPACE_ROOT="$workspace_root" \
    AGENT_CONTEXT_HARNESS_BIN="$binary" \
    PATH="$mock_bin:$PATH" "$entrypoint" "$@"
}

expected_plugins=$'claude\ncodex\ngemini\ngrok\nopencode\nwaza'
actual_plugins="$(run_factory plugin list)"
if [[ "$actual_plugins" != "$expected_plugins" ]]; then
  printf 'unexpected plugin list:\n%s\n' "$actual_plugins" >&2
  exit 1
fi
run_factory plugin doctor >/dev/null
for runtime in claude codex gemini grok; do
  runtime_plan="$(run_factory plugin plan --id "$runtime")"
  [[ "$runtime_plan" == *'plugin_type: runtime_adapter'* ]] || exit 1
  [[ "$runtime_plan" == *'result: plugin_result'* ]] || exit 1
  runtime_result="$test_root/$runtime-result.yaml"
  set +e
  run_factory plugin run --id "$runtime" --output "$runtime_result" >/dev/null 2>&1
  runtime_exit=$?
  set -e
  [[ "$runtime_exit" -eq 2 ]] || exit 1
  grep -q '^status: error$' "$runtime_result"
  grep -q 'prompt file is required' "$runtime_result"
done

run_factory plugin plan --id opencode | grep -Fq 'plugin_id: opencode'
if run_factory plugin run --id opencode --context-task "$experience_task_id" \
  --context-cwd "$workspace_root" --prompt-file "$prompt_file" \
  --output "$opencode_result" >/dev/null 2>&1; then
  printf '%s\n' 'mocked OpenCode capability gap unexpectedly passed' >&2
  exit 1
fi
grep -q '^status: capability_gap$' "$opencode_result"

plan="$(run_factory plugin plan --id waza)"
[[ "$plan" == *'trials_per_task: 2'* ]]
run_factory plugin run --id waza --engine mock --output "$result_path" >/dev/null
grep -q '^kind: plugin_result$' "$result_path"
grep -q '^status: pass$' "$result_path"
grep -q '^eval:$' "$result_path"
grep -q '^  trials_per_task: 2$' "$result_path"
grep -q 'no provider execution performed' "$result_path"

if run_factory plugin run --id waza --engine unsupported >/dev/null 2>&1; then
  printf '%s\n' 'unsupported engine unexpectedly passed' >&2
  exit 1
fi

printf '%s\n' 'plugin contract: OK'
