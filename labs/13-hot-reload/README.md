# 13-hot-reload

Reload config (upstreams, listen addr, limits) from a TOML file on
SIGHUP/file-watch, swapping it in without dropping active connections.

Handbook references:
- `instruction/09-architecture/config.md`

Run with:

```
cargo run -p hot-reload
```
