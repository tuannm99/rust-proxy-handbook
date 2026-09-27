# API Design and Modules

## What to learn

### The module system as an API boundary
`mod`/`pub`/`pub(crate)`/`pub(super)` control not just organization but what's actually part of your contract. A crate's `pub` surface is what downstream code can break by relying on; everything else can be refactored freely. Default to the least-visible option and widen only when something outside the module genuinely needs it — walking visibility back later is a breaking change.

```rust
pub mod pool {
    pub struct ConnectionPool { conns: Vec<Conn> } // fields private by default, even in a pub struct
    impl ConnectionPool {
        pub fn acquire(&mut self) -> Option<Conn> { self.conns.pop() } // the real API
    }
}
```

### The newtype pattern
Wrapping a primitive or foreign type in a single-field struct (`struct RequestId(u64);`) buys three things: a distinct type the compiler won't let you accidentally mix with a plain `u64` (a classic bug — passing a connection-count where a port number was expected, both `usize`), the ability to implement traits you don't own for a type you don't own (sidestepping the orphan rule from `03-rust/07-traits-and-generics.md`), and a place to enforce invariants in a constructor while keeping the inner value private.

```rust
pub struct UpstreamAddr(std::net::SocketAddr); // distinct from a raw SocketAddr used for the client's own address
```

### The builder pattern
For a type with many optional fields (per-route config, a client builder), a builder avoids both a ten-positional-argument constructor (unreadable, easy to swap two args of the same type by accident) and a struct-literal-with-`pub`-fields approach (no validation point). Each builder method takes/returns `self`, and a final `.build()` validates and produces the real type.

```rust
RouteConfig::builder()
    .path("/api")
    .upstream_pool(pool)
    .timeout(std::time::Duration::from_secs(5))
    .build()?; // validates, e.g. rejects a route with no upstream configured
```

### Sealed traits
A "sealed" trait is public enough to use as a bound, but carries a private supertrait (or lives behind an unreachable module) that stops anyone outside your crate from implementing it. This lets you add methods to the trait later without it being a breaking change, since no external implementation exists that could conflict.

```rust
mod private { pub trait Sealed {} }
pub trait Middleware: private::Sealed { fn handle(&self, req: Request) -> Response; }
```

### semver and API stability discipline
Adding a `pub` field, a new variant to a `pub` enum without `#[non_exhaustive]`, or a new required trait method are all breaking changes under semver even though they only "add" something — existing code that exhaustively matches or implements against the old shape stops compiling. `#[non_exhaustive]` on a struct/enum you expect to grow lets you add fields/variants without that break, at the cost of callers never being able to construct or exhaustively match it directly.

## Practice
1. Take a struct in `labs/06-load-balancer` with several public fields and refactor it to keep fields private behind a builder with a validating `.build()`.
2. Introduce a newtype wrapper around a raw `SocketAddr` or `u64` id somewhere it's currently a bare primitive, and find (via compiler errors) every place that was relying on it being interchangeable with the raw type.
3. Mark an enum you own with `#[non_exhaustive]`, add a variant, and confirm existing exhaustive `match`es outside the defining module now fail to compile — then fix them with a wildcard arm.
4. Write a sealed trait for a small `Middleware`-style abstraction, and confirm from a separate module that outside code cannot implement it.
5. Pick one `pub` item in a `labs/` crate and write down what changing its shape would break for a hypothetical downstream user — decide whether it should really be `pub`.
