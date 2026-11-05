#!/bin/sh
# Copy outside the target Git project, review, and pin its SHA-256 in the request.
set -eu
cd "$1"
exec cargo fmt --check
