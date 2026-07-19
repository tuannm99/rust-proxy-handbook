// Milestone 3: Reverse Proxy. See instruction/10-projects/project-03.md.
//
// TODO:
// - accept inbound connections and forward requests to an upstream pool
//   (see instruction/06-proxy/upstream.md)
// - pick an upstream per request (see instruction/06-proxy/load-balancer.md)
// - check upstream health before routing to it (see instruction/06-proxy/healthcheck.md)
// - retry / circuit-break failed upstream calls (see instruction/06-proxy/retry.md)

#[tokio::main]
async fn main() {
    todo!("implement the reverse proxy");
}
