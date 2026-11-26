# Kujo patch notes

This is tiny_http 0.12.0 from crates.io. Kujo adds opt-in socket timeouts, a cumulative per-request-head read deadline for servers with a read timeout, and HTTP 408 on socket read timeout. Request-head parsing uses 8,192 request-line bytes, 16,384 header bytes, and 100 fields by default. `Server::from_listener_with_limits` accepts custom request-head limits.
