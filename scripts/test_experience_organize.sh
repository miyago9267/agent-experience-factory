#!/usr/bin/env bash
set -Eeuo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/test_support.sh"
build_test_binary
register_test_task other-test implementation
old_umask=$(umask)
umask 000

for runtime in codex claude; do
  "$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
    --cwd "$workspace_root" --kind explicit_preference --runtime "$runtime" \
    --scope "$workspace_root" --source user_message \
    --summary 'Confirm rollback before deployment.' >/dev/null
done
"$binary" observe --workspace-root "$workspace_root" --task "$experience_task_id" \
  --cwd "$workspace_root" --kind counterexample --runtime grok \
  --scope "$workspace_root" --source verification \
  --summary 'Confirm rollback before deployment.' >/dev/null
output=$("$binary" organize --workspace-root "$workspace_root" \
  --task "$experience_task_id" --cwd "$workspace_root")
printf '%s\n' "$output" | grep -Fq 'candidate_count: 1'
printf '%s\n' "$output" | grep -Fq 'observation_count: 3'
printf '%s\n' "$output" | grep -Fq 'status: candidate_only'

candidate=$(find "$test_root/experience-data/candidates" -type f -name '*.yaml' -print)
test -f "$candidate"
grep -Fq 'status: candidate' "$candidate"
grep -Fq 'counterexample_observation_ids:' "$candidate"
test "$(grep -E '^- obs-' "$candidate" | sort -u | wc -l | tr -d ' ')" = 3
test "$(test_mode "$candidate")" = 600
test "$(test_mode "$test_root/experience-data/candidates")" = 700
before_hash=$(test_sha256 "$candidate")
"$binary" organize --workspace-root "$workspace_root" \
  --task "$experience_task_id" --cwd "$workspace_root" >/dev/null
test "$before_hash" = "$(test_sha256 "$candidate")"

cross_task_root="$test_root/cross-task-data"
for task_id in "$experience_task_id" other-test; do
  AGENT_EXPERIENCE_DATA_ROOT="$cross_task_root" "$binary" observe \
    --workspace-root "$workspace_root" --task "$task_id" --cwd "$workspace_root" \
    --kind explicit_preference --runtime codex --scope "$workspace_root" \
    --source user_message --summary 'Keep experiences isolated by task.' >/dev/null
done
for task_id in "$experience_task_id" other-test; do
  AGENT_EXPERIENCE_DATA_ROOT="$cross_task_root" "$binary" organize \
    --workspace-root "$workspace_root" --task "$task_id" --cwd "$workspace_root" >/dev/null
done
test "$(find "$cross_task_root/candidates" -type f -name '*.yaml' | wc -l | tr -d ' ')" = 2

scope_order_root="$test_root/scope-order-data"
AGENT_EXPERIENCE_DATA_ROOT="$scope_order_root" "$binary" observe \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root" \
  --kind explicit_preference --runtime codex --scope "$workspace_root" \
  --scope "$workspace_root/secondary" --source user_message \
  --summary 'Scope ordering should not split equivalent experiences.' >/dev/null
AGENT_EXPERIENCE_DATA_ROOT="$scope_order_root" "$binary" observe \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root" \
  --kind explicit_preference --runtime claude --scope "$workspace_root/secondary" \
  --scope "$workspace_root" --source user_message \
  --summary 'Scope ordering should not split equivalent experiences.' >/dev/null
scope_output=$(AGENT_EXPERIENCE_DATA_ROOT="$scope_order_root" "$binary" organize \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root")
printf '%s\n' "$scope_output" | grep -Fq 'candidate_count: 1'
printf '%s\n' "$scope_output" | grep -Fq 'observation_count: 2'

invalid_root="$test_root/invalid-data"
mkdir -p "$invalid_root/observations"
chmod 700 "$invalid_root" "$invalid_root/observations"
valid_observation=$(find "$test_root/experience-data/observations" -type f -name '*.yaml' -print | head -1)
cp "$valid_observation" "$invalid_root/observations/invalid.yaml"
perl -0pi -e 's/^kind:.*$/kind: unknown_kind/m' "$invalid_root/observations/invalid.yaml"
invalid_output=$(AGENT_EXPERIENCE_DATA_ROOT="$invalid_root" "$binary" organize \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root")
printf '%s\n' "$invalid_output" | grep -Fq 'candidate_count: 0'
printf '%s\n' "$invalid_output" | grep -Fq 'rejected_count: 1'

symlink_root="$test_root/symlink-data"
mkdir -p "$symlink_root/observations"
chmod 700 "$symlink_root" "$symlink_root/observations"
printf '%s\n' 'external observation must not be read' >"$test_root/external.yaml"
ln -s "$test_root/external.yaml" "$symlink_root/observations/external.yaml"
symlink_output=$(AGENT_EXPERIENCE_DATA_ROOT="$symlink_root" "$binary" organize \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root")
printf '%s\n' "$symlink_output" | grep -Fq 'candidate_count: 0'
printf '%s\n' "$symlink_output" | grep -Fq 'rejected_count: 1'

ln -s "$test_root/experience-data" "$test_root/data-alias"
if AGENT_EXPERIENCE_DATA_ROOT="$test_root/data-alias" "$binary" organize \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root" >/dev/null 2>&1; then
  echo 'observation storage symlink guard unexpectedly succeeded' >&2
  exit 1
fi

permissive_root="$test_root/permissive-data"
mkdir -p "$permissive_root/observations"
chmod 777 "$permissive_root" "$permissive_root/observations"
if AGENT_EXPERIENCE_DATA_ROOT="$permissive_root" "$binary" organize \
  --workspace-root "$workspace_root" --task "$experience_task_id" --cwd "$workspace_root" >/dev/null 2>&1; then
  echo 'permissive storage mode unexpectedly succeeded' >&2
  exit 1
fi
test ! -d "$permissive_root/candidates"

umask "$old_umask"
printf '%s\n' 'experience_organize: OK'
