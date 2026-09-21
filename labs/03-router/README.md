# 03-router

## Goal

A router, mounted on an HTTP server (`labs/02-http-server`), that matches
method + path to a handler. Done means: unmatched paths get 404, a matched
path with the wrong method gets 405, and path params/trailing slashes are
handled deliberately, not by accident.

## Handbook references
- `instruction/05-http-stack/03-router.md` — matching method+path, trailing slashes, path params

## Run

```
cargo run -p router
```
