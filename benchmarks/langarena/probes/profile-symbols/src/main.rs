//! Preserve sample text while resolving Rust legacy and v0 symbol tokens.
use std::io::{self, Read, Write};

fn resolve(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if character.is_ascii_alphanumeric()
            || character == '_'
            || character == '$'
            || character == '.'
        {
            continue;
        }
        append_token(&mut output, &text[start..index]);
        output.push(character);
        start = index + character.len_utf8();
    }
    append_token(&mut output, &text[start..]);
    output
}

fn append_token(output: &mut String, token: &str) {
    match rustc_demangle::try_demangle(token) {
        Ok(symbol) => output.push_str(&format!("{symbol:#}")),
        Err(_) => output.push_str(token),
    }
}

fn main() -> io::Result<()> {
    let mut text = String::new();
    io::stdin().read_to_string(&mut text)?;
    io::stdout().write_all(resolve(&text).as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_actual_failed_ci_frame_without_changing_sample_structure() {
        let raw = "  + 2 _RNvMs4_NtCs452tOPeSYGq_4kujo2vmNtB5_2VM16execute_dispatch  (in kujo) + 8901  [0x10e236945]  vm.rs:3459\n";
        assert_eq!(
            resolve(raw),
            "  + 2 <kujo::vm::VM>::execute_dispatch  (in kujo) + 8901  [0x10e236945]  vm.rs:3459\n"
        );
    }

    #[test]
    fn preserves_unresolved_names_addresses_unicode_and_counts() {
        let raw = "2 ??? (in kujo) [0x123]\n  1 memmove\n  1 _Rinvalid\nλ\n";
        assert_eq!(resolve(raw), raw);
        assert!(!resolve(raw).contains("kujo::vm::"));
        assert_eq!(resolve("_ZN4kujo2vm3runE"), "kujo::vm::run");
    }
}
