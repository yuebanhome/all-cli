#!/usr/bin/env bash
# Wait for all per-CLI `ci-*` workflow runs (matching this PR's head SHA)
# to complete. A workflow that didn't trigger at all (path filter mismatch)
# is treated as success — that's the whole point of this script:
# path-filtered required checks would otherwise dead-lock PRs that don't
# touch every CLI subdirectory.
# Required env: GH_TOKEN, OWNER, REPO, HEAD_SHA
set -euo pipefail

: "${GH_TOKEN:?GH_TOKEN required}"
: "${OWNER:?OWNER required}"
: "${REPO:?REPO required}"
: "${HEAD_SHA:?HEAD_SHA required}"

# Discover ci-* workflows by ID (not name) so we can query
# /actions/workflows/{id}/runs directly — avoids name-quoting and
# the SIGPIPE-vs-pipefail risk of stream-filtering by name.
mapfile -t cli_workflow_ids < <(
  gh api "repos/${OWNER}/${REPO}/actions/workflows" --paginate \
    --jq '.workflows[] | select(.path | test("\\.github/workflows/ci-.*\\.yml$")) | .id'
)
mapfile -t cli_workflow_names < <(
  gh api "repos/${OWNER}/${REPO}/actions/workflows" --paginate \
    --jq '.workflows[] | select(.path | test("\\.github/workflows/ci-.*\\.yml$")) | .name'
)

if (( ${#cli_workflow_ids[@]} == 0 )); then
  echo "No per-CLI ci-* workflows found; nothing to wait for."
  exit 0
fi

# Print a parallel listing for diagnostics.
for i in "${!cli_workflow_ids[@]}"; do
  echo "Watching: id=${cli_workflow_ids[$i]} name=${cli_workflow_names[$i]}"
done

deadline=$(( SECONDS + 1800 ))                  # hard cap 30 min
min_wait_until=$(( SECONDS + 60 ))              # never declare success in the first 60s
                                                # (avoids early-empty race before ci-* runs register)

declare -a pending=()
while (( SECONDS < deadline )); do
  all_done=true
  any_failed=false
  pending=()

  for i in "${!cli_workflow_ids[@]}"; do
    id="${cli_workflow_ids[$i]}"
    name="${cli_workflow_names[$i]}"

    # Capture API result and exit code separately. Treat real errors
    # as transient (don't mark "no run") so we re-poll instead of
    # silently passing through.
    set +e
    run_json=$(gh api -X GET "repos/${OWNER}/${REPO}/actions/workflows/${id}/runs" \
                 -f head_sha="${HEAD_SHA}" \
                 -f event=pull_request \
                 --jq '[.workflow_runs[]] | sort_by(.created_at) | reverse | first' 2>/dev/null)
    rc=$?
    set -e

    if (( rc != 0 )); then
      echo "::warning::gh api failed for workflow '${name}' (id=${id}); will retry"
      all_done=false
      pending+=("${name}")
      continue
    fi

    if [[ -z "$run_json" || "$run_json" == "null" ]]; then
      # Path-filter mismatch: the workflow's `paths:` filter excluded this
      # PR's diff, so no run was created. Required checks would dead-lock
      # without this clause; treat absence as success.
      continue
    fi

    status=$(echo "$run_json" | jq -r '.status // empty')
    conclusion=$(echo "$run_json" | jq -r '.conclusion // empty')
    case "$status" in
      completed)
        case "$conclusion" in
          success|skipped|neutral) ;;
          *) echo "::error::Workflow '${name}' concluded '${conclusion}'"; any_failed=true ;;
        esac
        ;;
      *)
        all_done=false
        pending+=("${name}")
        ;;
    esac
  done

  if $any_failed; then exit 1; fi
  if $all_done && (( SECONDS >= min_wait_until )); then
    echo "All per-CLI CI workflows passed."
    exit 0
  fi
  sleep 15
done

if (( ${#pending[@]} > 0 )); then
  echo "::error::Timed out waiting for per-CLI CI workflows. Still pending: ${pending[*]}"
else
  echo "::error::Timed out waiting for per-CLI CI workflows."
fi
exit 1
