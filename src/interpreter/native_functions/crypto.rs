// File: src/interpreter/native_functions/crypto.rs
//
// Cryptography native functions

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::Engine;
use md5::Md5;
use rsa::pkcs8::{
    DecodePrivateKey, DecodePublicKey, EncodePrivateKey, EncodePublicKey, LineEnding,
};
use rsa::{Oaep, RsaPrivateKey, RsaPublicKey};
use sha2::{Digest, Sha256};

use crate::interpreter::{DictMap, Value};
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const STREAM_AEAD_MAGIC: &[u8; 9] = b"KUJOAEAD1";
const STREAM_AEAD_MIN_CHUNK: usize = 4096;
const STREAM_AEAD_MAX_CHUNK: usize = 16 * 1024 * 1024;

fn error_object(message: String) -> Value {
    Value::ErrorObject { message, stack: Vec::new(), line: None, cause: None }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
                match std::fs::read(path.as_ref()) {
                    Ok(contents) => Value::Str(Arc::new(sha256_hex(&contents))),
                    Err(e) => {
                        error_object(format!("Failed to read file '{}': {}", path.as_ref(), e))
                    }
                }
            } else {
                Value::Error("sha256_file requires a string path argument".to_string())
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

                let mut rng = rand::thread_rng();
                match RsaPrivateKey::new(&mut rng, bits_usize) {
                    Ok(private_key) => {
                        let public_key = RsaPublicKey::from(&private_key);

                        let private_pem = match private_key.to_pkcs8_pem(LineEnding::LF) {
                            Ok(pem) => pem.to_string(),
                            Err(e) => {
                                return Some(error_object(format!(
                                    "Failed to encode private key: {}",
                                    e
                                )))
                            }
                        };

                        let public_pem = match public_key.to_public_key_pem(LineEnding::LF) {
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

        "rsa_encrypt" => {
            if arg_values.len() != 2 {
                return Some(Value::Error(
                    "rsa_encrypt requires (plaintext_string, public_key_pem) arguments".to_string(),
                ));
            }

            match (arg_values.first(), arg_values.get(1)) {
                (Some(Value::Str(plaintext)), Some(Value::Str(public_key_pem))) => {
                    match RsaPublicKey::from_public_key_pem(public_key_pem.as_ref()) {
                        Ok(public_key) => {
                            let mut rng = rand::thread_rng();
                            let padding = Oaep::new::<Sha256>();

                            match public_key.encrypt(&mut rng, padding, plaintext.as_bytes()) {
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
                    match RsaPrivateKey::from_pkcs8_pem(private_key_pem.as_ref()) {
                        Ok(private_key) => {
                            match base64::engine::general_purpose::STANDARD
                                .decode(ciphertext_b64.as_ref())
                            {
                                Ok(ciphertext) => {
                                    let padding = Oaep::new::<Sha256>();
                                    match private_key.decrypt(padding, &ciphertext) {
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
                    match RsaPrivateKey::from_pkcs8_pem(private_key_pem.as_ref()) {
                        Ok(private_key) => {
                            use rsa::pkcs1v15::SigningKey;
                            use rsa::signature::{SignatureEncoding, Signer};

                            let signing_key = SigningKey::<Sha256>::new(private_key);
                            let signature = signing_key.sign(message.as_bytes());
                            Value::Str(Arc::new(
                                base64::engine::general_purpose::STANDARD
                                    .encode(signature.to_bytes()),
                            ))
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
                ) => match RsaPublicKey::from_public_key_pem(public_key_pem.as_ref()) {
                    Ok(public_key) => match base64::engine::general_purpose::STANDARD
                        .decode(signature_b64.as_ref())
                    {
                        Ok(signature_bytes) => {
                            use rsa::pkcs1v15::{Signature, VerifyingKey};
                            use rsa::signature::Verifier;

                            let verifying_key = VerifyingKey::<Sha256>::new(public_key);

                            match Signature::try_from(signature_bytes.as_slice()) {
                                Ok(signature) => {
                                    match verifying_key.verify(message.as_bytes(), &signature) {
                                        Ok(_) => Value::Bool(true),
                                        Err(_) => Value::Bool(false),
                                    }
                                }
                                Err(e) => error_object(format!("Invalid signature format: {}", e)),
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
    use super::handle;
    use crate::interpreter::Value;
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

        let _ = fs::remove_file(&path);
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
            &[string_value("tampered"), Value::Str(signature_b64), Value::Str(public_pem)],
        )
        .unwrap();
        assert!(matches!(tampered, Value::Bool(false)));
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
