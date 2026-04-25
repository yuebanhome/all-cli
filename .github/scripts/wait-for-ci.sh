#!/usr/bin/env bash
# Wait for all per-CLI `ci-*` workflow runs (matching this PR's head SHA)
# to complete. A workflow that didn't trigger at all (path filter mismatch)
# is treated as success. Skipped runs also count as success.
# Required env: GH_TOKEN, OWNER, REPO, HEAD_SHA
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN required}"
: "${OWNER:?OWNER required}"
: "${REPO:?REPO required}"
: "${HEAD_SHA:?HEAD_SHA required}"

mapfile -t cli_workflows < <(
  gh api "repos/${OWNER}/${REPO}/actions/workflows" --paginate \
    --jq '.workflows[] | select(.path | test("\\.github/workflows/ci-.*\\.yml$")) | .name'
)

if (( ${#cli_workflows[@]} == 0 )); then
  echo "No per-CLI ci-* workflows found; nothing to wait for."
  exit 0
fi
echo "Watching: ${cli_workflows[*]}"

deadline=$(( SECONDS + 1800 ))   # 30 min
while (( SECONDS < deadline )); do
  all_done=true
  any_failed=false
  for name in "${cli_workflows[@]}"; do
    run_json=$(gh api -X GET "repos/${OWNER}/${REPO}/actions/runs" \
                 -f head_sha="${HEAD_SHA}" \
                 -f event=pull_request \
                 --jq ".workflow_runs[] | select(.name == \"${name}\")" 2>/dev/null || true)
    if [[ -z "$run_json" ]]; then
      # Workflow not triggered for this SHA → treat as success.
      continue
    fi
    status=$(echo "$run_json" | jq -r '.status' | head -1)
    conclusion=$(echo "$run_json" | jq -r '.conclusion' | head -1)
    case "$status" in
      completed)
        case "$conclusion" in
          success|skipped|neutral) ;;
          *) echo "::error::Workflow '$name' concluded '$conclusion'"; any_failed=true ;;
        esac
        ;;
      *)
        all_done=false
        ;;
    esac
  done
  if $any_failed; then exit 1; fi
  if $all_done; then echo "All per-CLI CI workflows passed."; exit 0; fi
  sleep 15
done

echo "::error::Timed out waiting for per-CLI CI workflows."
exit 1
