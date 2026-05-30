# orv Platform Boundary

이 문서는 orv가 **compiler core에 넣는 것**, **compiler plugin으로 제공하는 도메인 추상화**, 그리고 **라이브러리, template, provider package로 밀어내는 것**을 구분한다.

핵심 원칙:

> Compiler core는 표준 이론, 보편 기술 추상화, plugin protocol만 알아야 한다. 도메인별 추상화, 특정 비즈니스 도메인, 외부 API provider, SaaS 제품명은 compiler core intrinsic이 아니다.

쇼핑몰은 north-star benchmark이자 first-party template이다. 쇼핑몰이 compiler architecture를 정의하지 않는다.

## Compiler Core Intrinsic

Compiler core가 직접 소유해도 되는 표면:

| Surface | 이유 |
|---------|------|
| syntax / AST / HIR | 언어의 기본 구조 |
| normalized `DomainCall` | 모든 도메인 플러그인이 공유하는 호출 형태 |
| type / schema / validation | 모든 앱 도메인에 공통 |
| ProjectGraph / origin / reveal | orv의 핵심 차별점 |
| diagnostics | 모든 플러그인이 같은 span/error 모델 사용 |
| compiler plugin registry / hook protocol | 도메인별 추상화를 core 밖으로 빼기 위한 확장점 |
| effect/capability metadata | plugin 결과의 compile-time analysis와 deploy/reveal에 필요 |
| generic capability schemas | HTTP route, HTML tree, DB operation, security policy 같은 표준 IR 계약 |
| `@secret` / env contract / redaction | provider를 모르는 secret plumbing |
| generic adapter bridge | external capability call을 표현하는 표준 boundary |
| build/deploy/preflight/smoke artifact schema | production handoff primitive |

Compiler가 알아야 하는 것은 다음이다.

```text
schema
normalized domain call
plugin hook
capability schema
transaction
secret usage
capability/adapter call
idempotency key
origin/reveal edge
deploy env contract
```

Compiler가 몰라야 하는 것은 다음이다.

```text
@server / @route / @html / @db / @design implementation details
Stripe
carrier-specific shipping
cart business rules
order lifecycle policy
payment capture semantics
shop admin read-model policy
```

## First-Party Compiler Plugins

도메인별 추상화는 compiler core가 아니라 compiler plugin이 제공한다. First-party plugin은 repo 안에 함께 배포될 수 있고 기본 활성화될 수 있지만, architecture상 core intrinsic은 아니다.

| Plugin surface | 제공하는 도메인 예시 | Core가 보는 것 |
|----------------|----------------------|----------------|
| `orv-web` | `@server`, `@route`, `@respond`, `@serve`, `@html`, `@form` | HTTP/HTML capability metadata, origin/reveal edge |
| `orv-data` | `@db`, transaction, migration, adapter declaration | DB operation/transaction capability metadata |
| `orv-security` | `@Auth`, `@session`, `@csrf`, `@rateLimit` | security policy metadata |
| `orv-design` | `@design`, token domains | style/token artifact metadata |
| `orv-commerce` | `@payment`, `@shipping`, checkout helper vocabulary | generic adapter/secret/idempotency metadata |
| advanced plugins | `@gpu`, `@media`, `@sync`, `@net`, `@mail` | plugin-declared capability metadata |

Compiler plugin은 최소한 다음 계약을 노출해야 한다.

- property/token/content schema
- type checking hooks
- HIR/capability lowering output
- diagnostics with source spans
- ProjectGraph/origin/reveal metadata
- build/deploy artifact contributions
- permission and sandbox requirements when needed

## First-Party Library / Template

다음은 compiler core intrinsic이 아니라 first-party package 또는 template surface다.

| Surface | 위치 |
|---------|------|
| shop scaffold | `orv init --template shop` |
| cart/order/checkout/admin read models | `orv-shop` style package/template |
| `@payment` / `@shipping` reference domains | `orv-commerce` style package surface |
| Stripe-style webhook route | provider package/template example |
| carrier booking flow | provider package/template example |
| 5-hour shop benchmark | product benchmark, not language boundary |

현재 repository에는 benchmark를 닫기 위해 commerce reference adapters가 `orv-runtime`/`orv-cli` 안에 들어 있다. 이것은 MVP implementation convenience다. Public architecture에서는 이 표면을 library-like contract로 취급하고, compiler core semantics로 승격하지 않는다.

## Provider Package

Provider package는 generic adapter/secret/deploy/reveal boundary 위에서 작동해야 한다. Provider package가 compile-time 분석을 필요로 하면 compiler plugin을 함께 제공할 수 있지만, 그 경우에도 core는 provider 이름을 특별 취급하지 않고 plugin protocol만 호출한다.

예시:

```orv
import { Checkout } from "orv-commerce"
import { Stripe } from "orv-stripe"

let payments = Stripe.connect(secret=@secret.STRIPE_KEY)

@route POST /checkout {
  @body: CheckoutForm
  @session required
  @csrf

  Checkout.capture(provider=payments, order=@body)
}
```

이때 compiler는 `Stripe`를 특별 취급하지 않는다. Compiler는 provider call이 다음 정보를 노출하는지만 본다.

- input/output schema
- secret/env requirements
- idempotency key shape
- retry/replay policy metadata
- origin id and source span
- deploy artifact entries
- redaction guarantees

## Promotion Rule

새 domain의 기본 승격 대상은 compiler core가 아니라 compiler plugin이다. Core intrinsic 승격은 마지막 수단이다.

새 기능이 compiler core intrinsic으로 들어오려면 다음 조건을 모두 만족해야 한다.

1. 특정 산업/제품이나 도메인 문법이 아니라 여러 compiler plugin이 공유하는 primitive여야 한다.
2. plugin protocol로 표현했을 때 semantic gap이 명확해야 한다.
3. ProjectGraph/origin/reveal/build/deploy 분석에 직접 필요해야 한다.
4. `IMPLEMENTATION_MATRIX.md`에서 contract level을 올릴 수 있는 regression과 fixture가 있어야 한다.

이 조건을 만족하지 못하면 compiler plugin, first-party package, provider package, template, 또는 `ADVANCED_DOMAINS.md`의 non-binding roadmap으로 둔다.

## Documentation Rule

문서에서 commerce/shop 기능을 설명할 때는 다음 용어를 사용한다.

- "shop benchmark"
- "shop template"
- "commerce reference package surface"
- "first-party compiler plugin"
- "domain plugin"
- "provider package"
- "generic adapter boundary"

피해야 할 표현:

- compiler core가 payment/shipping을 intrinsic으로 안다는 표현
- domain별 추상화가 core intrinsic이라는 표현
- Stripe/carrier가 language-level provider라는 표현
- shop checkout이 core language semantics라는 표현

## Current Refactor Direction

단기에는 기존 `@payment`/`@shipping` reference runtime을 유지한다. 동시에 문서와 contract는 이를 `orv-commerce` style library surface로 해석한다.

중기 목표:

- commerce adapter contracts를 generic adapter/secret/idempotency/reveal boundary로 재표현
- web/data/security/design domain을 first-party compiler plugin surface로 문서화
- shop scaffold가 first-party package import처럼 읽히도록 정리
- provider-specific hardening은 `orv-stripe`, `orv-carrier-*` style package layer로 분리
- compiler matrix에서는 commerce를 core platform progress와 별도 계층으로 추적
