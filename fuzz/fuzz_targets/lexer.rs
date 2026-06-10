#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Kujo's lexer API accepts UTF-8 source text. We still feed arbitrary bytes
    // by using lossy conversion for invalid UTF-8 sequences.
    let source = String::from_utf8_lossy(data);
    let _ = kujo::lexer::tokenize(source.as_ref());
});
