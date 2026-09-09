# orv

orv는 `.orv` 소스에서 웹 앱, 실행 결과, 배포 산출물과 원본 소스의 연결을
다루는 Rust 기반 언어·도구체인 MVP입니다. 현재 reference runtime으로
HTTP 서버와 SQLite 기반 쇼핑몰 예제를 실행할 수 있습니다.

## 로컬에서 실행

Rust/rustup과 C/C++ 빌드 도구가 필요합니다. 저장소의
`rust-toolchain.toml`이 개발 도구체인을 Rust 1.97.1로 고정하며,
선언된 최소 Rust 버전은 1.86.0입니다.

저장소 루트에서 실행합니다.

```sh
cargo build --locked -p orv-cli
export PATH="$PWD/target/debug:$PATH"
orv init my-shop --template shop
cd my-shop
orv check .
orv build . --prod --out dist
orv verify-build dist
orv deploy-env-check dist
orv run-build dist
```

서버를 실행한 터미널을 유지한 채 브라우저에서
[로컬 쇼핑몰](http://127.0.0.1:8080)을 엽니다. 종료는 `Ctrl+C`입니다.
다른 터미널에서 같은 `my-shop` 디렉터리와 `orv` PATH를 사용해
`sh dist/deploy/smoke-test.sh`로 생성된 앱을 확인할 수 있습니다.
실제 컨테이너 배포 절차는 생성된 `dist/deploy/README.md`를 따릅니다.
직접 실행의 bind 주소는 `127.0.0.1`, 생성된 컨테이너의 기본값은
`ORV_HOST=0.0.0.0`입니다.

## 개발 검증

저장소 루트에서 실행합니다. Docker smoke에는 실행 중인 Docker daemon과
이미지 다운로드가, grammar smoke에는 Node.js가 필요합니다.

```sh
bash scripts/preflight.sh --all
cargo build --locked -p orv-cli
sh scripts/shop_acceptance_smoke.sh
sh scripts/container_smoke.sh
node tree-sitter-orv/test/grammar-smoke.cjs
```

## 프로젝트 문서

- [현재 구현 범위와 상태](docs/IMPLEMENTATION_STATUS.md) / [기능별 근거](docs/IMPLEMENTATION_MATRIX.md)
- [언어 명세](docs/SPEC.md) / [아키텍처와 모듈 소유권](docs/ARCHITECTURE.md)
- [CLI·에디터·빌드 운영 기능](docs/OPERATIONAL_SURFACES.md)
- [테스트 실행과 중복 방지 기준](docs/TESTING.md)
- [표준 라이브러리와 plugin 경계](docs/PLATFORM_BOUNDARY.md)
- [5시간 쇼핑몰 benchmark](docs/BENCHMARK_SHOP_5H.md) / [사람 대상 실행 안내](docs/HUMAN_BENCHMARK_RUNBOOK.md)

자동화된 쇼핑몰 실행 경로는 테스트 대상입니다. 실제 비개발자 최소 2명의
5시간 실측은 아직 없으며, production 수준의 native compiler, editor UI,
외부 provider SDK와 plugin 실행 확장은 구현 상태표의 범위를 따릅니다.
