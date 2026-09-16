#![no_main]

use kujo::interpreter::validate_pdf_html_for_fuzz;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let length = data.len().min(64 * 1024);
    let html = String::from_utf8_lossy(&data[..length]);
    let _ = validate_pdf_html_for_fuzz(&html);
});
