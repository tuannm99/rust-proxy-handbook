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

### When dynamic plugins are worth it anyway
If you need third parties (or non-Rust teams) to ship logic without rebuilding your proxy, two realistic options:
- **WASM** (via `wasmtime`): sandboxed, memory-safe even if the plugin is buggy or hostile, but you pay a serialization/host-call boundary cost per request and the API surface plugins can call is limited to what you expose.
- **Dynamic libraries** (`libloading` + a stable `extern "C"` ABI, or `abi_stable`): near-native speed, but a crash or ABI drift in the plugin can take down the whole process — Rust's `dyn Trait`/generics don't have a stable ABI across compiler versions, so you're restricted to a C-shaped interface.
Envoy's WASM filters are the reference design if you want to see this done for a real proxy.

### Data and control flow across a plugin boundary
Whatever mechanism you pick, decide explicitly: can a plugin see the full request/response body, or just headers? Can it short-circuit (return a response without calling upstream)? Can it fail open (pass through) or must it fail closed (reject) on plugin error? These are security-relevant decisions (see `07-security/waf.md`), not just architecture ones.

## Practice
1. In `proxy`, define a `Module` trait (or reuse `tower::Layer`) and implement two modules behind it (e.g. logging + rate limiting) to confirm the abstraction doesn't leak implementation details between them.
2. Decide and document your fail-open/fail-closed policy per module (a WAF module should probably fail closed; a metrics module should fail open).
3. As a stretch exercise, prototype loading one simple module as a WASM guest with `wasmtime` (e.g. a request-header-mutation plugin) and measure the added per-request latency vs the compiled-in equivalent.
4. Compare: how many lines of "plumbing" does the WASM version need (host functions, serialization) vs the compiled `Module` trait version, for the same behavior?
