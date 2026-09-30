#!/usr/bin/env bash
# Attach files to a GitHub Release for tag vX.Y.Z.
# The macOS job passes --edit-notes so the Apple signing footer stays on the
# release. Linux and Windows only upload, unless they are first and must create it.
#
#   scripts/publish-release.sh [--edit-notes] TAG VERSION FILE...
set -euo pipefail

edit_notes=0
if [[ "${1:-}" == "--edit-notes" ]]; then
  edit_notes=1
  shift
fi

tag="${1:?tag}"
version="${2:?version}"
shift 2
if [[ $# -lt 1 ]]; then
  echo "no files to publish" >&2
  exit 1
fi

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

pkg="$(node -p "require('./package.json').version")"
if [[ "$version" != "$pkg" ]]; then
  echo "tag $tag does not match package.json version $pkg" >&2
  exit 1
fi

notes_args=(scripts/changelog-section.py "$version")
if [[ "$edit_notes" -eq 1 ]]; then
  notes_args+=(--signed-footer)
fi
# Windows runners open Python stdout as cp1252. UTF-8 mode must be set
# before the interpreter starts; reconfigure does not stick on a redirect.
export PYTHONUTF8=1
export PYTHONIOENCODING=utf-8
python3 "${notes_args[@]}" > notes.md

for attempt in 1 2 3 4 5 6; do
  if gh release view "$tag" >/dev/null 2>&1; then
    if [[ "$edit_notes" -eq 1 ]]; then
      gh release edit "$tag" --title "1537paperstreet $version" --notes-file notes.md
    fi
    gh release upload "$tag" "$@" --clobber
    exit 0
  fi
  if gh release create "$tag" \
    --title "1537paperstreet $version" \
    --notes-file notes.md \
    "$@"; then
    exit 0
  fi
  echo "release publish raced; retrying in 5 seconds (attempt ${attempt})" >&2
  sleep 5
done

echo "could not publish $tag" >&2
exit 1
