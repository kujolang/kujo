"""Compare the checked-out lexer with a baseline revision using existing Cargo artifacts."""

import json
from pathlib import Path
import resource
import statistics
import subprocess
import sys
import tempfile


RUST_HARNESS = """
fn main() {
    let source = std::fs::read_to_string(std::env::args_os().nth(1).unwrap()).unwrap();
    let mut decoded_bytes = 0;
    for _ in 0..4 {
        let tokens = std::hint::black_box(lexer::tokenize(&source)).unwrap();
        decoded_bytes += tokens.iter().map(|token| match &token.kind {
            lexer::TokenKind::String(text) => text.len(),
            _ => 0,
        }).sum::<usize>();
    }
    println!("{decoded_bytes}");
}
"""


def compile_harness(root, scratch, label, lexer, deps):
    source = scratch / f"{label}.rs"
    executable = scratch / label
    modules = {"errors": root / "src/errors.rs", "runtime_limits": root / "src/runtime_limits.rs",
               "lexer": lexer}
    source.write_text("".join(f"#[path = {json.dumps(str(path))}] mod {name};\n"
                              for name, path in modules.items()) + RUST_HARNESS)
    command = ["rustc", "--edition", "2021", "-C", "opt-level=3", "-Awarnings",
               "-L", f"dependency={deps}"]
    for crate in ("colored", "serde_json"):
        library = next(deps.glob(f"lib{crate}-*.rlib"))
        command.extend(("--extern", f"{crate}={library}"))
    subprocess.run([*command, source, "-o", executable], check=True)
    return executable


def sample(executable, fixture):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    result = subprocess.run([executable, fixture], check=True, capture_output=True)
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    seconds = (after.ru_utime - before.ru_utime) + (after.ru_stime - before.ru_stime)
    return round(seconds * 1000, 2), int(result.stdout)


def main():
    revision = sys.argv[1]
    root = Path(__file__).resolve().parents[3]
    deps = root / "target/debug/deps"
    cases = [("ASCII 512", "x", 512), ("ASCII 4096", "x", 4096),
             ("ASCII 8192", "x", 8192), ("Unicode 8192", "界", 8192)]
    with tempfile.TemporaryDirectory() as directory:
        scratch = Path(directory)
        baseline_source = scratch / "baseline_lexer.rs"
        baseline_source.write_bytes(subprocess.check_output(
            ["git", "show", f"{revision}:src/lexer.rs"], cwd=root))
        baseline = compile_harness(root, scratch, "baseline", baseline_source, deps)
        candidate = compile_harness(root, scratch, "candidate", root / "src/lexer.rs", deps)
        fixture = scratch / "lexer.kujo"
        for name, character, width in cases:
            literal = character * width
            literal_bytes = len(literal.encode())
            repeats = (768 * 1024) // literal_bytes
            fixture.write_text("".join(f'let value_{i} := "{literal}"\n' for i in range(repeats)),
                               encoding="utf-8")
            samples = [[], []]
            for iteration in range(6):
                order = (0, 1) if iteration % 2 == 0 else (1, 0)
                for index in order:
                    elapsed, decoded = sample((baseline, candidate)[index], fixture)
                    assert decoded == 4 * repeats * literal_bytes, name
                    if iteration:
                        samples[index].append(elapsed)
            print(json.dumps({"case": name, "sourceBytes": fixture.stat().st_size,
                              "baselineMs": samples[0], "candidateMs": samples[1],
                              "baselineMedianMs": statistics.median(samples[0]),
                              "candidateMedianMs": statistics.median(samples[1])}))


if __name__ == "__main__":
    main()
