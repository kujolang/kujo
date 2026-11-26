use ascii::AsciiString;

use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use std::net::SocketAddr;

use crate::common::{HTTPVersion, Header, Method};
use crate::util::RefinedTcpStream;
use crate::util::{SequentialReader, SequentialReaderBuilder, SequentialWriterBuilder};
use crate::Request;

/// Maximum wire size of the request line and header fields, including each
/// line's CRLF and excluding the empty line that terminates the request head.
#[derive(Clone, Copy, Debug)]
pub struct RequestHeadLimits {
    pub request_line_bytes: usize,
    pub header_bytes: usize,
    pub header_count: usize,
}

impl Default for RequestHeadLimits {
    fn default() -> Self {
        Self { request_line_bytes: 8192, header_bytes: 16384, header_count: 100 }
    }
}

/// A ClientConnection is an object that will store a socket to a client
/// and return Request objects.
pub struct ClientConnection {
    // address of the client
    remote_addr: IoResult<Option<SocketAddr>>,

    // sequence of Readers to the stream, so that the data is not read in
    //  the wrong order
    source: SequentialReaderBuilder<BufReader<RefinedTcpStream>>,

    // sequence of Writers to the stream, to avoid writing response #2 before
    //  response #1
    sink: SequentialWriterBuilder<BufWriter<RefinedTcpStream>>,

    // Reader to read the next header from
    next_header_source: SequentialReader<BufReader<RefinedTcpStream>>,

    // set to true if we know that the previous request is the last one
    no_more_requests: bool,

    // true if the connection goes through SSL
    secure: bool,
    head_limits: RequestHeadLimits,
    read_timeout: Option<Duration>,
}

/// Error that can happen when reading a request.
#[derive(Debug)]
enum ReadError {
    WrongRequestLine,
    WrongHeader(HTTPVersion),
    RequestLineTooLong,
    HeadersTooLarge(HTTPVersion),
    /// the client sent an unrecognized `Expect` header
    ExpectationFailed(HTTPVersion),
    ReadIoError(IoError),
}

impl ClientConnection {
    /// Creates a new `ClientConnection` that takes ownership of the `TcpStream`.
    pub fn new(
        write_socket: RefinedTcpStream,
        mut read_socket: RefinedTcpStream,
        head_limits: RequestHeadLimits,
        read_timeout: Option<Duration>,
    ) -> ClientConnection {
        let remote_addr = read_socket.peer_addr();
        let secure = read_socket.secure();

        let mut source = SequentialReaderBuilder::new(BufReader::with_capacity(1024, read_socket));
        let first_header = source.next().unwrap();

        ClientConnection {
            source,
            sink: SequentialWriterBuilder::new(BufWriter::with_capacity(1024, write_socket)),
            remote_addr,
            next_header_source: first_header,
            no_more_requests: false,
            secure,
            head_limits,
            read_timeout,
        }
    }

    /// true if the connection is HTTPS
    pub fn secure(&self) -> bool {
        self.secure
    }

    /// Reads the request head before allocating a request or handing off its body.
    fn read(&mut self) -> Result<Request, ReadError> {
        // Wait for the previous request's body to release the reader before
        // starting this request's independent head budget.
        let reader = self.next_header_source.reader_mut();
        let started = Instant::now();
        if let Some(timeout) = self.read_timeout {
            reader
                .get_mut()
                .configure_request_head(Some(started), timeout)
                .map_err(ReadError::ReadIoError)?;
        }
        let (method, path, version, headers) = read_request_head(reader, self.head_limits)?;
        if let Some(timeout) = self.read_timeout {
            if started.elapsed() >= timeout {
                return Err(ReadError::ReadIoError(ErrorKind::TimedOut.into()));
            }
            reader
                .get_mut()
                .configure_request_head(None, timeout)
                .map_err(ReadError::ReadIoError)?;
        }

        // building the writer for the request
        let writer = self.sink.next().unwrap();

        // follow-up for next potential request
        let mut data_source = self.source.next().unwrap();
        std::mem::swap(&mut self.next_header_source, &mut data_source);

        // building the next reader
        let request = crate::request::new_request(
            self.secure,
            method,
            path,
            version.clone(),
            headers,
            *self.remote_addr.as_ref().unwrap(),
            data_source,
            writer,
        )
        .map_err(|e| {
            use crate::request;
            match e {
                request::RequestCreationError::CreationIoError(e) => ReadError::ReadIoError(e),
                request::RequestCreationError::ExpectationFailed => {
                    ReadError::ExpectationFailed(version)
                }
                request::RequestCreationError::InvalidFraming => ReadError::WrongHeader(version),
            }
        })?;

        // return the request
        Ok(request.with_connection_writer(self.sink.shared_writer()))
    }

    fn reject_request_head(&mut self, status: u16, version: HTTPVersion) -> Option<Request> {
        self.no_more_requests = true;
        let mut writer = self.sink.next().unwrap();
        crate::Response::new_empty(crate::StatusCode(status))
            .raw_print(&mut writer, version, &[], false, None)
            .and_then(|()| writer.flush())
            .ok();
        None
    }
}

impl Iterator for ClientConnection {
    type Item = Request;

    /// Blocks until the next Request is available.
    /// Returns None when no new Requests will come from the client.
    fn next(&mut self) -> Option<Request> {
        use crate::{Response, StatusCode};

        // the client sent a "connection: close" header in this previous request
        //  or is using HTTP 1.0, meaning that no new request will come
        if self.no_more_requests {
            return None;
        }

        loop {
            let rq = match self.read() {
                Err(ReadError::WrongRequestLine) => {
                    return self.reject_request_head(400, HTTPVersion(1, 1));
                }
                Err(ReadError::WrongHeader(version)) => {
                    return self.reject_request_head(400, version);
                }
                Err(ReadError::RequestLineTooLong) => {
                    return self.reject_request_head(414, HTTPVersion(1, 1));
                }
                Err(ReadError::HeadersTooLarge(version)) => {
                    return self.reject_request_head(413, version);
                }
                Err(ReadError::ReadIoError(ref err))
                    if matches!(err.kind(), ErrorKind::TimedOut | ErrorKind::WouldBlock) =>
                {
                    return self.reject_request_head(408, HTTPVersion(1, 1));
                }

                Err(ReadError::ExpectationFailed(ver)) => {
                    let writer = self.sink.next().unwrap();
                    let response = Response::new_empty(StatusCode(417));
                    response.raw_print(writer, ver, &[], true, None).ok();
                    return None; // TODO: should be recoverable, but needs handling in case of body
                }

                Err(ReadError::ReadIoError(_)) => return None,

                Ok(rq) => rq,
            };

            // checking HTTP version
            if *rq.http_version() > (1, 1) {
                let writer = self.sink.next().unwrap();
                let response = Response::from_string(
                    "This server only supports HTTP versions 1.0 and 1.1".to_owned(),
                )
                .with_status_code(StatusCode(505));
                response.raw_print(writer, HTTPVersion(1, 1), &[], false, None).ok();
                continue;
            }

            // updating the status of the connection
            let connection_header =
                rq.headers().iter().find(|h| h.field.equiv("Connection")).map(|h| h.value.as_str());

            let lowercase = connection_header.map(|h| h.to_ascii_lowercase());

            match lowercase {
                Some(ref val) if val.contains("close") => self.no_more_requests = true,
                Some(ref val) if val.contains("upgrade") => self.no_more_requests = true,
                Some(ref val)
                    if !val.contains("keep-alive") && *rq.http_version() == HTTPVersion(1, 0) =>
                {
                    self.no_more_requests = true
                }
                None if *rq.http_version() == HTTPVersion(1, 0) => self.no_more_requests = true,
                _ => (),
            };

            // returning the request
            return Some(rq);
        }
    }
}

// Returns None as soon as the wire limit is exhausted without a complete line.
fn read_line(reader: &mut impl Read, limit: usize) -> IoResult<Option<AsciiString>> {
    let mut bytes = Vec::new();
    for byte in reader.bytes().take(limit) {
        let byte = byte?;
        if byte == b'\n' && bytes.last() == Some(&b'\r') {
            bytes.pop();
            return AsciiString::from_ascii(bytes)
                .map(Some)
                .map_err(|_| IoError::new(ErrorKind::InvalidInput, "Header is not in ASCII"));
        }
        bytes.push(byte);
    }
    if bytes.len() == limit {
        return Ok(None);
    }
    Err(IoError::new(ErrorKind::ConnectionAborted, "Unexpected EOF"))
}

fn read_request_head(
    reader: &mut impl Read,
    limits: RequestHeadLimits,
) -> Result<(Method, String, HTTPVersion, Vec<Header>), ReadError> {
    let line = read_line(reader, limits.request_line_bytes)
        .map_err(|error| match error.kind() {
            ErrorKind::InvalidInput => ReadError::WrongRequestLine,
            _ => ReadError::ReadIoError(error),
        })?
        .ok_or(ReadError::RequestLineTooLong)?;
    let (method, path, version) = parse_request_line(line.as_str().trim())?;

    let mut headers = Vec::new();
    let mut remaining = limits.header_bytes;
    loop {
        // Allow the empty CRLF even after consuming the full byte or field limit.
        let line_limit = if headers.len() == limits.header_count { 2 } else { remaining.max(2) };
        let line = read_line(reader, line_limit)
            .map_err(|error| match error.kind() {
                ErrorKind::InvalidInput => ReadError::WrongHeader(version.clone()),
                _ => ReadError::ReadIoError(error),
            })?
            .ok_or_else(|| ReadError::HeadersTooLarge(version.clone()))?;
        if line.is_empty() {
            return Ok((method, path, version, headers));
        }
        remaining -= line.len() + 2;
        headers.push(
            parse_header(line.as_str()).ok_or_else(|| ReadError::WrongHeader(version.clone()))?,
        );
    }
}

fn parse_header(line: &str) -> Option<Header> {
    let (field, value) = line.split_once(':')?;
    // Validate the wire form before the generic Header parser trims the value.
    if field.is_empty()
        || !field
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        || !value.bytes().all(|byte| matches!(byte, b'\t' | b' '..=b'~'))
    {
        return None;
    }
    Header::from_bytes(field.as_bytes(), value.trim_matches([' ', '\t']).as_bytes()).ok()
}

/// Parses a "HTTP/1.1" string.
fn parse_http_version(version: &str) -> Result<HTTPVersion, ReadError> {
    let (major, minor) = match version {
        "HTTP/0.9" => (0, 9),
        "HTTP/1.0" => (1, 0),
        "HTTP/1.1" => (1, 1),
        "HTTP/2.0" => (2, 0),
        "HTTP/3.0" => (3, 0),
        _ => return Err(ReadError::WrongRequestLine),
    };

    Ok(HTTPVersion(major, minor))
}

/// Parses the request line of the request.
/// eg. GET / HTTP/1.1
fn parse_request_line(line: &str) -> Result<(Method, String, HTTPVersion), ReadError> {
    let mut parts = line.split(' ');

    let method = parts.next().and_then(|w| w.parse().ok());
    let path = parts.next().map(ToOwned::to_owned);
    let version = parts.next().and_then(|w| parse_http_version(w).ok());

    method.and_then(|method| Some((method, path?, version?))).ok_or(ReadError::WrongRequestLine)
}

#[cfg(test)]
mod test {
    use super::{read_request_head, RequestHeadLimits};

    #[test]
    fn exact_head_limits_leave_following_bytes_for_the_body() {
        let request_line = b"POST / HTTP/1.1\r\n";
        let headers = b"Host: localhost\r\nContent-Length: 4\r\n";
        let data = [request_line.as_slice(), headers, b"\r\nbody"].concat();
        let limits = RequestHeadLimits {
            request_line_bytes: request_line.len(),
            header_bytes: headers.len(),
            header_count: 2,
        };
        let mut reader = data.as_slice();
        let (method, _, _, fields) = read_request_head(&mut reader, limits).unwrap();
        assert_eq!(method, crate::Method::Post);
        assert_eq!(fields.len(), 2);
        assert_eq!(reader, b"body");
    }

    #[test]
    fn test_parse_request_line() {
        let (method, path, ver) = super::parse_request_line("GET /hello HTTP/1.1").unwrap();

        assert!(method == crate::Method::Get);
        assert!(path == "/hello");
        assert!(ver == crate::common::HTTPVersion(1, 1));

        assert!(super::parse_request_line("GET /hello").is_err());
        assert!(super::parse_request_line("qsd qsd qsd").is_err());
    }
}
