//! Pinning must constrain the actual socket, including in proxy-configured hosts.
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::Command;
use std::time::Duration;

#[test]
fn explicitly_pinned_http_ignores_ambient_proxy_resolvers() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let mut request = [0; 4096];
        assert!(stream.read(&mut request).unwrap() > 0);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\nConnection: close\r\n\r\npinned")
            .unwrap();
    });
    let directory = tempfile::tempdir().unwrap();
    let script = directory.path().join("request.kujo");
    std::fs::write(&script, format!("let result := http_request(\"http://{address}/\", {{\"pin_dns\": true, \"redirects\": \"none\", \"timeout\": 3}})\nmatch result {{ case Ok(response): {{ print(response[\"body\"]) }} case Err(message): {{ eprint(message); exit(1) }} }}\n")).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
    command.args(["run", script.to_str().unwrap(), "--allow-net-client"]);
    for variable in
        ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"]
    {
        command.env(variable, "http://127.0.0.1:1");
    }
    command.env("NO_PROXY", "").env("no_proxy", "");
    let output = command.output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "pinned");
    server.join().unwrap();
}
