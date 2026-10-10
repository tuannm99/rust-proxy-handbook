# 07-tls

## Goal

Terminate TLS in front of an HTTP server using `tokio-rustls`, with ALPN
picking HTTP/1.1 vs HTTP/2 per client and SNI picking the certificate.

## Done when

- [ ] `curl --cacert <your-ca.pem> https://localhost:<port>/` succeeds against a certificate you generated yourself.
- [ ] `openssl s_client -alpn h2` negotiates `h2` and `-alpn http/1.1` negotiates HTTP/1.1, and each is then served by the matching protocol.
- [ ] Two certificates for two hostnames: `openssl s_client -servername a.test` and `-servername b.test` each receive the right certificate.
- [ ] TLS 1.0 and 1.1 are refused (`openssl s_client -tls1_1` fails to connect).
- [ ] Plain-text HTTP sent to the TLS port, and a client that stalls mid-handshake, both end in a closed connection after a handshake timeout — no crash, no leaked task (check with `ss` and `tokio-console`).
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/01-network/06-crypto-basics.md`](../../instruction/01-network/06-crypto-basics.md) — certificates and chains, if the vocabulary isn't solid yet
- [`instruction/01-network/19-tls.md`](../../instruction/01-network/19-tls.md) — handshake, SNI, ALPN, session resumption
- [`instruction/07-security/10-slowloris.md`](../../instruction/07-security/10-slowloris.md) — why the handshake needs its own deadline
- [`instruction/05-http-stack/02-hyper.md`](../../instruction/05-http-stack/02-hyper.md) — handing the TLS stream to hyper
- [`instruction/05-http-stack/12-vhost-routing.md`](../../instruction/05-http-stack/12-vhost-routing.md) — choosing certificates by SNI at scale

## Run

```
cargo run -p tls
```
