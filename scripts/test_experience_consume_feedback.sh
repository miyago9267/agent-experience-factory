#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary
mkdir -p "$workspace_root/src"

"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind explicit_preference --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'Confirmed experience should be consumable by supported runtimes.' >/dev/null
"$binary" organize --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" >/dev/null
candidate=$(find "$test_root/experience-data/candidates" -type f -name '*.yaml' -print)
candidate_id=$(sed -n 's/^candidate_id: //p' "$candidate")
"$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision confirm \
  --owner experience_library --reason 'Confirmed in generic integration test.' >/dev/null
for runtime in codex claude; do
  "$binary" consume --workspace-root "$workspace_root" --task "$experience_task_id" \
    --cwd "$workspace_root" --runtime "$runtime" --candidate "$candidate_id" >/dev/null
done
codex_bundle="$test_root/experience-data/consumptions/$experience_task_id-codex.yaml"
claude_bundle="$test_root/experience-data/consumptions/$experience_task_id-claude.yaml"
grep -Fq 'runtime: codex' "$codex_bundle"
grep -Fq 'runtime: claude' "$claude_bundle"
grep -Fq "candidate_id: $candidate_id" "$codex_bundle"
test "$(test_mode "$test_root/experience-data/consumptions")" = 700
test "$(test_mode "$codex_bundle")" = 600

before_candidate=$(test_sha256 "$candidate")
if "$binary" feedback --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --runtime claude \
  --kind counterexample --action narrow --summary 'A scope is required.' >/dev/null 2>&1; then
  echo 'feedback without explicit scope unexpectedly succeeded' >&2
  exit 1
fi
test "$(test_sha256 "$candidate")" = "$before_candidate"
if "$binary" feedback --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --runtime claude \
  --kind counterexample --action narrow --scope "$workspace_root" \
  --summary 'The new scope must be narrower.' >/dev/null 2>&1; then
  echo 'narrowing to the same scope unexpectedly succeeded' >&2
  exit 1
fi

"$binary" feedback --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --runtime claude \
  --kind counterexample --action narrow --scope "$workspace_root/src" \
  --summary 'This experience only applies to the source subtree.' >/dev/null
grep -Fq 'status: confirmed' "$candidate"
grep -Fq 'action: narrow' "$candidate"
grep -Fq -- "- $workspace_root/src" "$candidate"
test ! -e "$codex_bundle"
test ! -e "$claude_bundle"

"$binary" feedback --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --runtime codex \
  --kind correction --action downgrade --scope "$workspace_root/src" \
  --summary 'A correction needs another review.' >/dev/null
grep -Fq 'status: candidate' "$candidate"
"$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision confirm \
  --owner experience_library --reason 'Reconfirmed after correction review.' >/dev/null
"$binary" feedback --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --runtime grok \
  --kind counterexample --action retire --scope "$workspace_root/src" \
  --summary 'Retire after repeated contrary evidence.' >/dev/null
grep -Fq 'status: retired' "$candidate"
if "$binary" consume --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --runtime gemini --candidate "$candidate_id" >/dev/null 2>&1; then
  echo 'retired experience was unexpectedly consumable' >&2
  exit 1
fi
"$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision restore \
  --owner experience_library --reason 'Restore after a later review.' >/dev/null
"$binary" consume --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --runtime gemini --candidate "$candidate_id" >/dev/null

printf '%s\n' 'experience_consume_feedback: OK'
