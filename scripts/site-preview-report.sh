#!/usr/bin/env bash
# Reports a site preview deploy on its pull request (.github/workflows/site-preview.yml):
# a "Site preview" commit status in the PR's checks, and one comment on the PR
# that is edited on every push instead of adding a new one.
#
#   scripts/site-preview-report.sh success|failure
#
# Needs GH_TOKEN, GITHUB_REPOSITORY, PR (number), HEAD_SHA and RUN_URL.
set -euo pipefail

state="$1"
url="https://pr-$PR.clipsa-bhi.pages.dev"
case "$state" in
  success)
    target="$url"
    description="Deployed to pr-$PR.clipsa-bhi.pages.dev"
    row="Ready ([logs]($RUN_URL)) | [Visit preview]($url)" ;;
  failure)
    target="$RUN_URL"
    description="Preview deploy failed"
    row="Failed ([logs]($RUN_URL)) | none" ;;
  *) echo "usage: $0 success|failure" >&2; exit 2 ;;
esac

gh api "repos/$GITHUB_REPOSITORY/statuses/$HEAD_SHA" --silent \
  -f state="$state" -f context="Site preview" -f target_url="$target" -f description="$description"

marker='<!-- clipsa-site-preview -->'
body="$marker
**Site preview** of clipsa.dev for $HEAD_SHA

| Status | Preview | Updated (UTC) |
| :-- | :-- | :-- |
| $row | $(date -u '+%Y-%m-%d %H:%M') |"

# Only the workflow's own comment: anyone could paste the marker into theirs.
comment="$(gh api --paginate "repos/$GITHUB_REPOSITORY/issues/$PR/comments" \
  --jq ".[] | select(.user.login == \"github-actions[bot]\" and (.body | startswith(\"$marker\"))) | .id" | head -n 1)"
if [ -n "$comment" ]; then
  gh api -X PATCH "repos/$GITHUB_REPOSITORY/issues/comments/$comment" --silent -f body="$body"
else
  gh api "repos/$GITHUB_REPOSITORY/issues/$PR/comments" --silent -f body="$body"
fi
