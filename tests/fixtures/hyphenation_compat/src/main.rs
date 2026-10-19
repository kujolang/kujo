use bincode_next::{config, serde::encode_into_std_write};
use hyphenation::{extended::Extended, Hyphenator, Iter, Language, Load, Standard};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Cursor, path::Path};

fn digest(mut value: serde_json::Value) -> String {
    value.sort_all_objects();
    format!("{:x}", Sha256::digest(serde_json::to_vec(&value).unwrap()))
}

fn main() {
    let baseline: BTreeMap<String, String> =
        serde_json::from_str(include_str!("../baseline.json")).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../vendor/hyphenation-0.8.4/dictionaries");
    let output = std::env::args_os().nth(1).map(std::path::PathBuf::from);
    if let Some(ref dir) = output {
        fs::create_dir_all(dir).unwrap();
    }
    let count = fs::read_dir(&root)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().extension().is_some_and(|x| x == "bincode"))
        .count();
    assert_eq!(baseline.len(), count);
    for (name, expected) in &baseline {
        let bytes = fs::read(root.join(name)).unwrap();
        let mut reader = Cursor::new(&bytes);
        let mut encoded = Vec::new();
        let actual = if name.contains(".extended.") {
            let dict = Extended::any_from_reader(&mut reader).unwrap();
            let embedded = Extended::from_embedded(dict.language()).unwrap();
            assert_eq!(
                digest(serde_json::to_value(&embedded).unwrap()),
                *expected,
                "embedded {name}"
            );
            encode_into_std_write(&dict, &mut encoded, config::legacy()).unwrap();
            digest(serde_json::to_value(dict).unwrap())
        } else {
            let dict = Standard::any_from_reader(&mut reader).unwrap();
            let embedded = Standard::from_embedded(dict.language()).unwrap();
            assert_eq!(
                digest(serde_json::to_value(&embedded).unwrap()),
                *expected,
                "embedded {name}"
            );
            encode_into_std_write(&dict, &mut encoded, config::legacy()).unwrap();
            digest(serde_json::to_value(dict).unwrap())
        };
        assert_eq!(actual, *expected, "reader {name}");
        assert_eq!(reader.position(), bytes.len() as u64, "reader consumption {name}");
        assert_eq!(encoded.len(), bytes.len(), "encoded length {name}");
        if let Some(ref dir) = output {
            fs::write(dir.join(name), encoded).unwrap();
        }
    }
    let english = Standard::from_embedded(Language::EnglishUS).unwrap();
    assert_eq!(
        english.hyphenate("hyphenation").iter().collect::<Vec<_>>(),
        ["hy-", "phen-", "a-", "tion"]
    );
    let bytes = fs::read(root.join("en-us.standard.bincode")).unwrap();
    assert!(matches!(
        Standard::from_reader(Language::French, &mut Cursor::new(&bytes)),
        Err(hyphenation::load::Error::LanguageMismatch { .. })
    ));
    for len in [0, 1, bytes.len() / 2, bytes.len() - 1] {
        assert!(
            Standard::any_from_reader(&mut Cursor::new(&bytes[..len])).is_err(),
            "truncated {len}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.extend_from_slice(b"trailing");
    let mut reader = Cursor::new(trailing);
    Standard::any_from_reader(&mut reader).unwrap();
    assert_eq!(reader.position(), bytes.len() as u64);
    println!("{} original dictionaries match through reader and embedded paths; encoding lengths, hyphenation, mismatch, truncation and stream boundaries passed", baseline.len());
}
