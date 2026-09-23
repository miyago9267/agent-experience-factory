#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary

"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind explicit_preference --runtime codex \
  --scope "$workspace_root" --source user_message \
  --summary 'A review decision must remain explicit.' >/dev/null
"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind counterexample --runtime claude \
  --scope "$workspace_root" --source verification \
  --summary 'A review decision must remain explicit.' \
  --hypothesis 'A counterexample should remain visible.' >/dev/null
"$binary" organize --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" >/dev/null

review_output=$("$binary" review --workspace-root "$workspace_root" \
  --task "$experience_task_id" --cwd "$workspace_root")
printf '%s\n' "$review_output" | grep -Fq 'human_gate: required'
queue=$(find "$test_root/experience-data/reviews" -type f -name '*.yaml' -print)
grep -Fq 'human_gate: required' "$queue"
grep -Fq 'action:' "$queue"
grep -Fq 'source_count: 2' "$queue"
grep -Fq 'counterexample_count: 1' "$queue"
test "$(test_mode "$(dirname "$queue")")" = 700
test "$(test_mode "$queue")" = 600

candidate=$(find "$test_root/experience-data/candidates" -type f -name '*.yaml' -print)
candidate_id=$(sed -n 's/^candidate_id: //p' "$candidate")
cp "$candidate" "$candidate.backup"
perl -0pi -e 's/^candidate_id: .*/candidate_id: candidate-mismatch/m' "$candidate"
if "$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision confirm >/dev/null 2>&1; then
  echo 'candidate filename mismatch unexpectedly succeeded' >&2
  exit 1
fi
mv "$candidate.backup" "$candidate"
if "$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision reject >/dev/null 2>&1; then
  echo 'reject without reason unexpectedly succeeded' >&2
  exit 1
fi
decision_output=$("$binary" decide --workspace-root "$workspace_root" \
  --task "$experience_task_id" --cwd "$workspace_root" --decision confirm \
  --reason 'Confirmed during generic review test.')
printf '%s\n' "$decision_output" | grep -Fq 'writeback: none'
grep -Fq 'status: confirmed' "$candidate"
grep -Fq 'decision: confirm' "$candidate"

"$binary" organize --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" >/dev/null
grep -Fq 'status: confirmed' "$candidate"
"$binary" decide --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --candidate "$candidate_id" --decision revoke \
  --reason 'A later counterexample requires rollback.' >/dev/null
grep -Fq 'status: superseded' "$candidate"
grep -Fq 'decision: revoke' "$candidate"
test "$(grep -c '^-[[:space:]]decision:' "$candidate")" = 2

printf '%s\n' 'experience_review: OK'
