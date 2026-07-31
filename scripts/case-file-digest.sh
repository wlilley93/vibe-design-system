#!/usr/bin/env bash
# The digest over a submission's case file.
#
# A submission carries `case_file_digest` so the bench rules on a FIXED record.
# SUBMISSION-VDS-007 carried sixty-four zeros - a placeholder that reads exactly
# like a computed digest - which means the evidence could change under a pending
# question and nothing would say so. A court deciding on a moving record is the
# same defect as a proof over an un-digested reading, already filed as the
# reason the geometry reading carries one.
#
# Reproducible and derived: name the files, get the number. Never hand-typed.
#
# usage: scripts/case-file-digest.sh <file>...
set -euo pipefail
[ $# -gt 0 ] || { echo "usage: $0 <file>..." >&2; exit 2; }
for f in "$@"; do
  [ -f "$f" ] || { echo "case file names a path that does not exist: $f" >&2; exit 1; }
done
{
  for f in "$@"; do
    printf '%s\0' "$f"
    cat "$f"
    printf '\n'
  done
} | sha256sum | awk '{print "sha256:" $1}'
