# Async Trait Methods and Dyn-Compatibility

## What to learn

### Why `trait Foo { async fn bar(&self); }` doesn't just work as `dyn Foo`
Native `async fn` in traits (stable since Rust 1.75) desugars to a method returning an opaque, compiler-generated `impl Future` type — and that opaque type is different for every implementor. A trait object (`dyn Foo`) needs one fixed vtable layout shared by every implementor, but "a different concrete future type per impl" is exactly what a vtable can't erase away automatically. So `async fn` in a trait compiles fine for static dispatch (`impl Foo`, generics) but the trait is not usable as `dyn Foo` without extra work.

```rust
trait Middleware {
    async fn handle(&self, req: Request) -> Response; // fine for `impl Middleware`, not for `dyn Middleware`
}
```

### The manual fix: return a boxed future explicitly
Instead of `async fn`, write the method to return `Pin<Box<dyn Future<Output = Response> + Send + '_>>` directly, and implement it with an `async move` block wrapped in `Box::pin`. This makes the return type concrete and uniform across every implementor, which is exactly what a vtable needs.

```rust
trait Middleware: Send + Sync {
    fn handle<'a>(&'a self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
}

impl Middleware for LoggingMiddleware {
    fn handle<'a>(&'a self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            tracing::info!("request in");
            self.inner.handle(req).await
        })
    }
}
```
This is exactly [`03-rust/10-smart-pointers-and-interior-mutability.md`](10-smart-pointers-and-interior-mutability.md)'s `Pin<Box<dyn Future>>` shape and [`03-rust/06-pin.md`](06-pin.md)'s reason for `Pin` existing, applied at a trait boundary instead of inside a hand-rolled executor.

### The `async-trait` crate: the same fix, via macro
`#[async_trait]` on a trait (and on each `impl`) rewrites `async fn` methods into exactly the boxed-future shape above, automatically. Reach for it over hand-writing the boxed signature when a trait has several async methods — the manual version's signature is verbose enough that repeating it by hand across five methods is worse than accepting one macro-expanded allocation per call. This connects to [`03-rust/12-macros.md`](12-macros.md): knowing what a derive/attribute macro expands to is what makes reaching for `async-trait` a deliberate choice instead of "the answer someone pasted from Stack Overflow."

### The real cost: one allocation per call
Every call through a boxed-future trait method allocates a `Box` for that call's future, even when the caller only ever uses one concrete implementor and never needed dynamic dispatch at that call site. For a plugin/middleware chain ([`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)) called once per request, this is a real, measurable cost at high request rates, not a rounding error — profile it ([`08-observability/04-profiling.md`](../08-observability/04-profiling.md)) before assuming it's fine, and consider an enum of known middlewares dispatched via `match` (static dispatch) instead of `Vec<Box<dyn Middleware>>` if the plugin set is actually fixed at compile time.

### When you don't need `dyn` at all
If every implementor of a trait is known at compile time (a fixed set of load-balancing strategies chosen by config, not loaded as plugins), a generic function or an enum dispatching via `match` avoids this problem entirely — `async fn` in a trait works fine there, because nothing ever needs a `dyn` trait object. Reach for the boxed-future pattern only when you genuinely need runtime polymorphism (a plugin loaded from config, a `Vec` of heterogeneous handlers) — the same static-vs-dynamic-dispatch judgment call as [`03-rust/07-traits-and-generics.md`](07-traits-and-generics.md), just with async added on top.

## Practice
1. Write a trait with a native `async fn` method, implement it for two types, and confirm `dyn YourTrait` fails to compile with an object-safety error — read the error message closely.
2. Rewrite the same trait by hand to return `Pin<Box<dyn Future<Output = T> + Send + '_>>`, implement it for the same two types, and confirm `Box<dyn YourTrait>` now compiles and works.
3. Redo the same trait with `#[async_trait]` instead, and use `cargo expand` (from [`03-rust/12-macros.md`](12-macros.md)) to compare the macro's generated signature against what you wrote by hand.
4. In [`labs/14-plugin`](../../labs/14-plugin), design the middleware trait both ways — as `Vec<Box<dyn Middleware>>` with boxed futures, and as a fixed enum of known middlewares dispatched via `match` — and benchmark ([`03-rust/16-testing-idioms.md`](16-testing-idioms.md)'s `criterion` section) one allocation-heavy call path against the other.
5. Decide, and write down, whether [`06-proxy/02-load-balancer.md`](../06-proxy/02-load-balancer.md)'s strategy selection in [`labs/06-load-balancer`](../../labs/06-load-balancer) actually needs `dyn LoadBalancer` (runtime-configurable) or would be just as correct, and faster, as a compile-time enum — justify your answer with what the config format actually allows.
