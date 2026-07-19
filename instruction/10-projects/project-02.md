# Project 2
HTTP Server.

## Goal
A hyper-based HTTP/1.1 (and HTTP/2) server that routes requests by method +
path to different handlers and can serve static files from disk. "Done"
means: `curl` gets correct status codes for matched/unmatched routes,
keep-alive connections correctly serve multiple requests without hyper
hanging or closing early, and a static file request streams the file rather
than reading it fully into memory first.

## What to learn
- `05-http-stack/parser.md` — request line/headers/body framing (do the `labs/http-parser-raw` exercise first so hyper's API makes sense)
- `05-http-stack/router.md` — matching method+path to a handler, trailing slashes, path params
- `05-http-stack/static.md` — streaming files, `Content-Type` from extension, range requests
- `05-http-stack/keepalive.md` — persistent connections, when a connection *can't* be reused
- `01-network/http.md`, `01-network/http2.md` — status codes/headers, and what changes when a client negotiates h2

## Practice
Build `milestones/02-http` incrementally:
1. Serve a single hardcoded 200 response on every request over hyper's HTTP/1 server; confirm with `curl -v` that keep-alive works across repeated requests on one connection.
2. Add a router: match on method + exact path, return 404 for anything unmatched, 405 for a matched path with wrong method.
3. Add static file serving from a directory: stream the file body (don't `read_to_string`/`read` fully), set `Content-Type` from extension, return 404 for missing files and reject path traversal (`../`).
4. Enable HTTP/2 (h2c or with TLS) and confirm the same routes work; note what hyper does differently under the hood (multiplexing vs one-request-per-connection-at-a-time).
5. Deliberately send a client that opens a connection and never sends a full request — confirm the server doesn't hang forever holding that connection (add a read timeout).
