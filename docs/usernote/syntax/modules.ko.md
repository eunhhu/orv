# 모듈 & 임포트

[← 목차로 돌아가기](./index.ko.md)

---

## 임포트 구문

orv는 Python과 Rust에서 영감을 받은 점 경로 임포트를 사용합니다:

```orv
// Single import
import libs.counter.myFunc

// Multiple imports from the same module
import components.{Button, Input, Card}

// Aliased import
import libs.http.Client as HttpClient

// Wildcard (use sparingly)
import utils.*

// Standard library
import @std.io
import @std.collections.{Vec, HashMap}

// External packages
import @pkg.jwt
import @pkg.database.postgres
```

## 모듈 구조

각 `.orv` 파일은 하나의 모듈입니다. 파일 경로가 임포트 경로에 직접 매핑됩니다:

```
project/
├── main.orv              // entry point
├── components/
│   ├── Button.orv        // import components.Button
│   ├── Input.orv         // import components.Input
│   └── Card.orv          // import components.Card
├── libs/
│   ├── counter.orv       // import libs.counter
│   └── http.orv          // import libs.http
└── pages/
    └── Home.orv          // import pages.Home
```

## 내보내기

최상위 선언은 기본적으로 비공개입니다. `pub`을 사용하여 내보냅니다:

```orv
pub struct User {
  name: string
  age: i32
}

pub function greet(name: string): string -> "Hello, {name}"

pub define Button(label: string) -> @button label rounded-md

// Private — only accessible within this module
function internalHelper(): void -> { ... }
```
