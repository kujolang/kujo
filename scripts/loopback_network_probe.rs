// Dependency-free host diagnostic for loopback stalls, separate from Kujo.
// rustc --edition=2021 --test scripts/loopback_network_probe.rs -o target/debug/deps/kujo_host_network_probe
// target/debug/deps/kujo_host_network_probe --test-threads=2
// A failure is evidence of a host networking problem, not a Kujo runtime failure.
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

fn roundtrip() {
    let deadline = Duration::from_secs(2);
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).expect("nonblocking accept");
    let server = thread::spawn(move || -> std::io::Result<()> {
        let started = Instant::now();
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(connection) => break connection,
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && started.elapsed() < deadline =>
                {
                    thread::sleep(Duration::from_millis(1));
                }
                Err(error) => return Err(error),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(deadline))?;
        stream.set_write_timeout(Some(deadline))?;
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        stream.write_all(&byte)
    });
    let client = TcpStream::connect_timeout(&address, deadline).and_then(|mut stream| {
        stream.set_read_timeout(Some(deadline))?;
        stream.set_write_timeout(Some(deadline))?;
        stream.write_all(b"a")?;
        let mut byte = [0];
        stream.read_exact(&mut byte)?;
        assert_eq!(byte, *b"a");
        Ok(())
    });
    let server = server.join().expect("loopback server thread");
    assert!(client.is_ok(), "loopback client failed: {client:?}; server: {server:?}");
    assert!(server.is_ok(), "loopback server failed: {server:?}");
}

#[test]
fn first_loopback_connection() {
    roundtrip();
}

#[test]
fn concurrent_loopback_connection() {
    roundtrip();
}
