# TLS

Handshake, SNI, ALPN, mTLS, session resumption. If "asymmetric
encryption," "certificate chain," or "digital signature" aren't already
precise terms, read [`01-network/06-crypto-basics.md`](06-crypto-basics.md) first — this file
assumes them.

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
what lets `hyper-util`'s auto server in [`labs/02-http-server`](../../labs/02-http-server) pick the
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
trust inside a private network — tie to [`07-security/01-auth.md`](../07-security/01-auth.md) for how this
composes with JWT-based auth for end-user requests.

### Session resumption
Session tickets (or session IDs) let a returning client skip the full
handshake on a new connection, cutting a round trip. For a proxy, this
matters most under high connection churn — resumption support (and its
key rotation) directly affects tail latency for clients reconnecting
frequently. Gotcha: 0-RTT-style resumption reintroduces replay risk similar
to QUIC 0-RTT ([`13-http3.md`](13-http3.md)) — apply the same "only for idempotent requests"
caution.

### Certificate management
A production proxy needs certs issued, renewed (typically via ACME/Let's
Encrypt), and reloaded *without* dropping existing connections or requiring
a restart — this is why [`proxy`](../../proxy) treats cert reload
as a config-reload concern, see [`09-architecture/03-config.md`](../09-architecture/03-config.md).

### A local CA for testing
`curl --cacert` needs a certificate that chains to a CA you trust, so make
two things: a CA, and a server ("leaf") certificate signed by it. Serving
the CA certificate itself is fragile: curl may accept it, but a
rustls-based client (like the `quinn` client in [`labs/09-http3`](../../labs/09-http3)) rejects a
CA certificate presented as a server certificate. Modern clients also ignore the subject `CN`
and match the hostname only against the **Subject Alternative Name** (SAN)
list, so a leaf without a SAN fails verification even though it "has the
right name".

```sh
# 1. the CA: a self-signed certificate plus its key (CA:TRUE is set by default)
openssl req -x509 -newkey rsa:2048 -nodes -keyout ca.key -out ca.pem \
  -days 365 -subj "/CN=Lab CA"
# 2. a key and signing request for the server
openssl req -newkey rsa:2048 -nodes -keyout a.key -out a.csr -subj "/CN=a.test"
# 3. the CA signs it, adding the SAN list the client will match against
printf "subjectAltName=DNS:a.test,DNS:localhost,IP:127.0.0.1\n" > a.ext
openssl x509 -req -in a.csr -CA ca.pem -CAkey ca.key -CAcreateserial \
  -out a.pem -days 30 -extfile a.ext
openssl verify -CAfile ca.pem a.pem        # must print "a.pem: OK"
```

Repeat steps 2-3 with `b.test` for the second hostname. The server loads
`a.pem` + `a.key`. Clients get `ca.pem`: `curl --cacert ca.pem
https://localhost:8443/`. To test a name that doesn't resolve, pin it:
`curl --cacert ca.pem --resolve a.test:8443:127.0.0.1 https://a.test:8443/`.
`openssl s_client -connect 127.0.0.1:8443 -servername a.test -CAfile ca.pem`
shows which certificate came back (`subject=`) and the negotiated ALPN
(`ALPN protocol:`).

### rustls and tokio-rustls in practice
The pieces, in the order a connection uses them:

- **A crypto provider.** rustls 0.23 does no cryptography itself. It uses a
  provider: `aws-lc-rs` (the default feature) or `ring`. With exactly one of
  them enabled in your whole dependency graph, `ServerConfig::builder()`
  picks it automatically. If both end up enabled (another crate pulled in
  the other one), the builder panics at runtime asking for a
  process-level `CryptoProvider`. The fix is one call at startup, for
  example `rustls::crypto::aws_lc_rs::default_provider().install_default()`.
  This repo hits it: `quinn` ([`labs/09-http3`](../../labs/09-http3)) enables `ring` and
  `tokio-rustls` enables `aws-lc-rs`. `cargo run -p tls` builds only lab
  07's features and works, but `cargo build --workspace` unifies features
  across all members, and the resulting `tls` binary gets both.
- **Loading PEM files.** With the `rustls::pki_types::pem::PemObject` trait
  in scope, `CertificateDer::pem_file_iter(path)` yields every certificate in
  a file and `PrivateKeyDer::from_pem_file(path)` loads the key (PKCS#8,
  PKCS#1 or SEC1). The certificate file you serve holds the leaf first,
  then any intermediates, never the root.
- **The config.** `ServerConfig::builder().with_no_client_auth()` then either
  `.with_single_cert(chain, key)` for one certificate, or
  `.with_cert_resolver(resolver)` to choose per SNI name (below). Set
  `alpn_protocols` on the resulting config, as in the ALPN section above.
- **The handshake.** `tokio_rustls::TlsAcceptor::from(Arc::new(config))`,
  then `acceptor.accept(tcp_stream)` is a future that performs the whole
  handshake and yields a TLS stream. That future has no deadline of its
  own: a client that connects and sends nothing holds it open forever.
  Wrap it in `tokio::time::timeout` ([`07-security/10-slowloris.md`](../07-security/10-slowloris.md)).
- **After the handshake.** The TLS stream goes into `TokioIo::new(...)` and
  then into hyper exactly like a TCP stream ([`05-http-stack/02-hyper.md`](../05-http-stack/02-hyper.md)).
  `tls_stream.get_ref().1` is the rustls connection. Its `alpn_protocol()`
  returns `Some(b"h2")` or `Some(b"http/1.1")`, so you can pick an HTTP/2-only
  or HTTP/1.1-only builder instead of letting the auto builder sniff.

**Choosing a certificate by SNI.** `rustls::server::ResolvesServerCertUsingSni`
maps names to certificates: `.add("a.test", certified_key)`, where a
`CertifiedKey` is the chain plus a signing key built from the private key
(`CertifiedKey::from_der` takes the chain, the key and the provider). It
has no fallback: a client that sends no SNI, or an unknown name, fails the
handshake. If you want a default certificate, implement the
`ResolvesServerCert` trait yourself. Its one method receives the
`ClientHello`, whose `server_name()` is the SNI value. The same mechanism
at scale, with many tenants, is [`05-http-stack/12-vhost-routing.md`](../05-http-stack/12-vhost-routing.md).

**Protocol versions.** rustls implements only TLS 1.2 and TLS 1.3. TLS 1.0
and 1.1 don't exist in it, so "refuse TLS 1.1" holds by construction. To
go further and allow only 1.3, build with
`ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS13])`.

Gotcha: when testing that an old version is refused, make sure it's your
*server* refusing. OpenSSL 3's own client also refuses TLS 1.0/1.1 at its
default security level, so `openssl s_client -tls1_1` fails against any
server. Add `-cipher 'DEFAULT@SECLEVEL=0'` so the client really offers
TLS 1.1. Then a failure proves the server said no.

## Practice

1. Use `openssl s_client -connect host:443 -servername example.com` and
   read the handshake output to identify the negotiated cipher suite and
   TLS version.
2. Do [`labs/07-tls`](../../labs/07-tls) first: terminate TLS with `tokio-rustls` on a bare
   listener, serving a self-signed cert for local testing. Once it works
   there, port the same setup into [`proxy`](../../proxy).
3. Configure ALPN so `curl --http2` and `curl --http1.1` both work against
   the same port, and confirm via `curl -v` which protocol was negotiated.
4. Add mTLS: require and verify a client certificate, and reject
   connections presenting none or an untrusted one.
5. Simulate a cert rotation (swap the cert file, trigger reload per
   [`09-architecture/03-config.md`](../09-architecture/03-config.md)) and confirm existing connections aren't
   dropped mid-request.
