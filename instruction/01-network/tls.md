# TLS

Handshake, SNI, ALPN, mTLS, session resumption.

## What to learn

### The TLS 1.3 handshake
Client and server negotiate a cipher suite, exchange (EC)DHE key shares,
and derive session keys — TLS 1.3 does this in one round trip (vs two for
TLS 1.2), with the server's certificate itself encrypted once keys are
derived. A proxy terminating TLS sits exactly here: it holds the private
key, completes the handshake with the client, and (usually) speaks
plaintext or a *separate* TLS session to the upstream.

### SNI (Server Name Indication)
The client sends the target hostname in cleartext during the handshake
(before the server picks a certificate), which is how one proxy on one IP
can terminate TLS for many different domains/certs — pick the cert based
on SNI, not on the IP the request arrived on. Gotcha: SNI is sent in
cleartext by default (ECH/Encrypted Client Hello is the fix, not yet
universal), so a proxy can route on it but shouldn't assume it's private.

### ALPN (Application-Layer Protocol Negotiation)
Negotiated inside the same handshake, ALPN is how client and server agree
on HTTP/1.1 vs HTTP/2 (`h2`) before any HTTP bytes are exchanged — this is
what lets `hyper-util`'s auto server in `labs/02-http-server` pick the
right protocol without a separate port per version.

```rust
// tokio-rustls: advertise HTTP/2 then HTTP/1.1 via ALPN
let mut config = rustls::ServerConfig::builder()
    .with_no_client_auth()
    .with_single_cert(cert_chain, private_key)?;
config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
```

### mTLS (mutual TLS)
The server also requests and verifies a client certificate, authenticating
the client at the transport layer instead of (or in addition to) an
application-layer token. Common at the proxy layer for service-to-service
trust inside a private network — tie to `07-security/auth.md` for how this
composes with JWT-based auth for end-user requests.

### Session resumption
Session tickets (or session IDs) let a returning client skip the full
handshake on a new connection, cutting a round trip. For a proxy, this
matters most under high connection churn — resumption support (and its
key rotation) directly affects tail latency for clients reconnecting
frequently. Gotcha: 0-RTT-style resumption reintroduces replay risk similar
to QUIC 0-RTT (`http3.md`) — apply the same "only for idempotent requests"
caution.

### Certificate management
A production proxy needs certs issued, renewed (typically via ACME/Let's
Encrypt), and reloaded *without* dropping existing connections or requiring
a restart — this is why `proxy` treats cert reload
as a config-reload concern, see `09-architecture/config.md`.

## Practice

1. Use `openssl s_client -connect host:443 -servername example.com` and
   read the handshake output to identify the negotiated cipher suite and
   TLS version.
2. Add TLS termination to `proxy` using
   `tokio-rustls` (already a dependency), serving a self-signed cert for
   local testing.
3. Configure ALPN so `curl --http2` and `curl --http1.1` both work against
   the same port, and confirm via `curl -v` which protocol was negotiated.
4. Add mTLS: require and verify a client certificate, and reject
   connections presenting none or an untrusted one.
5. Simulate a cert rotation (swap the cert file, trigger reload per
   `09-architecture/config.md`) and confirm existing connections aren't
   dropped mid-request.
