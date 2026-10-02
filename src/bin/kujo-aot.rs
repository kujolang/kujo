//! Native AOT compiler for Kujo's scalar command-line subset.

use kujo::native_aot::{compile_file_to_c, compile_file_to_executable, default_output_path};
use std::path::PathBuf;

const USAGE: &str = "Usage: kujo-aot SOURCE [-o OUTPUT] [--emit-c OUTPUT.c]";

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let Some(first) = args.next() else {
        return Err(USAGE.to_string());
    };
    if first == "--help" || first == "-h" {
        println!("{USAGE}");
        println!();
        println!("Compile Kujo scalar-core programs to optimized native executables.");
        return Ok(());
    }
    if first == "--version" || first == "-V" {
        println!("kujo-aot {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let source = PathBuf::from(first);
    let mut output = None;
    let mut emit_c = None;
    while let Some(arg) = args.next() {
        if arg == "-o" || arg == "--output" {
            output = Some(PathBuf::from(args.next().ok_or_else(|| USAGE.to_string())?));
        } else if arg == "--emit-c" {
            emit_c = Some(PathBuf::from(args.next().ok_or_else(|| USAGE.to_string())?));
        } else {
            return Err(USAGE.to_string());
        }
    }

    if let Some(c_path) = emit_c {
        let generated = compile_file_to_c(&source)?;
        std::fs::write(&c_path, generated)
            .map_err(|error| format!("failed to write '{}': {error}", c_path.display()))?;
        println!("{}", c_path.display());
    } else {
        let output = output.unwrap_or_else(|| default_output_path(&source));
        compile_file_to_executable(&source, &output, None)?;
        println!("{}", output.display());
    }
    Ok(())
}
