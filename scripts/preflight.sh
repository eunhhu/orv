#!/usr/bin/env bash
# Local commit gate: mirrors .github/workflows/ci.yml lint job.
# Usage: scripts/preflight.sh            (lint only, fast)
#        scripts/preflight.sh --test     (lint + full workspace tests)
# Install as a hook: ln -sf ../../scripts/preflight.sh .git/hooks/pre-push
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

cargo fmt --all --check
cargo clippy --workspace --all-targets

if [[ "${1:-}" == "--test" ]]; then
  cargo test --workspace
fi

echo "preflight ok"
