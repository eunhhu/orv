# orv 언어 레퍼런스

[← 사용자 문서로 돌아가기](./README.ko.md)

> 이 문서는 모듈화되었습니다. 전체 명세는 [`syntax/index.md`](./syntax/index.md)를 참조하세요.

## 이 페이지의 용도

- 언어 레퍼런스의 전체 구조 파악
- 모듈형 구문 문서로의 빠른 탐색
- [`syntax/index.md`](./syntax/index.md)의 정식 레퍼런스 인덱스 진입

## 빠른 탐색

| 문서 | 내용 |
|------|------|
| [인덱스 & 철학](./syntax/index.md) | 설계 원칙, 목차 |
| [기초](./syntax/fundamentals.md) | `@domain` head 문법, node/property 구문, 타입, 변수 |
| [함수 & 제어 흐름](./syntax/functions.md) | 함수, 클로저, 파이프, if/for/while, 패턴 매칭 |
| [컬렉션, 에러 & 비동기](./syntax/collections.md) | Vec, HashMap, try/catch, async/await |
| [모듈 & 임포트](./syntax/modules.md) | 임포트 경로, 파일 구조, 익스포트 |
| [Node 시스템 & 반응성](./syntax/nodes.md) | `@`/`%` 시스템, subtoken, head data, `@io`, `@env`, 시그널 |
| [UI & 디자인 도메인](./syntax/ui.md) | HTML, Tailwind, 이벤트, 라이프사이클, 디자인 토큰 |
| [서버 도메인](./syntax/server.md) | 라우트, 미들웨어, RPC, 도메인 유효성 검사 |
| [커스텀 Node -- `define`](./syntax/define.md) | 커스텀 도메인, subtoken 계약, head data, `@children` slot projection |
| [컴파일러 힌트](./syntax/hints.md) | 프로토콜, 렌더, 캐시, 청킹을 위한 `@hint` 오버라이드 |
| [모범 사례 & 예제](./syntax/best-practices.md) | 가이드라인, 전체 Todo 앱 예제 |
