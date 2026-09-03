#!/usr/bin/env bash
# Local commit gate: mirrors .github/workflows/ci.yml lint job.
# Usage: scripts/preflight.sh            (lint only, fast)
#        scripts/preflight.sh --test     (lint + full workspace tests)
#        scripts/preflight.sh --msrv     (lint + declared-MSRV check)
#        scripts/preflight.sh --all      (lint + MSRV check + full tests)
# Install as a hook: ln -sf ../../scripts/preflight.sh .git/hooks/pre-push
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

cargo fmt --all --check
cargo clippy --workspace --all-targets

case "${1:-}" in
  "") ;;
  --test)
    cargo test --workspace --all-targets
    ;;
  --msrv)
    cargo +1.86.0 check --workspace --all-targets
    ;;
  --all)
    cargo +1.86.0 check --workspace --all-targets
    cargo test --workspace --all-targets
    ;;
  *)
    echo "usage: scripts/preflight.sh [--test|--msrv|--all]" >&2
    exit 2
    ;;
esac

echo "preflight ok"
