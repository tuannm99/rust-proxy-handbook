# Traits and Generics

## What to learn

### Static dispatch (generics) vs dynamic dispatch (trait objects)
A generic function is monomorphized — the compiler generates one specialized copy per concrete type used at each call site, at zero runtime cost and fully inlinable, at the price of code bloat (binary size grows with the number of instantiations). A `dyn Trait` call goes through a vtable — one compiled copy regardless of how many concrete types implement the trait, but the call can't be inlined across the vtable indirection, and the trait must be object-safe.

```rust
trait LoadBalancer {
    fn pick(&self, key: &str) -> usize;
}

fn route_generic<T: LoadBalancer>(lb: &T, key: &str) -> usize { lb.pick(key) } // monomorphized per T
fn route_dyn(lb: &dyn LoadBalancer, key: &str) -> usize { lb.pick(key) }       // one vtable call
```
Gotcha: a plugin system ([`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md)) or a load-balancing strategy picked from config at startup almost always needs `Box<dyn Trait>` — the concrete type isn't known until runtime, so generics can't express the choice at all.

### Trait bounds and where clauses
`T: Send + Sync + 'static` shows up constantly around `tokio::spawn`: a spawned future must be `Send` to move across worker threads, and `'static` because the task can outlive the stack frame that spawned it. `where` clauses exist purely for readability once bounds get long.

```rust
fn spawn_handler<F>(fut: F) where F: std::future::Future<Output = ()> + Send + 'static {
    tokio::spawn(fut);
}
```
Gotcha: an async fn that holds a non-`Send` value (an `Rc`, a `MutexGuard` from `std::sync::Mutex` held across an `.await`) across a suspension point produces a future that isn't `Send`, and the error only surfaces at the `tokio::spawn` call site — often far from the actual cause. Connect this to [`03-rust/05-async.md`](05-async.md)'s desugaring: the generated future's fields are exactly whatever's alive across each `.await`, so one non-`Send` value anywhere in that set taints the whole future.

### Associated types vs generic parameters
Prefer an associated type (`Iterator::Item`) when there's exactly one sensible output type per implementor; prefer a generic parameter when a type legitimately implements the trait multiple ways for different type arguments (`From<T>` for several `T`).

```rust
trait UpstreamPool {
    type Conn;
    fn acquire(&self) -> Self::Conn;
}
```

### Blanket impls and the orphan rule
The orphan rule says you may only `impl` a trait for a type if you own the trait or the type — this prevents two crates from writing conflicting impls for the same foreign type. The practical workaround for "adding methods" to a type you don't own is the extension-trait pattern: define your own trait, blanket-impl it for every type satisfying some bound you *can* reference.

```rust
trait ResponseExt {
    fn is_upstream_error(&self) -> bool;
}
impl ResponseExt for http::Response<hyper::body::Incoming> {
    fn is_upstream_error(&self) -> bool { self.status().is_server_error() }
}
```

### Object safety
Not every trait can be `dyn`-dispatched: a method with a generic type parameter, or one returning `Self` by value, breaks object safety because the vtable can't be built without knowing the concrete type at the call site. `dyn Trait`'s methods are restricted to `&self`/`&mut self`/`Box<Self>` receivers with no generics — this is a compile-time check, not a style preference.

## Practice
1. Write both a generic and a `dyn Trait` version of a `pick(&self, key: &str) -> usize` load-balancer function, build both in release mode, and compare binary size (`cargo bloat` or plain `size`) to see monomorphization's cost directly.
2. Deliberately write an async fn that holds an `Rc<RefCell<_>>` across an `.await`, try to `tokio::spawn` it, and read the resulting compiler error closely enough to name exactly which bound failed.
3. In [`labs/06-load-balancer`](../../labs/06-load-balancer), define a `LoadBalancer` trait and implement it for round-robin, least-conn, and consistent-hash strategies; select the concrete strategy at runtime from a config string via `Box<dyn LoadBalancer>`.
4. Write an extension trait for a type you don't own (e.g. `http::HeaderMap`) and explain why the orphan rule would have blocked implementing a foreign trait directly on it instead.
5. Add a generic method to a trait and try to use it as `dyn Trait`; read the object-safety error and identify exactly which rule it violates.
