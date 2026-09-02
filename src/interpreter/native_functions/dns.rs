// DNS record lookup primitives with bounded, deterministic result envelopes.

use crate::interpreter::{DictMap, Interpreter, Value};
use hickory_resolver::config::ResolverOpts;
use hickory_resolver::proto::dnssec::Proof;
use hickory_resolver::proto::rr::{Name, RData};
use hickory_resolver::Resolver;
use std::net::IpAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

const DEFAULT_TIMEOUT_MS: u64 = 5_000;
const MIN_TIMEOUT_MS: i64 = 250;
const MAX_TIMEOUT_MS: i64 = 30_000;
const DEFAULT_ATTEMPTS: usize = 2;
const MAX_ATTEMPTS: i64 = 5;
const DEFAULT_MAX_RECORDS: usize = 64;
const MAX_RECORDS: i64 = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LookupKind {
    A,
    Aaaa,
    Mx,
    Txt,
    Ptr,
    Tlsa,
}

impl LookupKind {
    fn record_type(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::Aaaa => "AAAA",
            Self::Mx => "MX",
            Self::Txt => "TXT",
            Self::Ptr => "PTR",
            Self::Tlsa => "TLSA",
        }
    }
}

#[derive(Clone, Debug)]
struct LookupOptions {
    timeout_ms: u64,
    attempts: usize,
    max_records: usize,
    dnssec: bool,
}

impl Default for LookupOptions {
    fn default() -> Self {
        Self {
            timeout_ms: DEFAULT_TIMEOUT_MS,
            attempts: DEFAULT_ATTEMPTS,
            max_records: DEFAULT_MAX_RECORDS,
            dnssec: true,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum NormalizedRecord {
    Address {
        value: String,
        ttl: u32,
        proof: Proof,
    },
    Mx {
        preference: u16,
        exchange: String,
        ttl: u32,
        proof: Proof,
    },
    Txt {
        value: String,
        ttl: u32,
        proof: Proof,
    },
    Ptr {
        value: String,
        ttl: u32,
        proof: Proof,
    },
    Tlsa {
        usage: u8,
        selector: u8,
        matching: u8,
        association_data_hex: String,
        ttl: u32,
        proof: Proof,
    },
}

#[derive(Clone, Debug)]
struct LookupResult {
    records: Vec<NormalizedRecord>,
    no_records: bool,
}

fn error(message: impl Into<String>) -> Value {
    Value::ErrorObject { message: message.into(), stack: Vec::new(), line: None, cause: None }
}

fn string_value(value: impl Into<String>) -> Value {
    Value::Str(Arc::new(value.into()))
}

fn insert(map: &mut DictMap, key: &'static str, value: Value) {
    map.insert(Arc::from(key), value);
}

fn option_int(
    options: &DictMap,
    key: &str,
    default: i64,
    min: i64,
    max: i64,
) -> Result<i64, String> {
    match options.get(key) {
        None => Ok(default),
        Some(Value::Int(value)) if (*value >= min) && (*value <= max) => Ok(*value),
        Some(Value::Int(value)) => Err(format!(
            "dns lookup option '{}' must be between {} and {}, got {}",
            key, min, max, value
        )),
        Some(_) => Err(format!("dns lookup option '{}' must be an integer", key)),
    }
}

fn parse_options(value: Option<&Value>) -> Result<LookupOptions, String> {
    let Some(value) = value else {
        return Ok(LookupOptions::default());
    };
    let Value::Dict(options) = value else {
        return Err("dns lookup options must be a dictionary".to_string());
    };

    let timeout_ms = option_int(
        options,
        "timeout_ms",
        DEFAULT_TIMEOUT_MS as i64,
        MIN_TIMEOUT_MS,
        MAX_TIMEOUT_MS,
    )? as u64;
    let attempts =
        option_int(options, "attempts", DEFAULT_ATTEMPTS as i64, 1, MAX_ATTEMPTS)? as usize;
    let max_records =
        option_int(options, "max_records", DEFAULT_MAX_RECORDS as i64, 1, MAX_RECORDS)? as usize;
    let dnssec = match options.get("dnssec") {
        None => true,
        Some(Value::Bool(value)) => *value,
        Some(_) => return Err("dns lookup option 'dnssec' must be a boolean".to_string()),
    };

    let allowed = ["timeout_ms", "attempts", "max_records", "dnssec"];
    if let Some(unknown) = options.keys().find(|key| !allowed.contains(&key.as_ref())) {
        return Err(format!("unknown dns lookup option '{}'", unknown));
    }

    Ok(LookupOptions { timeout_ms, attempts, max_records, dnssec })
}

fn normalize_name(name: &Name) -> String {
    name.to_utf8().trim_end_matches('.').to_ascii_lowercase()
}

fn validate_query(kind: LookupKind, input: &str) -> Result<String, String> {
    let input = input.trim();
    if input.is_empty() || input.len() > 253 || input.chars().any(char::is_whitespace) {
        return Err("dns lookup name must be a non-empty DNS name of at most 253 bytes".to_string());
    }
    if kind == LookupKind::Ptr {
        return input
            .parse::<IpAddr>()
            .map(|ip| ip.to_string())
            .map_err(|_| "dns_lookup_ptr requires a valid IPv4 or IPv6 address".to_string());
    }
    let fqdn = format!("{}.", input.trim_end_matches('.'));
    Name::from_str(&fqdn).map_err(|_| "dns lookup name is not a valid DNS name".to_string())?;
    Ok(fqdn)
}

fn lookup_in_runtime(
    kind: LookupKind,
    query: String,
    options: LookupOptions,
) -> Result<LookupResult, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("dns resolver runtime initialization failed: {}", err))?;
    runtime.block_on(async move {
        let mut builder = Resolver::builder_tokio()
            .map_err(|err| format!("dns resolver configuration failed: {}", err))?;
        let resolver_options: &mut ResolverOpts = builder.options_mut();
        resolver_options.timeout = Duration::from_millis(options.timeout_ms);
        resolver_options.attempts = options.attempts;
        resolver_options.validate = options.dnssec;
        let resolver = builder
            .build()
            .map_err(|err| format!("dns resolver initialization failed: {}", err))?;

        let lookup = match kind {
            LookupKind::A => resolver.ipv4_lookup(query).await,
            LookupKind::Aaaa => resolver.ipv6_lookup(query).await,
            LookupKind::Mx => resolver.mx_lookup(query).await,
            LookupKind::Txt => resolver.txt_lookup(query).await,
            LookupKind::Ptr => {
                let ip = query.parse::<IpAddr>().map_err(|_| {
                    "dns_lookup_ptr requires a valid IPv4 or IPv6 address".to_string()
                })?;
                resolver.reverse_lookup(Name::from(ip)).await
            }
            LookupKind::Tlsa => resolver.tlsa_lookup(query).await,
        };

        let lookup = match lookup {
            Ok(lookup) => lookup,
            Err(err) if err.is_no_records_found() => {
                return Ok(LookupResult { records: Vec::new(), no_records: true });
            }
            Err(err) => return Err(format!("dns {} lookup failed: {}", kind.record_type(), err)),
        };

        let mut records = Vec::new();
        for record in lookup.answers().iter().take(options.max_records + 1) {
            let ttl = record.ttl;
            let proof = if options.dnssec { record.proof } else { Proof::Indeterminate };
            let normalized = match (kind, &record.data) {
                (LookupKind::A, RData::A(address)) => {
                    NormalizedRecord::Address { value: address.0.to_string(), ttl, proof }
                }
                (LookupKind::Aaaa, RData::AAAA(address)) => {
                    NormalizedRecord::Address { value: address.0.to_string(), ttl, proof }
                }
                (LookupKind::Mx, RData::MX(mx)) => NormalizedRecord::Mx {
                    preference: mx.preference,
                    exchange: normalize_name(&mx.exchange),
                    ttl,
                    proof,
                },
                (LookupKind::Txt, RData::TXT(txt)) => NormalizedRecord::Txt {
                    value: String::from_utf8_lossy(
                        &txt.txt_data
                            .iter()
                            .flat_map(|part| part.iter().copied())
                            .collect::<Vec<_>>(),
                    )
                    .into_owned(),
                    ttl,
                    proof,
                },
                (LookupKind::Ptr, RData::PTR(ptr)) => {
                    NormalizedRecord::Ptr { value: normalize_name(&ptr.0), ttl, proof }
                }
                (LookupKind::Tlsa, RData::TLSA(tlsa)) => NormalizedRecord::Tlsa {
                    usage: tlsa.cert_usage.into(),
                    selector: tlsa.selector.into(),
                    matching: tlsa.matching.into(),
                    association_data_hex: tlsa
                        .cert_data
                        .iter()
                        .map(|byte| format!("{:02x}", byte))
                        .collect(),
                    ttl,
                    proof,
                },
                _ => continue,
            };
            records.push(normalized);
        }
        if records.len() > options.max_records {
            return Err(format!(
                "dns {} lookup exceeded max_records limit {}",
                kind.record_type(),
                options.max_records
            ));
        }
        records.sort();
        records.dedup();
        Ok(LookupResult { records, no_records: false })
    })
}

fn proof_name(proof: Proof) -> &'static str {
    match proof {
        Proof::Secure => "SECURE",
        Proof::Insecure => "INSECURE",
        Proof::Bogus => "BOGUS",
        Proof::Indeterminate => "INDETERMINATE",
    }
}

fn record_proof(record: &NormalizedRecord) -> Proof {
    match record {
        NormalizedRecord::Address { proof, .. }
        | NormalizedRecord::Mx { proof, .. }
        | NormalizedRecord::Txt { proof, .. }
        | NormalizedRecord::Ptr { proof, .. }
        | NormalizedRecord::Tlsa { proof, .. } => *proof,
    }
}

fn dnssec_status(records: &[NormalizedRecord], checked: bool) -> &'static str {
    if !checked {
        return "NOT_CHECKED";
    }
    if records.iter().any(|record| record_proof(record) == Proof::Bogus) {
        return "BOGUS";
    }
    if !records.is_empty() && records.iter().all(|record| record_proof(record) == Proof::Secure) {
        return "SECURE";
    }
    if records.iter().any(|record| record_proof(record) == Proof::Insecure) {
        return "INSECURE";
    }
    "INDETERMINATE"
}

fn record_value(record: NormalizedRecord) -> Value {
    let mut map = DictMap::default();
    match record {
        NormalizedRecord::Address { value, ttl, proof } => {
            insert(&mut map, "value", string_value(value));
            insert(&mut map, "ttl", Value::Int(ttl as i64));
            insert(&mut map, "dnssec", string_value(proof_name(proof)));
        }
        NormalizedRecord::Mx { preference, exchange, ttl, proof } => {
            insert(&mut map, "preference", Value::Int(preference as i64));
            insert(&mut map, "exchange", string_value(exchange));
            insert(&mut map, "ttl", Value::Int(ttl as i64));
            insert(&mut map, "dnssec", string_value(proof_name(proof)));
        }
        NormalizedRecord::Txt { value, ttl, proof }
        | NormalizedRecord::Ptr { value, ttl, proof } => {
            insert(&mut map, "value", string_value(value));
            insert(&mut map, "ttl", Value::Int(ttl as i64));
            insert(&mut map, "dnssec", string_value(proof_name(proof)));
        }
        NormalizedRecord::Tlsa { usage, selector, matching, association_data_hex, ttl, proof } => {
            insert(&mut map, "certificate_usage", Value::Int(usage as i64));
            insert(&mut map, "selector", Value::Int(selector as i64));
            insert(&mut map, "matching_type", Value::Int(matching as i64));
            insert(&mut map, "certificate_association_data", string_value(association_data_hex));
            insert(&mut map, "ttl", Value::Int(ttl as i64));
            insert(&mut map, "dnssec", string_value(proof_name(proof)));
        }
    }
    Value::dict(map)
}

fn run_lookup(kind: LookupKind, arg_values: &[Value]) -> Value {
    if !(1..=2).contains(&arg_values.len()) {
        return Value::Error(format!(
            "dns_lookup_{} requires (string_name_or_ip, optional_options) arguments",
            kind.record_type().to_ascii_lowercase()
        ));
    }
    let Some(Value::Str(input)) = arg_values.first() else {
        return Value::Error(format!(
            "dns_lookup_{} requires a string query",
            kind.record_type().to_ascii_lowercase()
        ));
    };
    let query = match validate_query(kind, input.as_ref()) {
        Ok(query) => query,
        Err(message) => return Value::Error(message),
    };
    let options = match parse_options(arg_values.get(1)) {
        Ok(options) => options,
        Err(message) => return Value::Error(message),
    };

    let worker_query = query.clone();
    let worker_options = options.clone();
    let worker = std::thread::Builder::new()
        .name(format!("kujo-dns-{}", kind.record_type().to_ascii_lowercase()))
        .spawn(move || lookup_in_runtime(kind, worker_query, worker_options));
    let result = match worker {
        Ok(worker) => match worker.join() {
            Ok(result) => result,
            Err(_) => return error("dns lookup worker terminated unexpectedly"),
        },
        Err(err) => return error(format!("dns lookup worker failed to start: {}", err)),
    };
    let result = match result {
        Ok(result) => result,
        Err(message) => return error(message),
    };

    let status = dnssec_status(&result.records, options.dnssec);
    let records = result.records.into_iter().map(record_value).collect::<Vec<_>>();
    let mut envelope = DictMap::default();
    insert(&mut envelope, "schema_version", string_value("kujo.dns.lookup.v1"));
    insert(&mut envelope, "query", string_value(query.trim_end_matches('.')));
    insert(&mut envelope, "record_type", string_value(kind.record_type()));
    insert(&mut envelope, "dnssec_status", string_value(status));
    insert(
        &mut envelope,
        "status",
        string_value(if result.no_records { "NO_RECORDS" } else { "OK" }),
    );
    insert(&mut envelope, "records", Value::Array(Arc::new(records)));
    Value::dict(envelope)
}

pub fn handle(_interp: &mut Interpreter, name: &str, arg_values: &[Value]) -> Option<Value> {
    let kind = match name {
        "dns_lookup_a" => LookupKind::A,
        "dns_lookup_aaaa" => LookupKind::Aaaa,
        "dns_lookup_mx" => LookupKind::Mx,
        "dns_lookup_txt" => LookupKind::Txt,
        "dns_lookup_ptr" => LookupKind::Ptr,
        "dns_lookup_tlsa" => LookupKind::Tlsa,
        _ => return None,
    };
    Some(run_lookup(kind, arg_values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_ptr_and_dns_names() {
        assert_eq!(validate_query(LookupKind::Ptr, "192.0.2.1").unwrap(), "192.0.2.1");
        assert!(validate_query(LookupKind::Ptr, "mail.example").is_err());
        assert_eq!(validate_query(LookupKind::A, "Mail.Example").unwrap(), "Mail.Example.");
        assert_eq!(validate_query(LookupKind::Aaaa, "v6.example").unwrap(), "v6.example.");
        assert_eq!(validate_query(LookupKind::Mx, "Example.COM").unwrap(), "Example.COM.");
        assert!(validate_query(LookupKind::Txt, "bad name").is_err());
    }

    #[test]
    fn validates_bounded_options_and_unknown_keys() {
        let mut map = DictMap::default();
        insert(&mut map, "timeout_ms", Value::Int(250));
        insert(&mut map, "attempts", Value::Int(5));
        insert(&mut map, "max_records", Value::Int(256));
        insert(&mut map, "dnssec", Value::Bool(false));
        let options = parse_options(Some(&Value::dict(map))).unwrap();
        assert_eq!(options.timeout_ms, 250);
        assert_eq!(options.attempts, 5);
        assert_eq!(options.max_records, 256);
        assert!(!options.dnssec);

        let mut invalid = DictMap::default();
        insert(&mut invalid, "timeout_ms", Value::Int(249));
        assert!(parse_options(Some(&Value::dict(invalid))).is_err());
    }

    #[test]
    fn dnssec_status_is_fail_closed_for_bogus_proof() {
        let records = vec![NormalizedRecord::Txt {
            value: "v=spf1 -all".to_string(),
            ttl: 300,
            proof: Proof::Bogus,
        }];
        assert_eq!(dnssec_status(&records, true), "BOGUS");
        assert_eq!(dnssec_status(&records, false), "NOT_CHECKED");
    }

    #[test]
    fn address_records_share_the_bounded_dns_envelope_contract() {
        let record = NormalizedRecord::Address {
            value: "2001:db8::1".to_string(),
            ttl: 300,
            proof: Proof::Secure,
        };
        let Value::Dict(value) = record_value(record) else {
            panic!("address record must normalize to a dictionary");
        };
        match value.get("value") {
            Some(Value::Str(value)) => assert_eq!(value.as_ref(), "2001:db8::1"),
            other => panic!("unexpected address value: {other:?}"),
        }
        match value.get("ttl") {
            Some(Value::Int(value)) => assert_eq!(*value, 300),
            other => panic!("unexpected address TTL: {other:?}"),
        }
        match value.get("dnssec") {
            Some(Value::Str(value)) => assert_eq!(value.as_ref(), "SECURE"),
            other => panic!("unexpected address DNSSEC proof: {other:?}"),
        }
    }
}
