// Repository-maintenance decoder for Heaptrack 1.3.0 interpreted format v3.
// Format authority: KDE/heaptrack v1.3.0 src/analyze/accumulatedtracedata.cpp,
// branches for 'a', '+', and '-'. Count allocation requests, NOT copied bytes.
// Full-stream totals are independent of symbols; no helper attribution is inferred.
use std::{
    collections::BTreeMap,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
};
#[derive(Default)]
struct Totals {
    allocations: u64,
    requested: u64,
    live: u64,
    peak: u64,
    sizes: BTreeMap<u64, u64>,
}
fn read(reader: impl BufRead) -> Result<Totals, String> {
    let mut infos: Vec<(u64, u64)> = Vec::new();
    let mut totals = Totals::default();
    let mut header = false;
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if !header {
            if line != "v 10300 3" {
                return Err("unsupported trace version".into());
            }
            header = true;
            continue;
        }
        let mode = line.as_bytes().first().copied();
        if !matches!(mode, Some(b'a' | b'+' | b'-')) {
            continue;
        }
        let mut fields = line.split_ascii_whitespace();
        let mode = fields.next().ok_or("missing mode")?;
        let number = |s: Option<&str>| {
            u64::from_str_radix(s.ok_or("missing integer")?, 16).map_err(|_| "invalid integer")
        };
        let first = number(fields.next())?;
        match mode {
            "a" => {
                let _trace_index = number(fields.next())?;
                infos.push((first, 0));
            }
            "+" | "-" => {
                let index = usize::try_from(first).map_err(|_| "index overflow")?;
                let (size, count) = infos.get_mut(index).ok_or("unknown allocation info")?;
                if mode == "+" {
                    *count += 1;
                    totals.allocations += 1;
                    totals.requested =
                        totals.requested.checked_add(*size).ok_or("byte overflow")?;
                    totals.live = totals.live.checked_add(*size).ok_or("live overflow")?;
                    totals.peak = totals.peak.max(totals.live);
                    *totals.sizes.entry(*size).or_default() += 1;
                } else {
                    *count = count.checked_sub(1).ok_or("free without allocation")?;
                    totals.live = totals.live.checked_sub(*size).ok_or("negative live bytes")?;
                }
            }
            _ => return Err("invalid record mode".into()),
        }
        if fields.next().is_some() {
            return Err("extra record field".into());
        }
    }
    if !header {
        return Err("empty trace".into());
    }
    Ok(totals)
}
fn main() {
    let path = std::env::args().nth(1).expect("TRACE.zst");
    let mut child =
        Command::new("zstd").args(["-dc", "--", &path]).stdout(Stdio::piped()).spawn().unwrap();
    let result = read(BufReader::new(child.stdout.take().unwrap()));
    if result.is_err() {
        let _ = child.kill();
    }
    let status = child.wait().unwrap();
    let totals = result.expect("invalid heaptrack stream");
    assert!(status.success(), "decompression failed");
    let sizes = totals
        .sizes
        .iter()
        .map(|(size, count)| format!("\"{size}\":{count}"))
        .collect::<Vec<_>>()
        .join(",");
    println!("{{\"allocations\":{},\"requested_bytes\":{},\"peak_live_bytes\":{},\"unfreed_bytes_unsuppressed\":{},\"allocation_sizes\":{{{sizes}}}}}", totals.allocations, totals.requested, totals.peak, totals.live);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_reused_info_sizes_and_live_memory() {
        let data = b"v 10300 3\na 10 1\na 20 2\n+ 0\n+ 1\n- 0\n+ 0\n- 1\n";
        let t = read(&data[..]).unwrap();
        assert_eq!((t.allocations, t.requested, t.peak, t.live), (3, 64, 48, 16));
        assert_eq!(t.sizes, BTreeMap::from([(16, 2), (32, 1)]));
    }
    #[test]
    fn rejects_missing_versions_invalid_indices_and_unbalanced_frees() {
        for data in [
            "",
            "v 10300 2\n",
            "v 10300 3\n+ 0\n",
            "v 10300 3\na 10 1\n- 0\n",
            "v 10300 3\na 10 1 3\n",
            "v 10300 3\n+ nope\n",
        ] {
            assert!(read(data.as_bytes()).is_err());
        }
    }
}
