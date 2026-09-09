#!/usr/bin/env sh
# Exercise the generated deployment image through a real published container port.
set -eu

REPO_ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
WORKDIR=$(mktemp -d "${TMPDIR:-/tmp}/orv-container-smoke.XXXXXX")
RUN_ID="$(date +%s)-$$"
RUNTIME_IMAGE="orv-container-smoke-runtime:$RUN_ID"
APP_IMAGE="orv-container-smoke-app:$RUN_ID"
CONTAINER_ID=""
BUILD_CONTAINER_ID=""

cleanup() {
  if [ -n "$CONTAINER_ID" ]; then
    docker rm -f "$CONTAINER_ID" >/dev/null 2>&1 || true
  fi
  if [ -n "$BUILD_CONTAINER_ID" ]; then
    docker rm -f "$BUILD_CONTAINER_ID" >/dev/null 2>&1 || true
  fi
  docker image rm "$APP_IMAGE" "$RUNTIME_IMAGE" >/dev/null 2>&1 || true
  rm -rf "$WORKDIR"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

docker info >/dev/null
MSRV=$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' "$REPO_ROOT/Cargo.toml")
docker build --build-arg "RUST_VERSION=$MSRV" \
  -f "$REPO_ROOT/scripts/Dockerfile.runtime-test" -t "$RUNTIME_IMAGE" "$REPO_ROOT"

cat > "$WORKDIR/app.orv" <<'ORV'
@server {
  @listen 8080
  @route GET /ping { @respond 200 { ok: true } }
}
ORV

BUILD_CONTAINER_ID=$(docker create -w /work --entrypoint /bin/sh "$RUNTIME_IMAGE" \
  -c 'orv build app.orv --prod --out dist && orv verify-build dist')
docker cp "$WORKDIR/app.orv" "$BUILD_CONTAINER_ID:/work/app.orv"
docker start -a "$BUILD_CONTAINER_ID"
docker cp "$BUILD_CONTAINER_ID:/work/dist" "$WORKDIR/dist"
docker rm "$BUILD_CONTAINER_ID" >/dev/null
BUILD_CONTAINER_ID=""
docker build --build-arg "ORV_RUNTIME_IMAGE=$RUNTIME_IMAGE" \
  -f "$WORKDIR/dist/deploy/Dockerfile" -t "$APP_IMAGE" "$WORKDIR/dist"

CONTAINER_ID=$(docker run --rm -d -p 127.0.0.1::8080 "$APP_IMAGE")
ADDRESS=$(docker port "$CONTAINER_ID" 8080/tcp)
attempt=0
until curl --silent --show-error --fail --connect-timeout 1 --max-time 2 \
  "http://$ADDRESS/ping" > "$WORKDIR/response.json" 2>/dev/null; do
  attempt=$((attempt + 1))
  if [ "$attempt" -ge 50 ]; then
    docker logs "$CONTAINER_ID" >&2
    printf 'container smoke failed: published port did not become reachable\n' >&2
    exit 1
  fi
  sleep 0.2
done
if ! grep -F '"ok":true' "$WORKDIR/response.json" >/dev/null; then
  cat "$WORKDIR/response.json" >&2
  exit 1
fi
printf 'container smoke passed: generated image reachable through published port\n'
