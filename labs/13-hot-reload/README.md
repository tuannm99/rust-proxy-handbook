# 13-hot-reload

## Goal

Reload config (upstreams, limits, listen address) from a TOML file on
`SIGHUP` or file change, swapping it in without dropping active
connections.

## Done when

- [ ] Editing the upstream list and sending `SIGHUP` routes new requests to the new list within a second.
- [ ] A load test running across ten reloads shows zero failed requests.
- [ ] An invalid config file is rejected: the old config stays active and the error is logged with the reason.
- [ ] The request path never blocks on a reload — config is read through `ArcSwap` or a `watch` channel, not a lock held during parsing ([`instruction/03-rust/11-concurrency-patterns.md`](../../instruction/03-rust/11-concurrency-patterns.md)).
- [ ] Stretch: changing the listen address binds the new listener and drains the old one.
- [ ] Reviewed per [`instruction/00-introduction/03-study-loop.md`](../../instruction/00-introduction/03-study-loop.md) step 5.

## Handbook references
- [`instruction/09-architecture/03-config.md`](../../instruction/09-architecture/03-config.md)
- [`instruction/02-linux/10-signals.md`](../../instruction/02-linux/10-signals.md) — handling `SIGHUP` in an async process

## Run

```
cargo run -p hot-reload
```
