# Plugin System

## What to learn
### The core trade-off: compile-time vs runtime extensibility
A "plugin system" can mean anything from "compose a fixed set of `tower::Layer`s at compile time" to "load arbitrary untrusted code at runtime" (dylibs, Lua, WASM). More flexibility at runtime means more risk (ABI mismatches, crashes taking down the whole proxy, security exposure) and worse performance (dynamic dispatch, serialization boundaries). Nginx/Envoy/HAProxy all lean toward compiled or sandboxed modules rather than arbitrary dynamic code, for exactly this reason.

### Why most Rust proxies choose compile-time composition
Rust's `tower::Service`/`Layer` traits let you compose request-handling behavior (auth, rate limit, WAF, logging — see `09-architecture/components.md`) as generic, statically-dispatched types. The compiler inlines and monomorphizes the whole stack, so there's no runtime cost for "plugins," and a bad module is a compile error or a contained panic, not an ABI crash. The cost: adding/removing a module requires a rebuild, not a hot-swappable artifact.

```rust
trait Module: Send + Sync {
    fn name(&self) -> &'static str;
    async fn handle(&self, req: Request, next: Next) -> Response;
}
```

Gotcha: monomorphization has costs that show up at scale rather than in a
benchmark. A deeply nested `ServiceBuilder` stack produces enormous
concrete types — compile times climb, error messages become unreadable,
and the generated future can get large enough that moving it around
matters. `BoxCloneService` at one or two layer boundaries erases the type
and usually costs less than it saves.

### The tension: configured order requires dynamic dispatch
Here is the practical fork in the road, and it decides your design.

A compile-time tower stack has its order baked into the type at build
time. But a real proxy typically wants the module set to be *per route*
and *configurable* (`09-architecture/config.md`): this route needs auth
and WAF, that one needs neither, and an operator changes it without a
rebuild. You cannot express "order comes from a TOML file" in a
monomorphized type.

So the honest answer for a configurable proxy is a `Vec<Arc<dyn Module>>`
built at config-load time — dynamic dispatch, one virtual call per module
per request, resolved once when config is parsed rather than per request.
That cost is small and predictable; the mistake is pretending you can have
config-driven ordering *and* full monomorphization.

Gotcha: `async fn` in a trait that you need as `dyn Module` is the wrinkle.
Native `async fn` in traits is not object-safe in the general case, so
object-safe plugin traits either use `#[async_trait]` (which boxes the
returned future — one allocation per module per request) or return an
explicit `Pin<Box<dyn Future>>`, which is the same cost written out by
hand. Budget for it, and keep the module count per route modest.

### When dynamic plugins are worth it anyway
If you need third parties (or non-Rust teams) to ship logic without rebuilding your proxy, two realistic options:
- **WASM** (via `wasmtime`): sandboxed, memory-safe even if the plugin is buggy or hostile, but you pay a serialization/host-call boundary cost per request and the API surface plugins can call is limited to what you expose.
- **Dynamic libraries** (`libloading` + a stable `extern "C"` ABI, or `abi_stable`): near-native speed, but a crash or ABI drift in the plugin can take down the whole process — Rust's `dyn Trait`/generics don't have a stable ABI across compiler versions, so you're restricted to a C-shaped interface.
Envoy's WASM filters are the reference design if you want to see this done for a real proxy.

### Bounding a plugin that misbehaves
Memory safety is the easy half of sandboxing. The harder half is that a
plugin running on your request path can simply *not return* — an infinite
loop in a WASM guest hangs the worker thread exactly like any other
blocking operation (`03-rust/async.md`), taking every connection
multiplexed on it down with the request that triggered it.

`wasmtime` provides two mechanisms for this, and you need one of them:
- **Fuel**: the guest is charged per operation and traps when it runs out.
  Deterministic and reproducible, at a small per-instruction cost.
- **Epoch interruption**: a background thread bumps an epoch counter and
  the guest traps at the next check. Cheaper at runtime, wall-clock based
  rather than deterministic.

Also bound the guest's memory (`StoreLimits`), and decide what a trap
means — which is the fail-open/fail-closed decision below, now with a
hostile-plugin flavor.

Gotcha: instance lifecycle is a real performance decision. Creating a
fresh `Instance` per request is the cleanest isolation and the most
expensive; pooling instances (wasmtime's pooling allocator) is much faster
but means state can leak between requests unless you reset it — the same
discipline as `14-memory/object-pool.md`, with a security consequence if
you get it wrong.

### Data and control flow across a plugin boundary
Whatever mechanism you pick, decide explicitly: can a plugin see the full request/response body, or just headers? Can it short-circuit (return a response without calling upstream)? Can it fail open (pass through) or must it fail closed (reject) on plugin error? These are security-relevant decisions (see `07-security/waf.md`), not just architecture ones.

Two more that bite later if left implicit:
- **Can a plugin mutate what earlier modules established?** If a plugin
  can rewrite the identity header that auth set
  (`07-security/auth.md`), it can escalate privilege. Expose a *read-only*
  view of trusted context and a separate, restricted channel for the
  mutations you actually intend to allow.
- **Body access forces buffering.** Granting body access silently
  disables streaming for every route that uses that plugin
  (`05-http-stack/grpc.md`, `05-http-stack/websocket.md`), with all of
  `07-security/waf.md`'s size-limit consequences. Make it opt-in per
  plugin and visible in config, not a capability everything gets.

Gotcha: a plugin on the request path is part of your latency budget, and
a third-party plugin is latency you don't control. Time each plugin
separately as its own metric (`08-observability/metrics.md`) and span
(`08-observability/tracing.md`) — "the proxy got slow" should be
attributable to a specific plugin without bisecting config.

## Practice
Build these in order.

1. In `labs/14-plugin`, define a `Module` trait and implement two modules
   (logging + rate limiting) behind it. **Done when** each is unit-tested
   in isolation and neither knows the other exists.
2. Build the stack from config rather than from a type. **Done when**
   changing module order in a TOML file changes execution order with no
   rebuild, and the ordering rules from
   `09-architecture/components.md` are validated at load time (rejecting a
   config that puts auth after WAF).
3. Measure dynamic dispatch's cost. **Done when** you have per-request
   overhead numbers for a monomorphized stack vs `Vec<Arc<dyn Module>>`
   with `#[async_trait]` — and can say whether it matters at your target
   request rate.
4. Define and document fail-open vs fail-closed per module. **Done when**
   a panicking WAF module rejects the request and a panicking metrics
   module doesn't — verified by tests that deliberately panic each.
5. Give plugins a read-only view of trusted context. **Done when** a
   plugin attempting to overwrite the auth-established identity header
   cannot, and a test proves it.
6. Add per-plugin timing metrics and spans. **Done when** a deliberately
   slow plugin is identifiable by name from the dashboard alone.
7. (Stretch) Load one module as a WASM guest with `wasmtime` — start with
   header mutation. **Done when** it works end-to-end and you have
   measured the added per-request latency against the compiled equivalent.
8. Add fuel or epoch interruption plus memory limits to the WASM host.
   **Done when** a guest containing `loop {}` traps and returns your
   configured failure response, while other concurrent requests show no
   latency impact — this is the test that proves the sandbox is real.
9. Compare the plumbing. **Done when** you can state how many lines of
   host functions and serialization the WASM version needed versus the
   compiled `Module`, for identical behavior.
