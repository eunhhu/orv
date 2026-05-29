#!/usr/bin/env sh
set -eu

SCRIPT_DIR=$(CDPATH= cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd "$SCRIPT_DIR/.." && pwd)

if [ -z "${ORV_BIN:-}" ]; then
  if [ -x "$REPO_ROOT/target/debug/orv" ]; then
    ORV_BIN="$REPO_ROOT/target/debug/orv"
  else
    ORV_BIN="orv"
  fi
fi

if ! command -v curl >/dev/null 2>&1; then
  printf 'shop acceptance smoke requires curl\n' >&2
  exit 127
fi

if ! command -v "$ORV_BIN" >/dev/null 2>&1 && [ ! -x "$ORV_BIN" ]; then
  printf 'shop acceptance smoke requires orv; set ORV_BIN=/path/to/orv\n' >&2
  exit 127
fi

WORKDIR="${ORV_SHOP_ACCEPTANCE_DIR:-$(mktemp -d /tmp/orv-shop-acceptance.XXXXXX)}"
SHOP_DIR="$WORKDIR/shop"
SERVER_PID=""

cleanup() {
  if [ -n "$SERVER_PID" ]; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT INT TERM

rm -rf "$SHOP_DIR"
"$ORV_BIN" init "$SHOP_DIR" --template shop

cd "$SHOP_DIR"
"$ORV_BIN" check .
"$ORV_BIN" build . --prod --out dist
"$ORV_BIN" verify-build dist
"$ORV_BIN" deploy-env-check dist

"$ORV_BIN" run-build dist &
SERVER_PID="$!"

attempt=0
until curl -fsS "${ORV_BASE_URL:-http://127.0.0.1:8080}/" >/dev/null 2>&1; do
  attempt=$((attempt + 1))
  if [ "$attempt" -ge "${ORV_SHOP_ACCEPTANCE_READY_ATTEMPTS:-50}" ]; then
    printf 'shop acceptance smoke failed: server did not become ready\n' >&2
    exit 1
  fi
  sleep 0.2
done

ORV_BIN="$ORV_BIN" sh dist/deploy/smoke-test.sh
"$ORV_BIN" benchmark-prepare dist --participants 2 > dist/deploy/benchmark-prepare.json
"$ORV_BIN" benchmark-report dist > dist/deploy/benchmark-report.json

printf 'shop acceptance smoke passed\n'
printf 'shop_dir=%s\n' "$SHOP_DIR"
printf 'smoke_output=%s\n' "$SHOP_DIR/dist/deploy/smoke-output.txt"
printf 'benchmark_prepare=%s\n' "$SHOP_DIR/dist/deploy/benchmark-prepare.json"
printf 'benchmark_report=%s\n' "$SHOP_DIR/dist/deploy/benchmark-report.json"
