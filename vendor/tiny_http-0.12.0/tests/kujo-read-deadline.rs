use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

#[test]
fn incomplete_headers_expire_before_dispatch() {
    let server =
        tiny_http::Server::http_with_read_timeout("127.0.0.1:0", Duration::from_millis(150))
            .unwrap();
    let mut client = TcpStream::connect(server.server_addr().to_ip().unwrap()).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    client.write_all(b"GET / HTTP/1.1\r\nHost: unfinished").unwrap();
    let start = Instant::now();
    let mut status = String::new();
    BufReader::new(client).read_line(&mut status).unwrap();
    assert!(status.starts_with("HTTP/1.1 408"), "{status:?}");
    assert!(start.elapsed() < Duration::from_secs(5));
    assert!(server.recv_timeout(Duration::from_millis(50)).unwrap().is_none());
}

#[test]
fn complete_requests_still_dispatch_with_peer_identity() {
    let server =
        tiny_http::Server::http_with_read_timeout("127.0.0.1:0", Duration::from_secs(2)).unwrap();
    let mut client = TcpStream::connect(server.server_addr().to_ip().unwrap()).unwrap();
    client.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let peer = client.local_addr().unwrap();
    client
        .write_all(b"GET /bounded HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .unwrap();
    let request = server.recv_timeout(Duration::from_secs(2)).unwrap().unwrap();
    assert_eq!(request.url(), "/bounded");
    assert_eq!(request.remote_addr(), Some(&peer));
    request.respond(tiny_http::Response::from_string("bounded")).unwrap();
    let mut status = String::new();
    BufReader::new(client).read_line(&mut status).unwrap();
    assert!(status.starts_with("HTTP/1.1 200"), "{status:?}");
}
