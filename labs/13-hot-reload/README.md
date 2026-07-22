# 13-hot-reload

## Goal

Reload config (upstreams, listen addr, limits) from a TOML file on
SIGHUP/file-watch, swapping it in without dropping active connections.

## Handbook references
- `instruction/09-architecture/config.md`

## Run

```
cargo run -p hot-reload
```
