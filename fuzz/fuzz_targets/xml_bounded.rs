#![no_main]

use kujo::interpreter::Interpreter;
use kujo::lexer::tokenize;
use kujo::parser::Parser;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // The language surface accepts UTF-8 XML strings. Keep the harness itself
    // bounded so corpus minimization cannot turn source construction into the
    // resource consumer being measured.
    let length = data.len().min(64 * 1024);
    let xml = String::from_utf8_lossy(&data[..length]);
    let literal = format!("{xml:?}");
    let source = format!(
        "result := parse_xml_bounded({literal}, {{\"max_input_bytes\": 65536, \"max_depth\": 32, \"max_nodes\": 4096, \"max_attributes\": 8192, \"max_text_bytes\": 65536, \"max_tree_bytes\": 131072}})"
    );

    if let Ok(tokens) = tokenize(&source) {
        let mut parser = Parser::new(tokens);
        let program = parser.parse();
        let mut interpreter = Interpreter::new();
        interpreter.eval_stmts(&program);
    }
});
