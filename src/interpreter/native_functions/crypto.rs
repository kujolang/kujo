// File: src/interpreter/native_functions/crypto.rs
//
// Cryptography native functions

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use hmac::{Hmac, Mac};
use md5::Md5;
use openssl::encrypt::{Decrypter, Encrypter};
use openssl::error::ErrorStack;
use openssl::hash::MessageDigest;
use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::{Padding, Rsa};
use openssl::sign::{Signer, Verifier};
use sha2::{Digest, Sha256};

use crate::interpreter::{DictMap, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, ErrorKind, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;

const STREAM_AEAD_MAGIC: &[u8; 9] = b"KUJOAEAD1";
const STREAM_AEAD_MIN_CHUNK: usize = 4096;
const STREAM_AEAD_MAX_CHUNK: usize = 16 * 1024 * 1024;
const DECODE_RANGE_MAX_OUTPUT: u64 = 64 * 1024 * 1024;
const DECODE_RANGE_MAX_PREFIX: usize = 4096;

fn error_object(message: String) -> Value {
    Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sha256_reader_range(file: File, offset: u64, count: Option<u64>) -> Result<String, String> {
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut remaining = count;
    loop {
        let ceiling = remaining
            .map(|value| usize::try_from(value.min(buffer.len() as u64)).unwrap_or(buffer.len()))
            .unwrap_or(buffer.len());
        if ceiling == 0 {
            break;
        }
        let read = reader.read(&mut buffer[..ceiling]).map_err(|error| error.to_string())?;
        if read == 0 {
            if remaining.is_some_and(|value| value > 0) {
                return Err("requested hash range exceeds file length".to_string());
            }
            break;
        }
        hasher.update(&buffer[..read]);
        if let Some(value) = remaining {
            remaining = Some(value - read as u64);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_canonical_text_reader_range(
    file: File,
    offset: u64,
    count: u64,
    mode: &str,
) -> Result<String, String> {
    if mode != "relaxed-crlf" && mode != "simple-crlf" {
        return Err("mode must be relaxed-crlf or simple-crlf".to_string());
    }
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut remaining = count;
    let mut line = Vec::new();
    let mut pending_empty_lines = 0_u64;
    let mut wrote_nonempty = false;
    let mut saw_cr = false;

    let commit_line = |line: &mut Vec<u8>,
                       hasher: &mut Sha256,
                       pending_empty_lines: &mut u64,
                       wrote_nonempty: &mut bool|
     -> Result<(), String> {
        let canonical = if mode == "relaxed-crlf" {
            let mut output = Vec::with_capacity(line.len());
            let mut whitespace = false;
            for byte in line.iter().copied() {
                if byte == b' ' || byte == b'\t' {
                    whitespace = true;
                } else {
                    if whitespace {
                        output.push(b' ');
                    }
                    output.push(byte);
                    whitespace = false;
                }
            }
            output
        } else {
            line.clone()
        };
        if canonical.is_empty() {
            *pending_empty_lines = pending_empty_lines
                .checked_add(1)
                .ok_or_else(|| "canonical empty-line count overflow".to_string())?;
        } else {
            for _ in 0..*pending_empty_lines {
                hasher.update(b"\r\n");
            }
            *pending_empty_lines = 0;
            hasher.update(&canonical);
            hasher.update(b"\r\n");
            *wrote_nonempty = true;
        }
        line.clear();
        Ok(())
    };

    while remaining > 0 {
        let ceiling = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader.read(&mut buffer[..ceiling]).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("requested canonical hash range exceeds file length".to_string());
        }
        remaining -= read as u64;
        for byte in buffer[..read].iter().copied() {
            if saw_cr {
                if byte != b'\n' {
                    return Err("canonical text range contains a bare carriage return".to_string());
                }
                commit_line(&mut line, &mut hasher, &mut pending_empty_lines, &mut wrote_nonempty)?;
                saw_cr = false;
            } else if byte == b'\r' {
                saw_cr = true;
            } else if byte == b'\n' {
                return Err("canonical text range contains a bare line feed".to_string());
            } else {
                line.push(byte);
                if line.len() > 1024 * 1024 {
                    return Err("canonical text line exceeds 1048576 bytes".to_string());
                }
            }
        }
    }
    if saw_cr {
        return Err("canonical text range ends with a bare carriage return".to_string());
    }
    if !line.is_empty() {
        commit_line(&mut line, &mut hasher, &mut pending_empty_lines, &mut wrote_nonempty)?;
    }
    if mode == "simple-crlf" && !wrote_nonempty {
        hasher.update(b"\r\n");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

struct DecodeRangeSink {
    hasher: Sha256,
    output_bytes: u64,
    max_output_bytes: u64,
    prefix: Vec<u8>,
    prefix_bytes: usize,
    ascii: bool,
    contains_nul: bool,
    utf8: Utf8State,
}

#[derive(Default)]
struct Utf8State {
    remaining: u8,
    next_min: u8,
    next_max: u8,
    valid: bool,
    initialized: bool,
}

impl Utf8State {
    fn push(&mut self, byte: u8) {
        if !self.initialized {
            self.valid = true;
            self.initialized = true;
        }
        if !self.valid {
            return;
        }
        if self.remaining > 0 {
            if byte < self.next_min || byte > self.next_max {
                self.valid = false;
                return;
            }
            self.remaining -= 1;
            self.next_min = 0x80;
            self.next_max = 0xbf;
            return;
        }
        match byte {
            0x00..=0x7f => {}
            0xc2..=0xdf => {
                self.remaining = 1;
                self.next_min = 0x80;
                self.next_max = 0xbf;
            }
            0xe0 => {
                self.remaining = 2;
                self.next_min = 0xa0;
                self.next_max = 0xbf;
            }
            0xe1..=0xec | 0xee..=0xef => {
                self.remaining = 2;
                self.next_min = 0x80;
                self.next_max = 0xbf;
            }
            0xed => {
                self.remaining = 2;
                self.next_min = 0x80;
                self.next_max = 0x9f;
            }
            0xf0 => {
                self.remaining = 3;
                self.next_min = 0x90;
                self.next_max = 0xbf;
            }
            0xf1..=0xf3 => {
                self.remaining = 3;
                self.next_min = 0x80;
                self.next_max = 0xbf;
            }
            0xf4 => {
                self.remaining = 3;
                self.next_min = 0x80;
                self.next_max = 0x8f;
            }
            _ => self.valid = false,
        }
    }

    fn is_valid(&self) -> bool {
        (!self.initialized || self.valid) && self.remaining == 0
    }
}

impl DecodeRangeSink {
    fn new(max_output_bytes: u64, prefix_bytes: usize) -> Self {
        Self {
            hasher: Sha256::new(),
            output_bytes: 0,
            max_output_bytes,
            prefix: Vec::with_capacity(prefix_bytes),
            prefix_bytes,
            ascii: true,
            contains_nul: false,
            utf8: Utf8State::default(),
        }
    }

    fn push(&mut self, bytes: &[u8]) -> Result<(), String> {
        let next = self
            .output_bytes
            .checked_add(bytes.len() as u64)
            .ok_or_else(|| "decoded output length overflow".to_string())?;
        if next > self.max_output_bytes {
            return Err(format!(
                "decoded output exceeds configured limit of {} bytes",
                self.max_output_bytes
            ));
        }
        let available = self.prefix_bytes.saturating_sub(self.prefix.len());
        self.prefix.extend_from_slice(&bytes[..bytes.len().min(available)]);
        for byte in bytes.iter().copied() {
            self.ascii &= byte <= 0x7f;
            self.contains_nul |= byte == 0;
            self.utf8.push(byte);
        }
        self.hasher.update(bytes);
        self.output_bytes = next;
        Ok(())
    }
}

fn read_exact_range<F>(file: File, offset: u64, count: u64, mut consume: F) -> Result<(), String>
where
    F: FnMut(&[u8]) -> Result<(), String>,
{
    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
    let mut remaining = count;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let ceiling = usize::try_from(remaining.min(buffer.len() as u64)).unwrap_or(buffer.len());
        let read = reader.read(&mut buffer[..ceiling]).map_err(|error| error.to_string())?;
        if read == 0 {
            return Err("requested decode range exceeds file length".to_string());
        }
        remaining -= read as u64;
        consume(&buffer[..read])?;
    }
    Ok(())
}

fn decode_file_range(
    file: File,
    offset: u64,
    count: u64,
    encoding: &str,
    max_output_bytes: u64,
    prefix_bytes: usize,
) -> Result<Value, String> {
    if max_output_bytes > DECODE_RANGE_MAX_OUTPUT {
        return Err(format!("max_output_bytes exceeds {} bytes", DECODE_RANGE_MAX_OUTPUT));
    }
    if prefix_bytes > DECODE_RANGE_MAX_PREFIX {
        return Err(format!("prefix_bytes exceeds {} bytes", DECODE_RANGE_MAX_PREFIX));
    }
    let mut sink = DecodeRangeSink::new(max_output_bytes, prefix_bytes);
    let mut max_input_line_bytes = 0_u64;
    let mut input_line_bytes = 0_u64;
    let mut saw_cr = false;

    match encoding {
        "identity" => read_exact_range(file, offset, count, |bytes| sink.push(bytes))?,
        "base64" => {
            let mut quartet = [0_u8; 4];
            let mut quartet_len = 0_usize;
            let mut finished = false;
            read_exact_range(file, offset, count, |bytes| {
                for byte in bytes.iter().copied() {
                    if saw_cr {
                        if byte != b'\n' {
                            return Err("base64 input contains a bare carriage return".to_string());
                        }
                        max_input_line_bytes = max_input_line_bytes.max(input_line_bytes);
                        input_line_bytes = 0;
                        saw_cr = false;
                        continue;
                    }
                    if byte == b'\r' {
                        saw_cr = true;
                        continue;
                    }
                    if byte == b'\n' {
                        return Err("base64 input contains a bare line feed".to_string());
                    }
                    if byte == b' ' || byte == b'\t' {
                        input_line_bytes += 1;
                        if input_line_bytes > 76 {
                            return Err("base64 input line exceeds 76 bytes".to_string());
                        }
                        continue;
                    }
                    input_line_bytes += 1;
                    if input_line_bytes > 76 {
                        return Err("base64 input line exceeds 76 bytes".to_string());
                    }
                    if finished {
                        return Err("base64 input contains data after padding".to_string());
                    }
                    if !(byte.is_ascii_alphanumeric()
                        || byte == b'+'
                        || byte == b'/'
                        || byte == b'=')
                    {
                        return Err("base64 input contains an invalid byte".to_string());
                    }
                    quartet[quartet_len] = byte;
                    quartet_len += 1;
                    if quartet_len == 4 {
                        let decoded =
                            base64::engine::general_purpose::STANDARD.decode(quartet).map_err(
                                |_| "base64 input has invalid padding or quartet".to_string(),
                            )?;
                        finished = quartet[2] == b'=' || quartet[3] == b'=';
                        sink.push(&decoded)?;
                        quartet_len = 0;
                    }
                }
                Ok(())
            })?;
            if saw_cr {
                return Err("base64 input ends with a bare carriage return".to_string());
            }
            if quartet_len != 0 {
                return Err("base64 input ends with an incomplete quartet".to_string());
            }
            max_input_line_bytes = max_input_line_bytes.max(input_line_bytes);
        }
        "quoted-printable" => {
            #[derive(Clone, Copy)]
            enum QpState {
                Normal,
                Equals,
                Hex(u8),
                SoftCr,
                Cr,
            }
            fn hex(byte: u8) -> Option<u8> {
                match byte {
                    b'0'..=b'9' => Some(byte - b'0'),
                    b'a'..=b'f' => Some(byte - b'a' + 10),
                    b'A'..=b'F' => Some(byte - b'A' + 10),
                    _ => None,
                }
            }
            let mut state = QpState::Normal;
            let mut pending_ws = Vec::new();
            read_exact_range(file, offset, count, |bytes| {
                for byte in bytes.iter().copied() {
                    match state {
                        QpState::Normal => {
                            match byte {
                                b'=' => {
                                    sink.push(&pending_ws)?;
                                    pending_ws.clear();
                                    input_line_bytes += 1;
                                    if input_line_bytes > 76 {
                                        return Err("quoted-printable input line exceeds 76 bytes"
                                            .to_string());
                                    }
                                    state = QpState::Equals;
                                }
                                b'\r' => {
                                    if !pending_ws.is_empty() {
                                        return Err("quoted-printable line has unencoded trailing whitespace".to_string());
                                    }
                                    state = QpState::Cr;
                                }
                                b'\n' => {
                                    return Err("quoted-printable input contains a bare line feed"
                                        .to_string())
                                }
                                b' ' | b'\t' => {
                                    pending_ws.push(byte);
                                    input_line_bytes += 1;
                                    if input_line_bytes > 76 {
                                        return Err("quoted-printable input line exceeds 76 bytes"
                                            .to_string());
                                    }
                                }
                                _ => {
                                    if !(33..=126).contains(&byte) {
                                        return Err("quoted-printable input contains an unencoded non-printable or non-ASCII byte".to_string());
                                    }
                                    sink.push(&pending_ws)?;
                                    pending_ws.clear();
                                    sink.push(&[byte])?;
                                    input_line_bytes += 1;
                                    if input_line_bytes > 76 {
                                        return Err("quoted-printable input line exceeds 76 bytes"
                                            .to_string());
                                    }
                                }
                            }
                        }
                        QpState::Equals => {
                            if byte == b'\r' {
                                state = QpState::SoftCr;
                            } else if let Some(value) = hex(byte) {
                                input_line_bytes += 1;
                                if input_line_bytes > 76 {
                                    return Err(
                                        "quoted-printable input line exceeds 76 bytes".to_string()
                                    );
                                }
                                state = QpState::Hex(value);
                            } else {
                                return Err(
                                    "quoted-printable input has an invalid escape".to_string()
                                );
                            }
                        }
                        QpState::Hex(high) => {
                            let Some(low) = hex(byte) else {
                                return Err(
                                    "quoted-printable input has an invalid hex escape".to_string()
                                );
                            };
                            input_line_bytes += 1;
                            if input_line_bytes > 76 {
                                return Err(
                                    "quoted-printable input line exceeds 76 bytes".to_string()
                                );
                            }
                            sink.push(&[(high << 4) | low])?;
                            state = QpState::Normal;
                        }
                        QpState::SoftCr => {
                            if byte != b'\n' {
                                return Err(
                                    "quoted-printable soft break has a bare carriage return"
                                        .to_string(),
                                );
                            }
                            max_input_line_bytes = max_input_line_bytes.max(input_line_bytes);
                            input_line_bytes = 0;
                            state = QpState::Normal;
                        }
                        QpState::Cr => {
                            if byte != b'\n' {
                                return Err(
                                    "quoted-printable input contains a bare carriage return"
                                        .to_string(),
                                );
                            }
                            sink.push(b"\r\n")?;
                            max_input_line_bytes = max_input_line_bytes.max(input_line_bytes);
                            input_line_bytes = 0;
                            state = QpState::Normal;
                        }
                    }
                }
                Ok(())
            })?;
            if !matches!(state, QpState::Normal) {
                return Err("quoted-printable input ends with an incomplete escape or line ending"
                    .to_string());
            }
            if !pending_ws.is_empty() {
                return Err(
                    "quoted-printable input ends with unencoded trailing whitespace".to_string()
                );
            }
            max_input_line_bytes = max_input_line_bytes.max(input_line_bytes);
        }
        _ => return Err("encoding must be identity, base64, or quoted-printable".to_string()),
    }

    let mut result = DictMap::default();
    result.insert(
        Arc::<str>::from("schema"),
        Value::Str(Arc::new("kujo.file.decode.v1".to_string())),
    );
    result.insert(Arc::<str>::from("encoding"), Value::Str(Arc::new(encoding.to_string())));
    result.insert(Arc::<str>::from("input_bytes"), Value::Int(count as i64));
    result.insert(Arc::<str>::from("output_bytes"), Value::Int(sink.output_bytes as i64));
    result.insert(
        Arc::<str>::from("sha256"),
        Value::Str(Arc::new(format!("{:x}", sink.hasher.finalize()))),
    );
    result.insert(Arc::<str>::from("prefix"), Value::Bytes(sink.prefix));
    result.insert(Arc::<str>::from("ascii"), Value::Bool(sink.ascii));
    result.insert(Arc::<str>::from("contains_nul"), Value::Bool(sink.contains_nul));
    result.insert(Arc::<str>::from("utf8_valid"), Value::Bool(sink.utf8.is_valid()));
    result
        .insert(Arc::<str>::from("max_input_line_bytes"), Value::Int(max_input_line_bytes as i64));
    Ok(Value::Dict(Arc::new(result)))
}

fn rsa_public_key_info(value: &Value) -> Result<Value, String> {
    let (key, format) = match value {
        Value::Str(pem) => (
            PKey::public_key_from_pem(pem.as_bytes())
                .map_err(|error| format!("Invalid RSA public key PEM: {error}"))?,
            "pem",
        ),
        Value::Bytes(der) => (
            PKey::public_key_from_der(der)
                .map_err(|error| format!("Invalid RSA public key DER: {error}"))?,
            "der",
        ),
        _ => return Err("rsa_public_key_info requires a PEM string or DER bytes".to_string()),
    };
    key.rsa().map_err(|error| format!("Public key is not RSA: {error}"))?;
    let pem = key.public_key_to_pem().map_err(|error| error.to_string())?;
    let pem = String::from_utf8(pem).map_err(|error| error.to_string())?;
    let der = key.public_key_to_der().map_err(|error| error.to_string())?;
    let mut result = DictMap::default();
    result.insert(Arc::<str>::from("algorithm"), Value::Str(Arc::new("rsa".to_string())));
    result.insert(Arc::<str>::from("bits"), Value::Int(i64::from(key.bits())));
    result.insert(Arc::<str>::from("format"), Value::Str(Arc::new(format.to_string())));
    result.insert(Arc::<str>::from("pem"), Value::Str(Arc::new(pem)));
    result.insert(Arc::<str>::from("sha256"), Value::Str(Arc::new(sha256_hex(&der))));
    Ok(Value::Dict(Arc::new(result)))
}

fn hmac_sha256_hex(secret: &[u8], message: &[u8]) -> String {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret)
        .expect("HMAC-SHA256 accepts keys of every length");
    mac.update(message);
    format!("{:x}", mac.finalize().into_bytes())
}

fn decode_sha256_hex(value: &[u8]) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut decoded = [0_u8; 32];
    for (index, pair) in value.chunks_exact(2).enumerate() {
        let high = (pair[0] as char).to_digit(16)? as u8;
        let low = (pair[1] as char).to_digit(16)? as u8;
        decoded[index] = (high << 4) | low;
    }
    Some(decoded)
}

fn hmac_sha256_verify(secret: &[u8], message: &[u8], expected_hex: &[u8]) -> bool {
    let Some(expected) = decode_sha256_hex(expected_hex) else {
        return false;
    };
    let mut mac = <HmacSha256 as Mac>::new_from_slice(secret)
        .expect("HMAC-SHA256 accepts keys of every length");
    mac.update(message);
    mac.verify_slice(&expected).is_ok()
}

fn rsa_encrypt_oaep_sha256(key: &PKey<Public>, plaintext: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    let mut encrypter = Encrypter::new(key)?;
    encrypter.set_rsa_padding(Padding::PKCS1_OAEP)?;
    encrypter.set_rsa_oaep_md(MessageDigest::sha256())?;
    encrypter.set_rsa_mgf1_md(MessageDigest::sha256())?;
    let mut ciphertext = vec![0; encrypter.encrypt_len(plaintext)?];
    let written = encrypter.encrypt(plaintext, &mut ciphertext)?;
    ciphertext.truncate(written);
    Ok(ciphertext)
}

fn rsa_decrypt_oaep_sha256(key: &PKey<Private>, ciphertext: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    let mut decrypter = Decrypter::new(key)?;
    decrypter.set_rsa_padding(Padding::PKCS1_OAEP)?;
    decrypter.set_rsa_oaep_md(MessageDigest::sha256())?;
    decrypter.set_rsa_mgf1_md(MessageDigest::sha256())?;
    let mut plaintext = vec![0; decrypter.decrypt_len(ciphertext)?];
    let written = decrypter.decrypt(ciphertext, &mut plaintext)?;
    plaintext.truncate(written);
    Ok(plaintext)
}

fn rsa_sign_sha256(key: &PKey<Private>, message: &[u8]) -> Result<Vec<u8>, ErrorStack> {
    let mut signer = Signer::new(MessageDigest::sha256(), key)?;
    signer.update(message)?;
    signer.sign_to_vec()
}

fn rsa_verify_sha256(
    key: &PKey<Public>,
    message: &[u8],
    signature: &[u8],
) -> Result<bool, ErrorStack> {
    let mut verifier = Verifier::new(MessageDigest::sha256(), key)?;
    verifier.update(message)?;
    verifier.verify(signature)
}

fn string_or_bytes(value: &Value) -> Option<&[u8]> {
    match value {
        Value::Str(value) => Some(value.as_bytes()),
        Value::Bytes(value) => Some(value.as_slice()),
        _ => None,
    }
}

fn md5_hex(bytes: &[u8]) -> String {
    let mut hasher = Md5::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn stream_temp_path(output: &Path) -> PathBuf {
    let name = output.file_name().and_then(|value| value.to_str()).unwrap_or("output");
    output.with_file_name(format!(".{}.kujo-stream-{}", name, uuid::Uuid::new_v4()))
}

fn stream_nonce(prefix: &[u8; 8], counter: u32) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(prefix);
    nonce[8..].copy_from_slice(&counter.to_be_bytes());
    nonce
}

fn stream_aad(header: &[u8], kind: u8, counter: u32, plain_len: u32) -> Vec<u8> {
    let mut aad = Vec::with_capacity(header.len() + 9);
    aad.extend_from_slice(header);
    aad.push(kind);
    aad.extend_from_slice(&counter.to_be_bytes());
    aad.extend_from_slice(&plain_len.to_be_bytes());
    aad
}

fn write_stream_frame<W: Write>(
    writer: &mut W,
    cipher: &Aes256Gcm,
    header: &[u8],
    prefix: &[u8; 8],
    kind: u8,
    counter: u32,
    plaintext: &[u8],
) -> Result<usize, String> {
    let plain_len = u32::try_from(plaintext.len())
        .map_err(|_| "stream AEAD frame exceeds u32 length".to_string())?;
    let nonce_bytes = stream_nonce(prefix, counter);
    let aad = stream_aad(header, kind, counter, plain_len);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), Payload { msg: plaintext, aad: &aad })
        .map_err(|error| format!("stream AEAD encryption failed: {}", error))?;
    writer.write_all(&[kind]).map_err(|error| error.to_string())?;
    writer.write_all(&counter.to_be_bytes()).map_err(|error| error.to_string())?;
    writer.write_all(&plain_len.to_be_bytes()).map_err(|error| error.to_string())?;
    writer.write_all(&ciphertext).map_err(|error| error.to_string())?;
    Ok(9 + ciphertext.len())
}

fn encrypt_file_stream(
    input_path: &str,
    output_path: &str,
    key: &str,
    chunk_size: usize,
) -> Result<Value, String> {
    if !(STREAM_AEAD_MIN_CHUNK..=STREAM_AEAD_MAX_CHUNK).contains(&chunk_size) {
        return Err(format!(
            "aes_encrypt_file_stream chunk size must be between {} and {} bytes",
            STREAM_AEAD_MIN_CHUNK, STREAM_AEAD_MAX_CHUNK
        ));
    }
    if Path::new(output_path).exists() {
        return Err(format!("stream AEAD output already exists: {}", output_path));
    }

    let input = File::open(input_path)
        .map_err(|error| format!("Cannot open stream AEAD input '{}': {}", input_path, error))?;
    let output = Path::new(output_path);
    let temp = stream_temp_path(output);
    let temp_file =
        OpenOptions::new().create_new(true).write(true).open(&temp).map_err(|error| {
            format!("Cannot create stream AEAD output '{}': {}", temp.display(), error)
        })?;

    let result = (|| -> Result<Value, String> {
        let mut key_hasher = Sha256::new();
        key_hasher.update(key.as_bytes());
        let key_bytes = key_hasher.finalize();
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|error| format!("Failed to create stream AEAD cipher: {}", error))?;
        let prefix: [u8; 8] = rand::random();
        let mut header = Vec::with_capacity(21);
        header.extend_from_slice(STREAM_AEAD_MAGIC);
        header.extend_from_slice(&prefix);
        header.extend_from_slice(&(chunk_size as u32).to_be_bytes());

        let mut reader = BufReader::with_capacity(chunk_size, input);
        let mut writer = BufWriter::with_capacity(chunk_size + 64, temp_file);
        writer.write_all(&header).map_err(|error| error.to_string())?;
        let mut digest = Sha256::new();
        let mut buffer = vec![0u8; chunk_size];
        let mut chunks: u32 = 0;
        let mut bytes_in: u64 = 0;
        let mut bytes_out: u64 = header.len() as u64;

        loop {
            let read = reader.read(&mut buffer).map_err(|error| error.to_string())?;
            if read == 0 {
                break;
            }
            if chunks == u32::MAX {
                return Err("stream AEAD input has too many chunks".to_string());
            }
            digest.update(&buffer[..read]);
            bytes_in += read as u64;
            bytes_out += write_stream_frame(
                &mut writer,
                &cipher,
                &header,
                &prefix,
                1,
                chunks,
                &buffer[..read],
            )? as u64;
            chunks += 1;
        }

        bytes_out +=
            write_stream_frame(&mut writer, &cipher, &header, &prefix, 0, chunks, &[])? as u64;
        writer.flush().map_err(|error| error.to_string())?;
        writer.get_ref().sync_all().map_err(|error| error.to_string())?;
        drop(writer);
        fs::rename(&temp, output).map_err(|error| {
            format!(
                "Cannot publish stream AEAD output '{}' from '{}': {}",
                output.display(),
                temp.display(),
                error
            )
        })?;

        let mut report = DictMap::default();
        report.insert("ok".into(), Value::Bool(true));
        report.insert("format".into(), Value::Str(Arc::new("KUJOAEAD1".to_string())));
        report.insert("chunk_size".into(), Value::Int(chunk_size as i64));
        report.insert("chunks".into(), Value::Int(chunks as i64));
        report.insert("bytes_in".into(), Value::Int(bytes_in as i64));
        report.insert("bytes_out".into(), Value::Int(bytes_out as i64));
        report.insert(
            "plaintext_sha256".into(),
            Value::Str(Arc::new(format!("{:x}", digest.finalize()))),
        );
        Ok(Value::Dict(Arc::new(report)))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn decrypt_file_stream(input_path: &str, output_path: &str, key: &str) -> Result<Value, String> {
    if Path::new(output_path).exists() {
        return Err(format!("stream AEAD output already exists: {}", output_path));
    }
    let input = File::open(input_path)
        .map_err(|error| format!("Cannot open stream AEAD input '{}': {}", input_path, error))?;
    let output = Path::new(output_path);
    let temp = stream_temp_path(output);
    let temp_file =
        OpenOptions::new().create_new(true).write(true).open(&temp).map_err(|error| {
            format!("Cannot create stream AEAD output '{}': {}", temp.display(), error)
        })?;

    let result = (|| -> Result<Value, String> {
        let mut reader = BufReader::new(input);
        let mut header = vec![0u8; 21];
        reader
            .read_exact(&mut header)
            .map_err(|error| format!("Invalid or truncated stream AEAD header: {}", error))?;
        if &header[..9] != STREAM_AEAD_MAGIC {
            return Err("Invalid stream AEAD magic".to_string());
        }
        let mut prefix = [0u8; 8];
        prefix.copy_from_slice(&header[9..17]);
        let chunk_size = u32::from_be_bytes(
            header[17..21].try_into().map_err(|_| "Invalid stream AEAD header".to_string())?,
        ) as usize;
        if !(STREAM_AEAD_MIN_CHUNK..=STREAM_AEAD_MAX_CHUNK).contains(&chunk_size) {
            return Err("Invalid stream AEAD chunk size".to_string());
        }

        let mut key_hasher = Sha256::new();
        key_hasher.update(key.as_bytes());
        let key_bytes = key_hasher.finalize();
        let cipher = Aes256Gcm::new_from_slice(&key_bytes)
            .map_err(|error| format!("Failed to create stream AEAD cipher: {}", error))?;
        let mut writer = BufWriter::with_capacity(chunk_size, temp_file);
        let mut digest = Sha256::new();
        let mut expected_counter: u32 = 0;
        let mut bytes_out: u64 = 0;

        loop {
            let mut frame_header = [0u8; 9];
            reader.read_exact(&mut frame_header).map_err(|error| {
                if error.kind() == ErrorKind::UnexpectedEof {
                    "Truncated stream AEAD input before authenticated final frame".to_string()
                } else {
                    error.to_string()
                }
            })?;
            let kind = frame_header[0];
            let counter = u32::from_be_bytes(frame_header[1..5].try_into().unwrap());
            let plain_len = u32::from_be_bytes(frame_header[5..9].try_into().unwrap());
            if counter != expected_counter {
                return Err(format!(
                    "Invalid stream AEAD frame order: expected {}, got {}",
                    expected_counter, counter
                ));
            }
            if kind == 1 && plain_len as usize > chunk_size {
                return Err("Stream AEAD frame exceeds declared chunk size".to_string());
            }
            if kind == 0 && plain_len != 0 {
                return Err("Invalid stream AEAD final frame length".to_string());
            }
            if kind != 0 && kind != 1 {
                return Err("Invalid stream AEAD frame type".to_string());
            }

            let cipher_len = plain_len as usize + 16;
            let mut ciphertext = vec![0u8; cipher_len];
            reader
                .read_exact(&mut ciphertext)
                .map_err(|_| "Truncated stream AEAD frame ciphertext".to_string())?;
            let nonce_bytes = stream_nonce(&prefix, counter);
            let aad = stream_aad(&header, kind, counter, plain_len);
            let plaintext = cipher
                .decrypt(Nonce::from_slice(&nonce_bytes), Payload { msg: &ciphertext, aad: &aad })
                .map_err(|_| format!("Stream AEAD authentication failed at frame {}", counter))?;

            if kind == 0 {
                if !plaintext.is_empty() {
                    return Err("Invalid stream AEAD final frame payload".to_string());
                }
                let mut trailing = [0u8; 1];
                if reader.read(&mut trailing).map_err(|error| error.to_string())? != 0 {
                    return Err("Trailing bytes after stream AEAD final frame".to_string());
                }
                break;
            }

            if plaintext.len() != plain_len as usize {
                return Err("Invalid stream AEAD plaintext length".to_string());
            }
            writer.write_all(&plaintext).map_err(|error| error.to_string())?;
            digest.update(&plaintext);
            bytes_out += plaintext.len() as u64;
            expected_counter = expected_counter
                .checked_add(1)
                .ok_or_else(|| "stream AEAD frame counter overflow".to_string())?;
        }

        writer.flush().map_err(|error| error.to_string())?;
        writer.get_ref().sync_all().map_err(|error| error.to_string())?;
        drop(writer);
        fs::rename(&temp, output).map_err(|error| {
            format!(
                "Cannot publish stream AEAD output '{}' from '{}': {}",
                output.display(),
                temp.display(),
                error
            )
        })?;

        let mut report = DictMap::default();
        report.insert("ok".into(), Value::Bool(true));
        report.insert("format".into(), Value::Str(Arc::new("KUJOAEAD1".to_string())));
        report.insert("chunk_size".into(), Value::Int(chunk_size as i64));
        report.insert("chunks".into(), Value::Int(expected_counter as i64));
        report.insert("bytes_out".into(), Value::Int(bytes_out as i64));
        report.insert(
            "plaintext_sha256".into(),
            Value::Str(Arc::new(format!("{:x}", digest.finalize()))),
        );
        Ok(Value::Dict(Arc::new(report)))
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub fn handle(name: &str, arg_values: &[Value]) -> Option<Value> {
    let result = match name {
        "sha256" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "sha256 requires a string or bytes argument".to_string(),
                ));
            }

            match arg_values.first() {
                Some(Value::Str(data)) => Value::Str(Arc::new(sha256_hex(data.as_bytes()))),
                Some(Value::Bytes(bytes)) => Value::Str(Arc::new(sha256_hex(bytes))),
                _ => Value::Error("sha256 requires a string or bytes argument".to_string()),
            }
        }

        "hmac_sha256" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "hmac_sha256 requires (secret, message) string or bytes arguments".to_string(),
                ));
            }

            match (
                arg_values.first().and_then(string_or_bytes),
                arg_values.get(1).and_then(string_or_bytes),
            ) {
                (Some(secret), Some(message)) => {
                    Value::Str(Arc::new(hmac_sha256_hex(secret, message)))
                }
                _ => Value::Error(
                    "hmac_sha256 requires (secret, message) string or bytes arguments".to_string(),
                ),
            }
        }

        "hmac_sha256_verify" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(
                    "hmac_sha256_verify requires (secret, message, expected_hex) string or bytes arguments"
                        .to_string(),
                ));
            }

            match (
                arg_values.first().and_then(string_or_bytes),
                arg_values.get(1).and_then(string_or_bytes),
                arg_values.get(2).and_then(string_or_bytes),
            ) {
                (Some(secret), Some(message), Some(expected_hex)) => {
                    Value::Bool(hmac_sha256_verify(secret, message, expected_hex))
                }
                _ => Value::Error(
                    "hmac_sha256_verify requires (secret, message, expected_hex) string or bytes arguments"
                        .to_string(),
                ),
            }
        }

        "md5" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("md5 requires a string or bytes argument".to_string()));
            }

            match arg_values.first() {
                Some(Value::Str(data)) => Value::Str(Arc::new(md5_hex(data.as_bytes()))),
                Some(Value::Bytes(bytes)) => Value::Str(Arc::new(md5_hex(bytes))),
                _ => Value::Error("md5 requires a string or bytes argument".to_string()),
            }
        }

        "sha256_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "sha256_file requires a string path argument".to_string(),
                ));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match File::open(path.as_ref())
                    .map_err(|error| error.to_string())
                    .and_then(|file| sha256_reader_range(file, 0, None))
                {
                    Ok(digest) => Value::Str(Arc::new(digest)),
                    Err(e) => {
                        error_object(format!("Failed to read file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("sha256_file requires a string path argument".to_string())
            }
        }

        "sha256_file_range" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(
                    "sha256_file_range requires path, offset, and count arguments".to_string(),
                ));
            }
            let (Some(Value::Str(path)), Some(Value::Int(offset)), Some(Value::Int(count))) =
                (arg_values.first(), arg_values.get(1), arg_values.get(2))
            else {
                return Some(Value::Error(
                    "sha256_file_range requires path (string), offset (int), and count (int)"
                        .to_string(),
                ));
            };
            if *offset < 0 || *count < 0 {
                return Some(Value::Error(
                    "sha256_file_range offset and count must be non-negative".to_string(),
                ));
            }
            match File::open(path.as_ref())
                .map_err(|error| error.to_string())
                .and_then(|file| sha256_reader_range(file, *offset as u64, Some(*count as u64)))
            {
                Ok(digest) => Value::Str(Arc::new(digest)),
                Err(error) => error_object(format!(
                    "Failed to hash {} bytes at offset {} in '{}': {}",
                    count,
                    offset,
                    path.as_ref(),
                    error
                )),
            }
        }

        "decode_file_range_info" => {
            if arg_values.len() != 6 {
                return Some(Value::Error("decode_file_range_info requires path, offset, count, encoding, max_output_bytes, and prefix_bytes arguments".to_string()));
            }
            let (
                Some(Value::Str(path)),
                Some(Value::Int(offset)),
                Some(Value::Int(count)),
                Some(Value::Str(encoding)),
                Some(Value::Int(max_output_bytes)),
                Some(Value::Int(prefix_bytes)),
            ) = (
                arg_values.first(),
                arg_values.get(1),
                arg_values.get(2),
                arg_values.get(3),
                arg_values.get(4),
                arg_values.get(5),
            )
            else {
                return Some(Value::Error(
                    "decode_file_range_info requires string, int, int, string, int, int arguments"
                        .to_string(),
                ));
            };
            if *offset < 0 || *count < 0 || *max_output_bytes < 0 || *prefix_bytes < 0 {
                return Some(Value::Error(
                    "decode_file_range_info integer arguments must be non-negative".to_string(),
                ));
            }
            match File::open(path.as_ref()).map_err(|error| error.to_string()).and_then(|file| {
                decode_file_range(
                    file,
                    *offset as u64,
                    *count as u64,
                    encoding.as_ref(),
                    *max_output_bytes as u64,
                    *prefix_bytes as usize,
                )
            }) {
                Ok(info) => info,
                Err(error) => error_object(format!(
                    "Failed to decode {} bytes at offset {} in '{}': {}",
                    count,
                    offset,
                    path.as_ref(),
                    error
                )),
            }
        }

        "sha256_canonical_text_file_range" => {
            if arg_values.len() != 4 {
                return Some(Value::Error(
                    "sha256_canonical_text_file_range requires path, offset, count, and mode arguments".to_string(),
                ));
            }
            let (
                Some(Value::Str(path)),
                Some(Value::Int(offset)),
                Some(Value::Int(count)),
                Some(Value::Str(mode)),
            ) = (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3))
            else {
                return Some(Value::Error(
                    "sha256_canonical_text_file_range requires path (string), offset (int), count (int), and mode (string)".to_string(),
                ));
            };
            if *offset < 0 || *count < 0 {
                return Some(Value::Error(
                    "sha256_canonical_text_file_range offset and count must be non-negative"
                        .to_string(),
                ));
            }
            match File::open(path.as_ref()).map_err(|error| error.to_string()).and_then(|file| {
                sha256_canonical_text_reader_range(
                    file,
                    *offset as u64,
                    *count as u64,
                    mode.as_ref(),
                )
            }) {
                Ok(digest) => Value::Str(Arc::new(digest)),
                Err(error) => error_object(format!(
                    "Failed to canonically hash {} bytes at offset {} in '{}': {}",
                    count,
                    offset,
                    path.as_ref(),
                    error
                )),
            }
        }

        "md5_file" => {
            if arg_values.len() != 1 {
                return Some(Value::Error("md5_file requires a string path argument".to_string()));
            }

            if let Some(Value::Str(path)) = arg_values.first() {
                match std::fs::read(path.as_ref()) {
                    Ok(contents) => Value::Str(Arc::new(md5_hex(&contents))),
                    Err(e) => {
                        error_object(format!("Failed to read file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("md5_file requires a string path argument".to_string())
            }
        }

        "hash_password" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "hash_password requires a string password argument".to_string(),
                ));
            }

            if let Some(Value::Str(password)) = arg_values.first() {
                match bcrypt::hash(password.as_ref(), bcrypt::DEFAULT_COST) {
                    Ok(hashed) => Value::Str(Arc::new(hashed)),
                    Err(e) => error_object(format!("Failed to hash password: {}", e)),
                }
            } else {
                Value::Error("hash_password requires a string password argument".to_string())
            }
        }

        "verify_password" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "verify_password requires (string_password, string_hash) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(password)), Some(Value::Str(hash))) => {
                    match bcrypt::verify(password.as_ref(), hash.as_ref()) {
                        Ok(is_valid) => Value::Bool(is_valid),
                        Err(e) => error_object(format!("Failed to verify password: {}", e)),
                    }
                }
                _ => Value::Error(
                    "verify_password requires (string_password, string_hash) arguments".to_string(),
                ),
            }
        }

        "aes_encrypt" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "aes_encrypt requires (plaintext_string, key_string) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(plaintext)), Some(Value::Str(key))) => {
                    let mut hasher = Sha256::new();
                    hasher.update(key.as_bytes());
                    let key_bytes = hasher.finalize();

                    let nonce_bytes: [u8; 12] = rand::random();
                    let nonce = Nonce::from_slice(&nonce_bytes);

                    match Aes256Gcm::new_from_slice(&key_bytes) {
                        Ok(cipher) => match cipher.encrypt(nonce, plaintext.as_bytes()) {
                            Ok(ciphertext) => {
                                let mut result = nonce_bytes.to_vec();
                                result.extend_from_slice(&ciphertext);
                                Value::Str(Arc::new(
                                    base64::engine::general_purpose::STANDARD.encode(result),
                                ))
                            }
                            Err(e) => error_object(format!("AES encryption failed: {}", e)),
                        },
                        Err(e) => error_object(format!("Failed to create AES cipher: {}", e)),
                    }
                }
                _ => Value::Error(
                    "aes_encrypt requires (plaintext_string, key_string) arguments".to_string(),
                ),
            }
        }

        "aes_decrypt" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "aes_decrypt requires (ciphertext_string, key_string) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(ciphertext_b64)), Some(Value::Str(key))) => {
                    let mut hasher = Sha256::new();
                    hasher.update(key.as_bytes());
                    let key_bytes = hasher.finalize();

                    match base64::engine::general_purpose::STANDARD.decode(ciphertext_b64.as_ref())
                    {
                        Ok(data) => {
                            if data.len() < 12 {
                                return Some(Value::Error(
                                    "Invalid ciphertext: too short".to_string(),
                                ));
                            }

                            let nonce = Nonce::from_slice(&data[..12]);
                            let ciphertext = &data[12..];

                            match Aes256Gcm::new_from_slice(&key_bytes) {
                                Ok(cipher) => match cipher.decrypt(nonce, ciphertext) {
                                    Ok(plaintext) => match String::from_utf8(plaintext) {
                                        Ok(s) => Value::Str(Arc::new(s)),
                                        Err(e) => error_object(format!(
                                            "Decrypted data is not valid UTF-8: {}",
                                            e
                                        )),
                                    },
                                    Err(e) => error_object(format!("AES decryption failed: {}", e)),
                                },
                                Err(e) => {
                                    error_object(format!("Failed to create AES cipher: {}", e))
                                }
                            }
                        }
                        Err(e) => error_object(format!("Invalid base64 ciphertext: {}", e)),
                    }
                }
                _ => Value::Error(
                    "aes_decrypt requires (ciphertext_string, key_string) arguments".to_string(),
                ),
            }
        }

        "aes_encrypt_bytes" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "aes_encrypt_bytes requires (data_string, key_string) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(data)), Some(Value::Str(key))) => {
                    let mut hasher = Sha256::new();
                    hasher.update(key.as_bytes());
                    let key_bytes = hasher.finalize();

                    let nonce_bytes: [u8; 12] = rand::random();
                    let nonce = Nonce::from_slice(&nonce_bytes);

                    match Aes256Gcm::new_from_slice(&key_bytes) {
                        Ok(cipher) => match cipher.encrypt(nonce, data.as_bytes()) {
                            Ok(ciphertext) => {
                                let mut result = nonce_bytes.to_vec();
                                result.extend_from_slice(&ciphertext);
                                Value::Str(Arc::new(
                                    base64::engine::general_purpose::STANDARD.encode(result),
                                ))
                            }
                            Err(e) => error_object(format!("AES encryption failed: {}", e)),
                        },
                        Err(e) => error_object(format!("Failed to create AES cipher: {}", e)),
                    }
                }
                _ => Value::Error(
                    "aes_encrypt_bytes requires (data_string, key_string) arguments".to_string(),
                ),
            }
        }

        "aes_decrypt_bytes" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "aes_decrypt_bytes requires (ciphertext_string, key_string) arguments"
                        .to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(ciphertext_b64)), Some(Value::Str(key))) => {
                    let mut hasher = Sha256::new();
                    hasher.update(key.as_bytes());
                    let key_bytes = hasher.finalize();

                    match base64::engine::general_purpose::STANDARD.decode(ciphertext_b64.as_ref())
                    {
                        Ok(data) => {
                            if data.len() < 12 {
                                return Some(Value::Error(
                                    "Invalid ciphertext: too short".to_string(),
                                ));
                            }

                            let nonce = Nonce::from_slice(&data[..12]);
                            let ciphertext = &data[12..];

                            match Aes256Gcm::new_from_slice(&key_bytes) {
                                Ok(cipher) => match cipher.decrypt(nonce, ciphertext) {
                                    Ok(plaintext) => match String::from_utf8(plaintext.clone()) {
                                        Ok(s) => Value::Str(Arc::new(s)),
                                        Err(_) => Value::Str(Arc::new(
                                            base64::engine::general_purpose::STANDARD
                                                .encode(&plaintext),
                                        )),
                                    },
                                    Err(e) => error_object(format!("AES decryption failed: {}", e)),
                                },
                                Err(e) => {
                                    error_object(format!("Failed to create AES cipher: {}", e))
                                }
                            }
                        }
                        Err(e) => error_object(format!("Invalid base64 ciphertext: {}", e)),
                    }
                }
                _ => Value::Error(
                    "aes_decrypt_bytes requires (ciphertext_string, key_string) arguments"
                        .to_string(),
                ),
            }
        }

        "aes_encrypt_file_stream" => {
            if arg_values.len() != 4 {
                return Some(Value::Error(
                    "aes_encrypt_file_stream requires (input_path, output_path, key, chunk_size) arguments"
                        .to_string(),
                ));
            }
            match (arg_values.first(), arg_values.get(1), arg_values.get(2), arg_values.get(3)) {
                (
                    Some(Value::Str(input)),
                    Some(Value::Str(output)),
                    Some(Value::Str(key)),
                    Some(Value::Int(chunk_size)),
                ) if *chunk_size >= 0 => match encrypt_file_stream(
                    input.as_ref(),
                    output.as_ref(),
                    key.as_ref(),
                    *chunk_size as usize,
                ) {
                    Ok(value) => value,
                    Err(error) => error_object(error),
                },
                _ => Value::Error(
                    "aes_encrypt_file_stream requires string paths/key and integer chunk_size"
                        .to_string(),
                ),
            }
        }

        "aes_decrypt_file_stream" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(
                    "aes_decrypt_file_stream requires (input_path, output_path, key) arguments"
                        .to_string(),
                ));
            }
            match (arg_values.first(), arg_values.get(1), arg_values.get(2)) {
                (Some(Value::Str(input)), Some(Value::Str(output)), Some(Value::Str(key))) => {
                    match decrypt_file_stream(input.as_ref(), output.as_ref(), key.as_ref()) {
                        Ok(value) => value,
                        Err(error) => error_object(error),
                    }
                }
                _ => Value::Error(
                    "aes_decrypt_file_stream requires string input_path, output_path, and key"
                        .to_string(),
                ),
            }
        }

        "rsa_generate_keypair" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "rsa_generate_keypair requires an integer (2048 or 4096)".to_string(),
                ));
            }

            if let Some(Value::Int(bits)) = arg_values.first() {
                let bits_usize = *bits as usize;
                if bits_usize != 2048 && bits_usize != 4096 {
                    return Some(Value::Error(
                        "RSA key size must be 2048 or 4096 bits".to_string(),
                    ));
                }

                match Rsa::generate(bits_usize as u32)
                    .and_then(PKey::from_rsa)
                    .and_then(|key| Ok((key.private_key_to_pem_pkcs8()?, key.public_key_to_pem()?)))
                {
                    Ok((private_pem, public_pem)) => {
                        let private_pem = match String::from_utf8(private_pem) {
                            Ok(pem) => pem,
                            Err(e) => {
                                return Some(error_object(format!(
                                    "Failed to encode private key: {}",
                                    e
                                )))
                            }
                        };
                        let public_pem = match String::from_utf8(public_pem) {
                            Ok(pem) => pem,
                            Err(e) => {
                                return Some(error_object(format!(
                                    "Failed to encode public key: {}",
                                    e
                                )))
                            }
                        };

                        let mut keypair = DictMap::default();
                        keypair
                            .insert(Arc::<str>::from("private"), Value::Str(Arc::new(private_pem)));
                        keypair
                            .insert(Arc::<str>::from("public"), Value::Str(Arc::new(public_pem)));
                        Value::Dict(Arc::new(keypair))
                    }
                    Err(e) => error_object(format!("Failed to generate RSA keypair: {}", e)),
                }
            } else {
                Value::Error("rsa_generate_keypair requires an integer (2048 or 4096)".to_string())
            }
        }

        "rsa_public_key_info" => {
            if arg_values.len() != 1 {
                return Some(Value::Error(
                    "rsa_public_key_info requires a PEM string or DER bytes argument".to_string(),
                ));
            }
            match rsa_public_key_info(&arg_values[0]) {
                Ok(value) => value,
                Err(error) => error_object(error),
            }
        }

        "rsa_encrypt" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "rsa_encrypt requires (plaintext_string, public_key_pem) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(plaintext)), Some(Value::Str(public_key_pem))) => {
                    match PKey::public_key_from_pem(public_key_pem.as_bytes()) {
                        Ok(public_key) => {
                            match rsa_encrypt_oaep_sha256(&public_key, plaintext.as_bytes()) {
                                Ok(ciphertext) => Value::Str(Arc::new(
                                    base64::engine::general_purpose::STANDARD.encode(ciphertext),
                                )),
                                Err(e) => error_object(format!("RSA encryption failed: {}", e)),
                            }
                        }
                        Err(e) => error_object(format!("Invalid RSA public key: {}", e)),
                    }
                }
                _ => Value::Error(
                    "rsa_encrypt requires (plaintext_string, public_key_pem) arguments".to_string(),
                ),
            }
        }

        "rsa_decrypt" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "rsa_decrypt requires (ciphertext_string, private_key_pem) arguments"
                        .to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(ciphertext_b64)), Some(Value::Str(private_key_pem))) => {
                    match PKey::private_key_from_pem(private_key_pem.as_bytes()) {
                        Ok(private_key) => {
                            match base64::engine::general_purpose::STANDARD
                                .decode(ciphertext_b64.as_ref())
                            {
                                Ok(ciphertext) => {
                                    match rsa_decrypt_oaep_sha256(&private_key, &ciphertext) {
                                        Ok(plaintext) => match String::from_utf8(plaintext) {
                                            Ok(s) => Value::Str(Arc::new(s)),
                                            Err(e) => error_object(format!(
                                                "Decrypted data is not valid UTF-8: {}",
                                                e
                                            )),
                                        },
                                        Err(e) => {
                                            error_object(format!("RSA decryption failed: {}", e))
                                        }
                                    }
                                }
                                Err(e) => error_object(format!("Invalid base64 ciphertext: {}", e)),
                            }
                        }
                        Err(e) => error_object(format!("Invalid RSA private key: {}", e)),
                    }
                }
                _ => Value::Error(
                    "rsa_decrypt requires (ciphertext_string, private_key_pem) arguments"
                        .to_string(),
                ),
            }
        }

        "rsa_sign" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "rsa_sign requires (message_string, private_key_pem) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(message)), Some(Value::Str(private_key_pem))) => {
                    match PKey::private_key_from_pem(private_key_pem.as_bytes()) {
                        Ok(private_key) => {
                            match rsa_sign_sha256(&private_key, message.as_bytes()) {
                                Ok(signature) => Value::Str(Arc::new(
                                    base64::engine::general_purpose::STANDARD.encode(signature),
                                )),
                                Err(e) => error_object(format!("RSA signing failed: {}", e)),
                            }
                        }
                        Err(e) => error_object(format!("Invalid RSA private key: {}", e)),
                    }
                }
                _ => Value::Error(
                    "rsa_sign requires (message_string, private_key_pem) arguments".to_string(),
                ),
            }
        }

        "rsa_verify" => {
            if arg_values.len() != 3 {
                return Some(Value::Error(
                    "rsa_verify requires (message, signature, public_key_pem) arguments"
                        .to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1), arg_values.get(2)) {
                (
                    Some(Value::Str(message)),
                    Some(Value::Str(signature_b64)),
                    Some(Value::Str(public_key_pem)),
                ) => match PKey::public_key_from_pem(public_key_pem.as_bytes()) {
                    Ok(public_key) => match base64::engine::general_purpose::STANDARD
                        .decode(signature_b64.as_ref())
                    {
                        Ok(signature_bytes) => {
                            let expected_len = (public_key.bits() as usize).div_ceil(8);
                            if signature_bytes.len() != expected_len {
                                error_object(format!(
                                    "Invalid signature format: expected {} bytes, got {}",
                                    expected_len,
                                    signature_bytes.len()
                                ))
                            } else {
                                match rsa_verify_sha256(
                                    &public_key,
                                    message.as_bytes(),
                                    &signature_bytes,
                                ) {
                                    Ok(valid) => Value::Bool(valid),
                                    Err(e) => {
                                        error_object(format!("RSA verification failed: {}", e))
                                    }
                                }
                            }
                        }
                        Err(e) => error_object(format!("Invalid base64 signature: {}", e)),
                    },
                    Err(e) => error_object(format!("Invalid RSA public key: {}", e)),
                },
                _ => Value::Error(
                    "rsa_verify requires (message, signature, public_key_pem) arguments"
                        .to_string(),
                ),
            }
        }

        _ => return None,
    };

    Some(result)
}

#[cfg(test)]
mod tests {
    use super::{handle, sha256_hex};
    use crate::interpreter::Value;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn string_value(value: &str) -> Value {
        Value::Str(Arc::new(value.to_string()))
    }

    fn unique_temp_file(prefix: &str) -> String {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("{}_{}.txt", prefix, nanos));
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_sha256_and_md5_hashes_match_known_values() {
        let sha = handle("sha256", &[string_value("kujo")]).unwrap();
        assert!(
            matches!(sha, Value::Str(value) if value.as_ref() == "cc89e49d76db3627520f5bd923995954a08f0bd5885b56cb99fc68c98e6ff7d1")
        );

        let sha_bytes = handle("sha256", &[Value::Bytes(b"kujo".to_vec())]).unwrap();
        assert!(
            matches!(sha_bytes, Value::Str(value) if value.as_ref() == "cc89e49d76db3627520f5bd923995954a08f0bd5885b56cb99fc68c98e6ff7d1")
        );

        let md5 = handle("md5", &[string_value("kujo")]).unwrap();
        assert!(
            matches!(md5, Value::Str(value) if value.as_ref() == "4865bdebde111b22232a5e509bc58c75")
        );

        let md5_bytes = handle("md5", &[Value::Bytes(b"kujo".to_vec())]).unwrap();
        assert!(
            matches!(md5_bytes, Value::Str(value) if value.as_ref() == "4865bdebde111b22232a5e509bc58c75")
        );
    }

    #[test]
    fn test_hmac_sha256_matches_known_vectors_for_strings_and_bytes() {
        let string_result = handle(
            "hmac_sha256",
            &[string_value("key"), string_value("The quick brown fox jumps over the lazy dog")],
        )
        .unwrap();
        assert!(
            matches!(string_result, Value::Str(value) if value.as_ref() == "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8")
        );

        let bytes_result = handle(
            "hmac_sha256",
            &[Value::Bytes(vec![0x0b; 20]), Value::Bytes(b"Hi There".to_vec())],
        )
        .unwrap();
        assert!(
            matches!(bytes_result, Value::Str(value) if value.as_ref() == "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7")
        );

        let verified = handle(
            "hmac_sha256_verify",
            &[
                string_value("key"),
                string_value("The quick brown fox jumps over the lazy dog"),
                string_value("f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"),
            ],
        )
        .unwrap();
        assert!(matches!(verified, Value::Bool(true)));

        let tampered = handle(
            "hmac_sha256_verify",
            &[
                string_value("key"),
                string_value("The quick brown fox jumps over the lazy dog"),
                string_value("07bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"),
            ],
        )
        .unwrap();
        assert!(matches!(tampered, Value::Bool(false)));

        let malformed = handle(
            "hmac_sha256_verify",
            &[string_value("key"), string_value("message"), string_value("not-hex")],
        )
        .unwrap();
        assert!(matches!(malformed, Value::Bool(false)));
    }

    #[test]
    fn test_md5_file_hashes_file_contents() {
        let path = unique_temp_file("kujo_crypto_md5_file");
        fs::write(&path, "kujo-file-hash").unwrap();

        let result = handle("md5_file", &[string_value(&path)]).unwrap();
        assert!(matches!(result, Value::Str(_)));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_sha256_file_hashes_binary_contents() {
        let path = unique_temp_file("kujo_crypto_sha256_file");
        fs::write(&path, [0u8, 1, 2, 0x80, 0xFF, 0]).unwrap();

        let result = handle("sha256_file", &[string_value(&path)]).unwrap();
        assert!(
            matches!(result, Value::Str(value) if value.as_ref() == "5b35354055af6a5442460fc80a36f4c47cf5fe7cade16773e1c474a2d37e9a3d")
        );

        let direct = handle("sha256", &[Value::Bytes(vec![0, 1, 2, 0x80, 0xFF, 0])]).unwrap();
        assert!(
            matches!(direct, Value::Str(value) if value.as_ref() == "5b35354055af6a5442460fc80a36f4c47cf5fe7cade16773e1c474a2d37e9a3d")
        );

        let range =
            handle("sha256_file_range", &[string_value(&path), Value::Int(1), Value::Int(3)])
                .unwrap();
        assert!(
            matches!(range, Value::Str(value) if value.as_ref() == "ace363ce5f10b61a6f5ec523b0717cffa1d98857cb6b19be4651339e0316ebe9")
        );

        let beyond_eof =
            handle("sha256_file_range", &[string_value(&path), Value::Int(5), Value::Int(2)])
                .unwrap();
        assert!(matches!(beyond_eof, Value::ErrorObject { message, .. }
            if message.contains("exceeds file length")));

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_decode_file_range_info_streams_strict_transfer_encodings() {
        let base64_path = unique_temp_file("kujo_crypto_decode_base64");
        fs::write(&base64_path, b"SGVsbG8s\r\nIHdvcmxkIQ==\r\n").unwrap();
        let base64 = handle(
            "decode_file_range_info",
            &[
                string_value(&base64_path),
                Value::Int(0),
                Value::Int(24),
                string_value("base64"),
                Value::Int(64),
                Value::Int(5),
            ],
        )
        .unwrap();
        let Value::Dict(base64) = base64 else { panic!("expected base64 decode metadata") };
        assert!(
            matches!(base64.get("schema"), Some(Value::Str(value)) if value.as_ref() == "kujo.file.decode.v1")
        );
        assert!(matches!(base64.get("output_bytes"), Some(Value::Int(13))));
        assert!(matches!(base64.get("prefix"), Some(Value::Bytes(value)) if value == b"Hello"));
        assert!(matches!(base64.get("ascii"), Some(Value::Bool(true))));
        assert!(matches!(base64.get("utf8_valid"), Some(Value::Bool(true))));
        assert!(matches!(base64.get("max_input_line_bytes"), Some(Value::Int(12))));
        assert!(
            matches!(base64.get("sha256"), Some(Value::Str(value)) if value.as_ref() == "315f5bdb76d078c43b8ac0064e4a0164612b1fce77c869345bfc94c75894edd3")
        );

        let qp_path = unique_temp_file("kujo_crypto_decode_qp");
        fs::write(&qp_path, b"caf=C3=A9=\r\n!\r\n").unwrap();
        let qp = handle(
            "decode_file_range_info",
            &[
                string_value(&qp_path),
                Value::Int(0),
                Value::Int(15),
                string_value("quoted-printable"),
                Value::Int(64),
                Value::Int(16),
            ],
        )
        .unwrap();
        let Value::Dict(qp) = qp else { panic!("expected quoted-printable decode metadata") };
        assert!(matches!(qp.get("output_bytes"), Some(Value::Int(8))));
        assert!(
            matches!(qp.get("prefix"), Some(Value::Bytes(value)) if value == b"caf\xc3\xa9!\r\n")
        );
        assert!(matches!(qp.get("ascii"), Some(Value::Bool(false))));
        assert!(matches!(qp.get("utf8_valid"), Some(Value::Bool(true))));

        let binary_path = unique_temp_file("kujo_crypto_decode_binary");
        fs::write(&binary_path, [0_u8, 0xff, b'A']).unwrap();
        let binary = handle(
            "decode_file_range_info",
            &[
                string_value(&binary_path),
                Value::Int(0),
                Value::Int(3),
                string_value("identity"),
                Value::Int(3),
                Value::Int(3),
            ],
        )
        .unwrap();
        let Value::Dict(binary) = binary else { panic!("expected identity decode metadata") };
        assert!(matches!(binary.get("contains_nul"), Some(Value::Bool(true))));
        assert!(matches!(binary.get("utf8_valid"), Some(Value::Bool(false))));

        let invalid_path = unique_temp_file("kujo_crypto_decode_invalid");
        fs::write(&invalid_path, b"AAAA=bad").unwrap();
        let invalid = handle(
            "decode_file_range_info",
            &[
                string_value(&invalid_path),
                Value::Int(0),
                Value::Int(8),
                string_value("base64"),
                Value::Int(64),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(invalid, Value::ErrorObject { message, .. } if message.contains("invalid padding or quartet"))
        );

        fs::write(&invalid_path, b"unsafe \r\n").unwrap();
        let invalid_qp = handle(
            "decode_file_range_info",
            &[
                string_value(&invalid_path),
                Value::Int(0),
                Value::Int(9),
                string_value("quoted-printable"),
                Value::Int(64),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(invalid_qp, Value::ErrorObject { message, .. } if message.contains("trailing whitespace"))
        );

        fs::write(&invalid_path, [b'A', 0_u8]).unwrap();
        let invalid_qp_octet = handle(
            "decode_file_range_info",
            &[
                string_value(&invalid_path),
                Value::Int(0),
                Value::Int(2),
                string_value("quoted-printable"),
                Value::Int(64),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(invalid_qp_octet, Value::ErrorObject { message, .. } if message.contains("unencoded non-printable"))
        );

        let capped = handle(
            "decode_file_range_info",
            &[
                string_value(&base64_path),
                Value::Int(0),
                Value::Int(24),
                string_value("base64"),
                Value::Int(12),
                Value::Int(0),
            ],
        )
        .unwrap();
        assert!(
            matches!(capped, Value::ErrorObject { message, .. } if message.contains("configured limit"))
        );

        for path in [base64_path, qp_path, binary_path, invalid_path] {
            let _ = fs::remove_file(path);
        }
    }

    #[test]
    fn test_sha256_canonical_text_file_range_is_streaming_and_strict() {
        let path = unique_temp_file("kujo_crypto_canonical_text");
        let input = b"prefix C \t\r\nD \t E\r\n\r\n\r\nsuffix";
        fs::write(&path, input).unwrap();
        let offset = 6_i64;
        let count = (input.len() - 12) as i64;

        let relaxed = handle(
            "sha256_canonical_text_file_range",
            &[
                string_value(&path),
                Value::Int(offset),
                Value::Int(count),
                string_value("relaxed-crlf"),
            ],
        )
        .unwrap();
        assert!(matches!(relaxed, Value::Str(value)
            if value.as_ref() == &sha256_hex(b" C\r\nD E\r\n")));

        let simple = handle(
            "sha256_canonical_text_file_range",
            &[
                string_value(&path),
                Value::Int(offset),
                Value::Int(count),
                string_value("simple-crlf"),
            ],
        )
        .unwrap();
        assert!(matches!(simple, Value::Str(value)
            if value.as_ref() == &sha256_hex(b" C \t\r\nD \t E\r\n")));

        fs::write(&path, b"bare\nline").unwrap();
        let invalid = handle(
            "sha256_canonical_text_file_range",
            &[string_value(&path), Value::Int(0), Value::Int(9), string_value("relaxed-crlf")],
        )
        .unwrap();
        assert!(matches!(invalid, Value::ErrorObject { message, .. }
            if message.contains("bare line feed")));

        fs::write(&path, b"").unwrap();
        let relaxed_empty = handle(
            "sha256_canonical_text_file_range",
            &[string_value(&path), Value::Int(0), Value::Int(0), string_value("relaxed-crlf")],
        )
        .unwrap();
        assert!(matches!(relaxed_empty, Value::Str(value)
            if value.as_ref() == &sha256_hex(b"")));
        let simple_empty = handle(
            "sha256_canonical_text_file_range",
            &[string_value(&path), Value::Int(0), Value::Int(0), string_value("simple-crlf")],
        )
        .unwrap();
        assert!(matches!(simple_empty, Value::Str(value)
            if value.as_ref() == &sha256_hex(b"\r\n")));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn test_rsa_public_key_info_accepts_pem_and_der() {
        let rsa = Rsa::generate(2048).unwrap();
        let key = PKey::from_rsa(rsa).unwrap();
        let pem = String::from_utf8(key.public_key_to_pem().unwrap()).unwrap();
        let der = key.public_key_to_der().unwrap();
        for input in [Value::Str(Arc::new(pem)), Value::Bytes(der)] {
            let result = handle("rsa_public_key_info", &[input]).unwrap();
            let Value::Dict(info) = result else { panic!("expected public key info") };
            assert!(
                matches!(info.get("algorithm"), Some(Value::Str(value)) if value.as_ref() == "rsa")
            );
            assert!(matches!(info.get("bits"), Some(Value::Int(2048))));
            assert!(
                matches!(info.get("pem"), Some(Value::Str(value)) if value.contains("BEGIN PUBLIC KEY"))
            );
            assert!(matches!(info.get("sha256"), Some(Value::Str(value)) if value.len() == 64));
        }
    }

    #[test]
    fn test_hash_password_and_verify_password_round_trip() {
        let password = "hardening-secret";
        let hashed = handle("hash_password", &[string_value(password)]).unwrap();

        let hash_string = match hashed {
            Value::Str(value) => value,
            other => panic!("Expected Value::Str hash, got {:?}", other),
        };

        let verify_ok =
            handle("verify_password", &[string_value(password), Value::Str(hash_string.clone())])
                .unwrap();
        assert!(matches!(verify_ok, Value::Bool(true)));

        let verify_fail =
            handle("verify_password", &[string_value("wrong"), Value::Str(hash_string)]).unwrap();
        assert!(matches!(verify_fail, Value::Bool(false)));
    }

    #[test]
    fn test_aes_encrypt_and_decrypt_round_trip() {
        let plaintext = "kujo-aes-roundtrip";
        let key = "key-material";

        let encrypted =
            handle("aes_encrypt", &[string_value(plaintext), string_value(key)]).unwrap();
        let ciphertext = match encrypted {
            Value::Str(value) => value,
            other => panic!("Expected ciphertext string, got {:?}", other),
        };

        let decrypted =
            handle("aes_decrypt", &[Value::Str(ciphertext), string_value(key)]).unwrap();
        assert!(matches!(decrypted, Value::Str(value) if value.as_ref() == plaintext));
    }

    #[test]
    fn test_aes_encrypt_bytes_and_decrypt_bytes_round_trip() {
        let payload = "binary-payload";
        let key = "key-material";

        let encrypted =
            handle("aes_encrypt_bytes", &[string_value(payload), string_value(key)]).unwrap();
        let ciphertext = match encrypted {
            Value::Str(value) => value,
            other => panic!("Expected ciphertext string, got {:?}", other),
        };

        let decrypted =
            handle("aes_decrypt_bytes", &[Value::Str(ciphertext), string_value(key)]).unwrap();
        assert!(matches!(decrypted, Value::Str(value) if value.as_ref() == payload));
    }

    #[test]
    fn test_stream_aead_file_round_trip_and_tamper_rejection() {
        let input = unique_temp_file("kujo_stream_aead_input");
        let encrypted = unique_temp_file("kujo_stream_aead_encrypted");
        let decrypted = unique_temp_file("kujo_stream_aead_decrypted");
        let rejected = unique_temp_file("kujo_stream_aead_rejected");
        let _ = fs::remove_file(&encrypted);
        let _ = fs::remove_file(&decrypted);
        let _ = fs::remove_file(&rejected);
        let mut payload = Vec::with_capacity(12_500);
        for index in 0..12_500 {
            payload.push((index % 251) as u8);
        }
        fs::write(&input, &payload).expect("stream input should be written");

        let encrypted_result = handle(
            "aes_encrypt_file_stream",
            &[
                string_value(&input),
                string_value(&encrypted),
                string_value("stream-key"),
                Value::Int(4096),
            ],
        )
        .expect("stream encrypt should be handled");
        assert!(matches!(encrypted_result, Value::Dict(ref fields)
            if matches!(fields.get("ok"), Some(Value::Bool(true)))
                && matches!(fields.get("chunks"), Some(Value::Int(4)))));

        let decrypted_result = handle(
            "aes_decrypt_file_stream",
            &[string_value(&encrypted), string_value(&decrypted), string_value("stream-key")],
        )
        .expect("stream decrypt should be handled");
        assert!(matches!(decrypted_result, Value::Dict(ref fields)
            if matches!(fields.get("ok"), Some(Value::Bool(true)))
                && matches!(fields.get("chunks"), Some(Value::Int(4)))));
        assert_eq!(fs::read(&decrypted).expect("decrypted output should exist"), payload);

        let mut tampered = fs::read(&encrypted).expect("encrypted file should be readable");
        let tamper_index = tampered.len() / 2;
        tampered[tamper_index] ^= 0x80;
        fs::write(&encrypted, tampered).expect("tampered file should be written");
        let rejected_result = handle(
            "aes_decrypt_file_stream",
            &[string_value(&encrypted), string_value(&rejected), string_value("stream-key")],
        )
        .expect("stream decrypt should be handled");
        assert!(matches!(rejected_result, Value::ErrorObject { message, .. }
            if message.contains("authentication failed")));
        assert!(!std::path::Path::new(&rejected).exists());

        let _ = fs::remove_file(input);
        let _ = fs::remove_file(encrypted);
        let _ = fs::remove_file(decrypted);
    }

    #[test]
    fn test_rsa_generate_encrypt_decrypt_sign_and_verify() {
        let keypair = handle("rsa_generate_keypair", &[Value::Int(2048)]).unwrap();

        let (private_pem, public_pem) = match keypair {
            Value::Dict(map) => {
                let private = match map.get("private") {
                    Some(Value::Str(value)) => value.clone(),
                    other => panic!("Expected private PEM string, got {:?}", other),
                };
                let public = match map.get("public") {
                    Some(Value::Str(value)) => value.clone(),
                    other => panic!("Expected public PEM string, got {:?}", other),
                };
                (private, public)
            }
            other => panic!("Expected keypair dict, got {:?}", other),
        };
        assert!(private_pem.starts_with("-----BEGIN PRIVATE KEY-----"));
        assert!(public_pem.starts_with("-----BEGIN PUBLIC KEY-----"));

        let message = "kujo-rsa-contract";

        let encrypted =
            handle("rsa_encrypt", &[string_value(message), Value::Str(public_pem.clone())])
                .unwrap();
        let ciphertext = match encrypted {
            Value::Str(value) => value,
            other => panic!("Expected RSA ciphertext string, got {:?}", other),
        };

        let decrypted =
            handle("rsa_decrypt", &[Value::Str(ciphertext), Value::Str(private_pem.clone())])
                .unwrap();
        assert!(matches!(decrypted, Value::Str(value) if value.as_ref() == message));

        let signature =
            handle("rsa_sign", &[string_value(message), Value::Str(private_pem)]).unwrap();
        let signature_b64 = match signature {
            Value::Str(value) => value,
            other => panic!("Expected RSA signature string, got {:?}", other),
        };

        let verified = handle(
            "rsa_verify",
            &[
                string_value(message),
                Value::Str(signature_b64.clone()),
                Value::Str(public_pem.clone()),
            ],
        )
        .unwrap();
        assert!(matches!(verified, Value::Bool(true)));

        let tampered = handle(
            "rsa_verify",
            &[string_value("tampered"), Value::Str(signature_b64), Value::Str(public_pem.clone())],
        )
        .unwrap();
        assert!(matches!(tampered, Value::Bool(false)));

        let malformed_signature = handle(
            "rsa_verify",
            &[string_value(message), string_value("AA=="), Value::Str(public_pem)],
        )
        .unwrap();
        assert!(matches!(malformed_signature, Value::ErrorObject { message, .. }
            if message.contains("Invalid signature format")));
    }

    #[test]
    fn test_rsa_rustcrypto_0_9_fixture_remains_compatible() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/crypto/rustcrypto_rsa_0_9.json"
        ))
        .expect("legacy RSA fixture should be valid JSON");
        let value = |name: &str| {
            fixture[name]
                .as_str()
                .unwrap_or_else(|| panic!("legacy RSA fixture should contain {name}"))
        };

        let decrypted = handle(
            "rsa_decrypt",
            &[
                string_value(value("oaep_sha256_ciphertext_base64")),
                string_value(value("private_key_pkcs8_pem")),
            ],
        )
        .unwrap();
        assert!(matches!(decrypted, Value::Str(message) if message.as_ref() == value("message")));

        let verified = handle(
            "rsa_verify",
            &[
                string_value(value("message")),
                string_value(value("pkcs1v15_sha256_signature_base64")),
                string_value(value("public_key_spki_pem")),
            ],
        )
        .unwrap();
        assert!(matches!(verified, Value::Bool(true)));
    }

    #[test]
    fn test_crypto_argument_validation_contracts() {
        let sha_missing = handle("sha256", &[]).unwrap();
        assert!(
            matches!(sha_missing, Value::Error(message) if message.contains("sha256 requires a string or bytes argument"))
        );

        let sha_extra = handle("sha256", &[string_value("data"), string_value("extra")]).unwrap();
        assert!(
            matches!(sha_extra, Value::Error(message) if message.contains("sha256 requires a string or bytes argument"))
        );

        let hmac_missing = handle("hmac_sha256", &[string_value("secret")]).unwrap();
        assert!(
            matches!(hmac_missing, Value::Error(message) if message.contains("hmac_sha256 requires"))
        );

        let hmac_bad_type =
            handle("hmac_sha256", &[Value::Int(1), string_value("message")]).unwrap();
        assert!(
            matches!(hmac_bad_type, Value::Error(message) if message.contains("hmac_sha256 requires"))
        );

        let sha_file_missing = handle("sha256_file", &[]).unwrap();
        assert!(
            matches!(sha_file_missing, Value::Error(message) if message.contains("sha256_file requires a string path argument"))
        );

        let verify_missing = handle("verify_password", &[string_value("only_one")]).unwrap();
        assert!(
            matches!(verify_missing, Value::Error(message) if message.contains("verify_password requires"))
        );

        let verify_extra = handle(
            "verify_password",
            &[string_value("pw"), string_value("hash"), string_value("extra")],
        )
        .unwrap();
        assert!(
            matches!(verify_extra, Value::Error(message) if message.contains("verify_password requires"))
        );

        let aes_missing = handle("aes_encrypt", &[string_value("plain")]).unwrap();
        assert!(
            matches!(aes_missing, Value::Error(message) if message.contains("aes_encrypt requires"))
        );

        let aes_extra = handle(
            "aes_encrypt",
            &[string_value("plain"), string_value("key"), string_value("extra")],
        )
        .unwrap();
        assert!(
            matches!(aes_extra, Value::Error(message) if message.contains("aes_encrypt requires"))
        );

        let rsa_bad_size = handle("rsa_generate_keypair", &[Value::Int(1024)]).unwrap();
        assert!(
            matches!(rsa_bad_size, Value::Error(message) if message.contains("RSA key size must be 2048 or 4096 bits"))
        );

        let rsa_keypair_extra =
            handle("rsa_generate_keypair", &[Value::Int(2048), string_value("extra")]).unwrap();
        assert!(
            matches!(rsa_keypair_extra, Value::Error(message) if message.contains("rsa_generate_keypair requires"))
        );

        let rsa_verify_missing =
            handle("rsa_verify", &[string_value("msg"), string_value("sig")]).unwrap();
        assert!(
            matches!(rsa_verify_missing, Value::Error(message) if message.contains("rsa_verify requires"))
        );

        let rsa_verify_extra = handle(
            "rsa_verify",
            &[
                string_value("msg"),
                string_value("sig"),
                string_value("pubkey"),
                string_value("extra"),
            ],
        )
        .unwrap();
        assert!(
            matches!(rsa_verify_extra, Value::Error(message) if message.contains("rsa_verify requires"))
        );
    }
}
