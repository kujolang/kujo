//! Capture complete decoded dictionary semantics with the original hyphenation 0.8.4.
use hyphenation::{extended::Extended, Load, Standard};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor};
fn main() {
    let root = std::env::args().nth(1).expect("dictionary directory");
    let mut result = BTreeMap::new();
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|x| x != "bincode") {
            continue;
        }
        let name = path.file_name().unwrap().to_str().unwrap().to_owned();
        let bytes = fs::read(&path).unwrap();
        let mut reader = Cursor::new(&bytes);
        let mut value = if name.contains(".extended.") {
            serde_json::to_value(Extended::any_from_reader(&mut reader).unwrap()).unwrap()
        } else {
            serde_json::to_value(Standard::any_from_reader(&mut reader).unwrap()).unwrap()
        };
        assert_eq!(reader.position(), bytes.len() as u64);
        value.sort_all_objects();
        result.insert(
            name,
            Sha256::digest(serde_json::to_vec(&value).unwrap())
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        );
    }
    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
