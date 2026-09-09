# 실제 사용자 쇼핑몰 benchmark 실행 안내

상태: **실측 대기**. 2026-09-09 현재 기존 참가자 실행 기록은 없다.
자동 테스트 결과를 사람의 작업 시간이나 성공 기록으로 사용하지 않는다.
판정 규칙과 전체 기록 필드는 [BENCHMARK_SHOP_5H.md](BENCHMARK_SHOP_5H.md)를 따른다.

## 진행자 준비

1. 비개발자 최소 2명, 가능하면 3명을 모집한다. HTML/CSS/JS 경험은
   0~1년이며 전문 backend/DB/배포/결제 연동 경험이 없어야 한다.
   한 명으로 하는 pilot은 최소 cohort 조건을 충족하지 않는다.
2. 동일한 도구 버전과 문서를 준비하고 저장소의
   `scripts/shop_acceptance_smoke.sh`를 먼저 실행한다. 참가자별로 빈 작업
   디렉터리를 제공한다. 이미 완성된 쇼핑몰 소스를 참가자 결과로 배정하지 않는다.
3. 참가자는 공식 문서와 내장 도움말을 사용할 수 있다. 실행 중 AI의
   코드 작성·수정·설명 도움은 허용하지 않는다. 진행자가 도와준 내용도 기록한다.
4. 참가자별 익명 ID, run ID, 실제 UTC 시작/종료 시각, 작업별 시간,
   오류와 수정 과정, 문서 조회, 수동 설정 변경, 막힌 이유를 원본 노트에
   기록한다. 노트와 로그의 원본은 재빌드로 덮어쓰지 않는 별도 폴더에 둔다.

## 참가자 작업

[README의 설치·실행 절차](../README.md)를 제공하고 다음을 직접 수행하게 한다.

- 새 shop 생성, 첫 실행, 홈 문구와 테마 변경
- 상품 3개 등록, 상품 필드 추가 및 catalog/admin 표시, validation 변경
- 회원 가입·로그인, 장바구니, checkout, mock 결제와 배송
- admin의 주문·결제·배송 확인
- production build, env check, 생성된 smoke 실행
- route/HTML/DB 실행 결과에서 origin을 통해 원본 소스 찾기

전체 제한은 참가자당 300분이다. benchmark 문서의 작업별 시간은
병목을 관찰하는 지표이며 전체 제한을 늘리지 않는다. 실패·중단·AI 도움도
발생한 그대로 기록한다.

## 실행 후 증거 정리

최종 소스의 production build와 smoke가 끝난 뒤 그 build에서 실행한다.

```sh
orv verify-build dist
orv benchmark-prepare dist --participants 2 > benchmark-prepare.json
orv benchmark-report dist > benchmark-report.json
```

`benchmark-prepare.json`의 `recording_handoff`가 작성할 필드와 파일을
안내한다. 빈 양식의 `benchmark-report.json`이 `incomplete`인 것은 정상이다.
3명을 진행했다면 `--participants 3`을 사용한다.

원본 노트를 참가자별 `dist/deploy/evidence/` 파일로 보존하고
`dist/deploy/benchmark-evidence.json`에 실제 결과를 옮긴다. 각 노트의
participant/run ID는 해당 JSON 행과 일치해야 하며 생성된 안내 문구와
placeholder는 실제 관찰 내용으로 교체한다. 노트를 완성한 뒤 파일의
SHA-256을 계산해 `raw_notes_sha256`에 기록한다. 참가자별 소스와 build,
원본 smoke 출력도 별도 archive에 남기고 최종 보고서의 노트에서 연결한다.
서로 다른 실행의 시간이나 출력이 어느 참가자 것인지 섞이지 않게 한다.

작업별 시간·관찰값·실패 분류와 각 참가자 행을 채우고, 실제로 원본 노트,
smoke, ID, AI 미사용 주장을 확인한 검토자가 `human_evidence_review`를
작성한다. 시각·성공 여부·확인 체크를 예시 값으로 채우지 않는다.
`preflight` hash와 생성된 실행/배포 코드는 그대로 유지한다. 증거 JSON과
참가자 노트 작성은 benchmark 기록 절차에 해당한다.

```sh
orv verify-build dist
orv benchmark-report dist --require-pass > benchmark-report.json
```

최종 보고서, 전체 build 디렉터리, 참가자별 원본 노트·소스·로그를 함께
보존한다. 보고서에 절대 build 경로가 들어가므로 검증한 경로도 기록한다.
수정 후 재빌드가 필요하면 먼저 증거를 archive하고, 새 산출물의
preflight에 맞춰 smoke와 증거 연결을 다시 확인한다. 실패나 미완료는
보고서에 남기고 다음 구현·문서 개선 항목으로 전환한다.
