# hyper 1.x: How the Pieces Fit

Every lab from [`labs/02-http-server`](../../labs/02-http-server) on, and [`proxy/`](../../proxy), is built on `hyper`
1.x. hyper 1.0 is deliberately small: it implements the HTTP/1 and HTTP/2
state machines and nothing else, so a working server is assembled from
five or six crates. This file is the map of those crates and of the
behaviors that matter to a proxy, several of which surprise people. It
names the types and methods to look for. docs.rs has the exact
signatures, and this file tells you which ones you need and why.
Everything here was checked against hyper 1.11 and hyper-util 0.1.20.

## What to learn

### The crate map
| Crate | What it gives you |
|---|---|
| `http` | Plain data types: `Request`, `Response`, `HeaderMap`, `Method`, `StatusCode`, `Uri`, `Version`. No I/O. |
| `http-body` | The `Body` trait: a body is something you *poll* for frames. |
| `http-body-util` | Ready-made bodies and helpers: `Full`, `Empty`, `StreamBody`, `Limited`, `BodyExt` (`.collect()`, `.boxed()`, `.map_err()`). |
| `bytes` | `Bytes`: a reference-counted byte buffer. Cloning or slicing it is O(1) and never copies. |
| `hyper` | The protocol engines: `server::conn::http1`/`http2`, `client::conn::http1`/`http2`, `body::Incoming`, `service::service_fn`. |
| `hyper-util` | Glue to tokio and conveniences: `rt::{TokioIo, TokioExecutor, TokioTimer}`, `server::conn::auto` (HTTP/1 + HTTP/2 on one port), `server::graceful`, `client::legacy::Client` (a pooled client). |
| `h2` | The HTTP/2 implementation hyper uses internally. You configure it through hyper, never call it directly. |

If a name doesn't resolve, check the Cargo features first. hyper and
hyper-util put almost everything behind features (`server`, `client`,
`http1`, `http2`, `server-auto`, `client-legacy`, `server-graceful`,
`tokio`), and a missing feature shows up as "no item named ... in module".

### One connection is one future you drive
hyper has no accept loop and no listener type. You own the loop: accept a
`TcpStream` from tokio, hand it to a connection builder together with a
*service*, and get back a future that runs the entire connection. That
means every request on it, keep-alive, and the protocol state. You
`tokio::spawn` that future, one per connection, exactly like the echo
server in [`labs/00-tcp-server`](../../labs/00-tcp-server). The future finishes when the connection
closes. An `Err` from it means the connection ended badly (client reset,
parse error, timeout). Log it at debug level. It is normal traffic, not a
server bug.

The builder doesn't take a `TcpStream` directly. hyper defines its own I/O
traits (`hyper::rt::Read`/`Write`) so that it doesn't depend on tokio, and
`hyper_util::rt::TokioIo::new(stream)` adapts a tokio stream to them. The
error "the trait `hyper::rt::io::Read` is not implemented for `TcpStream`"
always means a missing `TokioIo` wrapper. The same wrapper goes around a
`tokio_rustls` TLS stream in [`labs/07-tls`](../../labs/07-tls).

### A service turns a request into a future of a response
The service is called once per request, including many times on one
keep-alive connection. `hyper::service::service_fn` turns an async
closure into one. It receives `Request<hyper::body::Incoming>` and returns
a future of `Result<Response<B>, E>`. It must be `'static`, so shared state
(route table, upstream pool, config) goes in an `Arc` that the closure
clones.

The part that trips everyone: **`Err` does not mean "send a 500".** It
means "this connection is broken, abort it". On HTTP/1.1 hyper closes the
connection without writing any response. On HTTP/2 it resets that stream.
So every application failure (handler error, upstream down, bad input)
has to become a `Response` with the right status *inside* your service.
Many servers make the error type `std::convert::Infallible` so the
compiler forces this. That is what [`labs/02-http-server`](../../labs/02-http-server)'s "an error
produces a `500`, not a dropped connection" item tests.

A **panic** inside the service is worse. It unwinds the spawned
connection task, so the connection vanishes with no response and, unless
you observe the task's `JoinHandle`, no log line. Turning a panic into a
`500` needs an explicit panic boundary, see [`09-architecture/02-plugin.md`](../09-architecture/02-plugin.md).

### Bodies are streams of frames, pulled on demand
A body is not a `Vec<u8>`. It implements `http_body::Body`, whose one core
method `poll_frame` yields frames one at a time: data frames (`Bytes`) and
at most one trailers frame (a `HeaderMap`, used by gRPC and by chunked
trailers). Two consequences shape every lab:

- **The request body is not read for you.** `Incoming` only reads from the
  socket as you poll it. `BodyExt::collect()` reads all of it into memory,
  and on attacker-controlled input that needs a cap: wrap it in
  `http_body_util::Limited` first, or a client sends 10 GB into your RAM.
- **Backpressure is automatic if you stream.** hyper polls your response
  body only when it can write to the socket. A slow client means hyper
  stops polling, which means your file read or upstream read stops too.
  That is the whole mechanism behind "1 GB download with flat memory" in
  [`labs/04-static-server`](../../labs/04-static-server) and [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy). You lose it the moment you
  `collect()` a body "just to look at it". A proxy can pass an `Incoming`
  it received straight in as the body of the request it sends upstream,
  and the bytes stream through without ever being held whole.

Response body types, since one handler must return one concrete type:
`Full<Bytes>` for a small in-memory body, `Empty<Bytes>` for none,
`StreamBody` over a `Stream` of `Result<Frame<Bytes>, E>` for streaming,
for example a file through `tokio_util::io::ReaderStream`. When different
branches produce different body types, `.boxed()` erases them all into
one `BoxBody<Bytes, E>` (one allocation per response, which is fine).

### Headers hyper writes for you
hyper computes framing itself. If the body knows its exact length (`Full`
does, through `size_hint`), hyper sends `Content-Length`. If it doesn't,
HTTP/1.1 responses go out chunked. It also adds `Date`, and it honors
`Connection: close` on either side by closing after the response. For
`HEAD` requests, and for `204`/`304` responses, hyper never writes a body
to the wire, whatever your body contains. For `HEAD` you should still
avoid *producing* one (don't open and stream a 1 GB file just so hyper can
drop it), and `Content-Length` should still be the `GET` value, which is
what [`labs/04-static-server`](../../labs/04-static-server)'s `HEAD` item checks.

Gotcha: in a proxy, don't copy the upstream's `Content-Length` and
`Transfer-Encoding` onto your response by hand. They describe the
upstream hop's framing, not yours. See
[`05-http-stack/03-hop-by-hop-headers.md`](03-hop-by-hop-headers.md): strip framing headers and let hyper
regenerate them from the body you actually send.

### One port for HTTP/1.1 and HTTP/2
`hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())`
serves both. It looks at the first bytes of the connection: an HTTP/2
client always opens with the fixed 24-byte preface
`PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`, and anything else is treated as
HTTP/1.1. That is why cleartext HTTP/2 works with
`curl --http2-prior-knowledge`, where curl sends the preface immediately.
Plain `curl --http2` over cleartext instead asks for an HTTP/1.1
`Upgrade: h2c`, which hyper doesn't implement, so it stays on HTTP/1.1.
Over TLS the protocol is chosen by ALPN ([`01-network/19-tls.md`](../01-network/19-tls.md)), and the
client then sends the preface, so the auto builder still does the right
thing. The executor is required because HTTP/2 runs background tasks per
connection.

`.http1()` and `.http2()` on the builder return sub-builders for
protocol-specific settings. `serve_connection_with_upgrades` instead of
`serve_connection` is needed for anything using HTTP/1.1 `Upgrade`,
such as WebSocket ([`05-http-stack/10-websocket.md`](10-websocket.md)).

### Timeouts: nearly all of them are yours
- **HTTP/1 header read timeout.** `http1().header_read_timeout(d)` closes a
  connection that doesn't finish sending request headers within `d`. It
  only works once the builder has a timer: `http1().timer(TokioTimer::new())`.
  Setting the timeout without a timer panics. The clock starts whenever
  the connection waits for a new request head, *including* the idle gap
  between keep-alive requests. So in hyper 1.x this setting is also your
  HTTP/1 idle keep-alive timeout. Re-check this if you upgrade hyper, since
  it is behavior, not a documented contract.
- **HTTP/2 liveness.** `http2().keep_alive_interval(d)` sends PINGs and
  `keep_alive_timeout(d)` closes the connection if they go unanswered
  (also needs a timer). This detects dead peers. It does not close idle
  but healthy connections.
- **Everything else** has no timeout: the handler, reading the request
  body, writing the response, the upstream call. Wrap them with
  `tokio::time::timeout` yourself. An upstream call that times out is a
  `504` ([`01-network/15-http.md`](../01-network/15-http.md)). Per-read deadlines for slow senders are in
  [`07-security/10-slowloris.md`](../07-security/10-slowloris.md).

### HTTP/2 settings you will touch
On `http2()`: `max_concurrent_streams` (the per-connection concurrency
bound from [`01-network/17-http2.md`](../01-network/17-http2.md)), `initial_stream_window_size` and
`initial_connection_window_size` (flow-control windows), `adaptive_window`
(let h2 size windows from measured bandwidth), `max_header_list_size`
(decoded header cap, your HPACK-bomb limit), `max_send_buf_size`, and
`max_pending_accept_reset_streams` plus `max_local_error_reset_streams`
(h2's built-in Rapid Reset limits: a client that resets streams faster
than this gets its connection closed). These are the knobs
[`labs/08-http2`](../../labs/08-http2) exercises.

### Graceful shutdown
The connection future has a `graceful_shutdown()` method. It takes
`Pin<&mut Self>`, so pin the future and keep polling it after the call.
On HTTP/1.1 it finishes the in-flight response and then closes. On HTTP/2
it sends `GOAWAY`, lets open streams complete, and refuses new ones. To
drain *all* connections, `hyper_util::server::graceful::GracefulShutdown`
(feature `server-graceful`) wraps each connection with `.watch(conn)`, and
`.shutdown().await` triggers them all and waits. The surrounding process
design (stop accepting, drain deadline, force-close) is in
[`09-architecture/04-graceful-shutdown.md`](../09-architecture/04-graceful-shutdown.md).

### The client side (for proxies)
`hyper_util::client::legacy::Client::builder(TokioExecutor::new())
.build(connector)` gives a pooled client. The connector is `HttpConnector`,
or a TLS connector wrapping it. What a proxy needs to know:

- **The URI must be absolute.** The client picks the pool and the TCP
  destination from the URI's scheme and authority. A request you received
  as a server has an origin-form URI (`/path?q`), so forwarding it
  unchanged fails with `UserAbsoluteUriRequired`. Build the upstream URI
  from the chosen upstream's address plus the original path and query,
  and set `Host` per your policy.
- **Pool settings.** `pool_idle_timeout` (keep it below the upstream's own
  idle timeout, see [`05-http-stack/05-keepalive.md`](05-keepalive.md)) and
  `pool_max_idle_per_host`.
- **Errors say where they happened.** The client's error type has
  `is_connect()`: true means the TCP/TLS connection never came up, so the
  request was never sent and retrying on another upstream is safe even
  for `POST`. That maps to `502`, and to the retry rules in
  [`06-proxy/05-retry.md`](../06-proxy/05-retry.md). `HttpConnector::set_connect_timeout` bounds just
  the connect step. The overall deadline is your `tokio::time::timeout`.
- **Owning the pool yourself.** `hyper::client::conn::http1::handshake`
  gives one raw client connection with no pool. That is the building block
  if you implement the pool in [`06-proxy/01-upstream.md`](../06-proxy/01-upstream.md) yourself
  instead of using the legacy client.

## Practice
1. In [`labs/02-http-server`](../../labs/02-http-server), get one route answering `curl -v`, then make the service return `Err` on purpose and watch in `curl -v` that the connection closes without any response. Keep that observation in a comment. It is why application errors must be responses.
2. Serve one connection with `curl -v http://.../a http://.../b` and confirm in hyper's debug logs or `ss -tn` that one TCP connection carried both requests.
3. Add `header_read_timeout` (with a timer) and measure with `nc` that an idle connection is closed on schedule, both before the first request and between two requests.
4. Make a handler that returns a large streamed body, read it with `curl --limit-rate 100k`, and watch the server's RSS stay flat. Then change it to `collect()` the body first and watch RSS grow. **Done when** you can explain the difference from `poll_frame`.
5. In [`labs/05-reverse-proxy`](../../labs/05-reverse-proxy), forward one request with the legacy client, first with the original origin-form URI (observe the error), then with a rebuilt absolute URI. Stop the upstream and confirm the error reports `is_connect()`.
6. Call `graceful_shutdown()` while a slow response is in flight and confirm the response completes and the connection then closes.
