# Kujo patch notes

This is tiny_http 0.12.0 from crates.io. Kujo vendors it to apply a socket read timeout before tiny_http parses request headers or bodies and to treat the Unix `WouldBlock` timeout result as HTTP 408. The public upstream behavior remains the default unless `Server::http_with_read_timeout` or `Server::from_listener_with_read_timeout` is used.
