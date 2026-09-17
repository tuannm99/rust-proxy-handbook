// THE deliverable: the complete production L7 proxy. See proxy/README.md.
//
// TODO:
// - load config from file, support hot reload (see instruction/09-architecture/config.md)
// - wire up the component pipeline (see instruction/09-architecture/components.md)
// - TLS termination (see instruction/01-network/tls.md)
// - auth, rate limiting, WAF (see instruction/07-security/*.md)
// - structured logging, metrics, tracing (see instruction/08-observability/*.md)
// - graceful shutdown on SIGTERM (see instruction/09-architecture/graceful-shutdown.md)
// - SLOs + burn-rate alerting (see instruction/08-observability/slo.md, alerting.md)
// - zero-downtime binary upgrade (see instruction/09-architecture/rolling-restart.md)

#[tokio::main]
async fn main() {
    todo!("implement the production proxy");
}
