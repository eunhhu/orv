# 테스트 실행과 소유권

CLI 통합 테스트는 `crates/orv-cli/tests/contracts.rs`에서 기능별 모듈을
실행한다. Cargo 실행 대상은 하나이지만 각 테스트의 이름과 필터는 유지된다.
`autotests = false`이므로 새 통합 테스트 파일은 `contracts.rs`에 `mod`로
등록해야 한다. 같은 기능의 사례라면 기존 모듈에 추가한다.

## 실행

```sh
# 전체 workspace
cargo test --workspace --all-targets

# CLI 단위 테스트 / 공개 계약 통합 테스트
cargo test -p orv-cli --bin orv
cargo test -p orv-cli --test contracts

# 변경한 기능만 확인
cargo test -p orv-cli --test contracts project_graph_contract::
cargo test -p orv-cli --bin orv verify_graph_artifact_cases

# fmt, clippy, 전체 테스트, MSRV
bash scripts/preflight.sh --all
```

기존 `--test project_graph_contract`처럼 파일을 실행 대상으로 지정하던
명령은 `--test contracts project_graph_contract::`로 바꾼다. 필터 결과의
실행 개수를 확인한다. 테스트 필터는 오타여도 성공할 수 있다.

## 검증의 소유권

| 검증 대상 | 주로 두는 곳 |
|-----------|--------------|
| 파싱·분석·런타임 의미론, 오류 복원 | 해당 crate 단위 테스트 |
| CLI 입력/출력과 공개 JSON schema | `crates/orv-cli/tests/*_contract.rs` |
| 산출물 변조를 거부하는 verifier | `crates/orv-cli/src/tests/verify_*.rs` |
| HTTP·DB·LSP/DAP 연결과 프로세스 동작 | 기존 통합·회귀 테스트 |
| 생성된 쇼핑몰과 컨테이너 실제 실행 | `scripts/shop_acceptance_smoke.sh`, `scripts/container_smoke.sh` |

공개 JSON을 전체 golden과 비교한다면 같은 fixture의 key/type/고정값을
수동 assertion으로 다시 열거하지 않는다. 정규화가 가리는 값, 다른 binding
종류, 파일 간 관계, 실제 실행 결과처럼 golden이 보장하지 않는 동작은
별도로 검증한다. 단위 테스트에서 통합 테스트와 같은 빌드를 반복하기 전에
기존 테스트에 필요한 검증을 추가할 수 있는지 확인한다.

CLI 실행·JSON 읽기·key 검사·임시 경로는 `tests/support/mod.rs`를 공유한다.
임시 경로에는 PID, 시간, 프로세스 내 atomic 순번이 들어간다. 새 테스트는
`TestDir`와 `DapServer`를 재사용해 자원을 정리한다. `DapServer`는 내부
`ChildGuard`로 시작 실패 시에도 자식 프로세스를 정리한다. cwd/env가 다른
실행은 해당 기능 모듈에서 명시한다.

## 산출물 오류 사례

같은 입력과 build profile을 쓰는 오류 검증은 `artifact_cases.rs` 도구로
묶는다. 입력은 묶음당 한 번 빌드하고 먼저 정상 검증을 수행한다. 각 사례가
끝나면 기존 산출물의 내용과 권한을 복원한다. assertion 실패도 사례 이름과
함께 모으므로 첫 실패 때문에 뒤의 독립 사례가 누락되지 않는다. 마지막에
정상 검증을 반복하고 fixture 디렉터리를 정리한다.

현재 공유 묶음은 기존 파일의 JSON/텍스트/권한 변경을 대상으로 한다. 파일
추가나 디렉터리 삭제 등 별도 수명주기가 필요한 검증은 독립 fixture를 쓴다.
오류 사례는 구체적인 이름과 기대 오류를 유지하고, 다른 의미론을 검사하는
테스트를 단순히 함수 개수를 줄이기 위해 합치지 않는다.

## 2026-09-09 정리 결과

| 항목 | 정리 전 | 정리 후 |
|------|---------|---------|
| CLI 통합 테스트 실행 파일 | 39 | 1 |
| 동일 fixture를 반복 빌드하던 산출물 오류 검증 | 138개 테스트 | 같은 138개 사례, 13개 테스트 |
| 전체 workspace 테스트 함수 / suite | 1,592 / 55 | 1,465 / 17 |
| CLI 단위·통합 테스트 코드 줄 수 | 49,563 | 46,507 |

125개 테스트 함수 감소는 사례 묶음 전환이고, 나머지 2개는 공개 계약 중복
제거다. 동작별 고유 검증은 유지한다. 전체 테스트를 무시하거나 CI 검사
범위를 줄이지 않았으며, PR의 새 실행이 이전 실행을 취소하도록 했다.
실행 시간은 빌드 캐시와 환경에 좌우되므로 이 표는 속도 향상 비율을 뜻하지
않는다.
