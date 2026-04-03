# Modules & Imports

[← Back to Index](./index.md)

---

## Import Syntax

orv uses dot-path imports inspired by Python and Rust:

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

## Module Structure

Each `.orv` file is a module. The file path maps directly to the import path:

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

## Exports

Top-level declarations are private by default. Use `pub` to export:

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
