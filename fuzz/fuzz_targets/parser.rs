#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The parser consumes tokens, so we fuzz end-to-end lexer->parser plumbing.
    let source = String::from_utf8_lossy(data);
    if let Ok(tokens) = kujo::lexer::tokenize(source.as_ref()) {
        let mut parser = kujo::parser::Parser::new(tokens);
        let _ = parser.parse_with_diagnostics();
    }
});
