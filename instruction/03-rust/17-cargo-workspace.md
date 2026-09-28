# Cargo Workspaces, Features, and Build Scripts

## What to learn

### The workspace `Cargo.toml`
A `[workspace]` section with `members = ["labs/*", "proxy"]` (this repo's actual shape) makes every listed crate share one `Cargo.lock` and one `target/` directory — a dependency's version is resolved once for the whole workspace, not per crate. This is why adding `tokio` to a new `labs/NN-*` crate doesn't trigger a fresh, independent dependency resolution; it reuses whatever version the rest of the workspace already settled on.

```toml
# repo-root Cargo.toml
[workspace]
members = ["labs/*", "proxy"]
resolver = "2"
```
Gotcha: a *package* on edition 2021 implies `resolver = "2"` (edition 2024 implies `"3"`), but a *virtual* workspace like this repo's root has no edition of its own and silently falls back to resolver 1 unless you set it explicitly — which is why the line above exists. It matters for feature unification below: the v1 resolver unified features across dev-dependencies, build-dependencies, and target-specific dependencies in more surprising ways.

### `[workspace.dependencies]`: one version, declared once
Listing a dependency's version once under `[workspace.dependencies]` and having each member crate reference it with `dep.workspace = true` keeps every crate on the same version without repeating — and inevitably letting drift into — the version string across 18 different `Cargo.toml` files.

```toml
# workspace root
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }

# labs/05-reverse-proxy/Cargo.toml
[dependencies]
tokio = { workspace = true }
```

### Feature flags are additive, and that's load-bearing
Cargo features are unified across the whole *build*, not per crate: if crate A enables `hyper`'s `client` feature and crate B (built in the same workspace build) enables `server`, the compiled `hyper` gets both. This is why removing a feature from one crate's `Cargo.toml` can fail to shrink the binary if some other workspace member still turns it on — and why a library crate should almost never enable a feature "just in case," since it silently becomes mandatory for every consumer built alongside it.

Gotcha: features must be strictly additive — turning one on can never remove functionality. Cargo enforces this by construction, not as a style guideline, precisely because unification would otherwise make build outcomes depend on which other crates happen to be compiled alongside yours.

### `build.rs`: code generation before `rustc` runs
A `build.rs` at a crate's root is compiled and run before the crate itself, and can write files into `OUT_DIR` that the crate's own code then `include!`s. The concrete case this handbook needs it for: `tonic-build`/`prost-build` compiling `.proto` files into Rust structs for gRPC support ([`05-http-stack/10-grpc.md`](../05-http-stack/10-grpc.md)) — the generated client/server code doesn't exist as source you write by hand, it's regenerated on every build from the `.proto` schema.

```rust
// build.rs
fn main() {
    tonic_build::compile_protos("proto/health.proto").unwrap();
}
```
Gotcha: a `build.rs` that does real work (parsing a schema, invoking `protoc`) adds real time to every clean build and every CI run — keep it fast, and prefer checking in generated code for a crate that many downstream consumers build often, if build time becomes painful.

### Conditional compilation with `cfg` and features
`#[cfg(feature = "tls")]` compiles a module in or out entirely based on a feature flag, letting one crate support optional functionality (an optional TLS backend, an optional Prometheus metrics exporter) without forcing every consumer to pull in and compile that dependency.

```toml
[features]
default = []
tls = ["dep:tokio-rustls"]
```
```rust
#[cfg(feature = "tls")]
mod tls_listener;
```
Gotcha: the dependency itself must be declared `tokio-rustls = { version = "...", optional = true }` — `dep:` only works on optional dependencies. Without `optional = true`, every consumer compiles `tokio-rustls` even with the feature off; and without the `dep:` prefix, Cargo also creates an implicit public feature named `tokio-rustls` that leaks the dependency's name into your crate's feature API.

## Practice
1. Add `[workspace.dependencies]` for `tokio` and `bytes` at this repo's workspace root, and migrate two `labs/*` crates to `dep.workspace = true` instead of repeating the version.
2. Add a feature flag to one `labs/*` crate that conditionally compiles an optional module (`#[cfg(feature = "...")]`), and confirm via `cargo build --no-default-features` and `cargo build --features ...` that the module is actually excluded/included.
3. Deliberately observe feature unification: enable a feature only in a dev-dependency of one crate, build the whole workspace, and confirm (via `cargo tree -e features` or similar) that a sibling crate using the same dependency also gets that feature turned on.
4. Wire up `build.rs` with `tonic-build` in whichever crate handles [`05-http-stack/10-grpc.md`](../05-http-stack/10-grpc.md)'s exercise, to compile a `.proto` file, and inspect the generated code under `target/*/build/*/out/`.
5. Time a clean build before and after adding schema compilation to `build.rs`, and decide whether checking in the generated code would be worth it for this workspace's size.
