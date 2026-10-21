//! Opt-in measurements, not timing assertions. Run with --ignored --nocapture.
//! KUJO_PROFILE_BINARY may point at a baseline binary built from an earlier commit.
use std::process::Command;

#[test]
#[ignore = "manual collection scaling measurements"]
fn profile_collections() {
    let binary = std::env::var("KUJO_PROFILE_BINARY")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_kujo").to_string());
    // Setup is outside the measured interval. Each body verifies its result.
    let cases = [
        ("append", "mut x := []", "mut i := 0 while i < N { x = push(x, i) i += 1 } assert_equal(len(x), N)"),
        ("index", "mut x := range(0, N)", "mut i := 0 while i < N { x[i] = 7 i += 1 } assert_equal(x[-1], 7)"),
        ("queue", "mut x := Queue(range(0, N))", "mut i := 0 while i < N { let pair := queue_dequeue(x) x = queue_enqueue(pair[0], i) i += 1 } assert_equal(queue_size(x), N)"),
        ("nested", "mut x := [range(0, N)]", "mut i := 0 while i < N { x[0][i] = 7 i += 1 } assert_equal(x[0][-1], 7)"),
        ("struct_row", "struct State { rows: array } mut x := State { rows: [range(0, N)] }", "mut i := 0 while i < N { x.rows[0][i] = 7 i += 1 } assert_equal(x.rows[0][-1], 7)"),
        ("dict_row", "mut x := {\"rows\": [range(0, N)]}", "mut i := 0 while i < N { x[\"rows\"][0][i] = 7 i += 1 } assert_equal(x[\"rows\"][0][-1], 7)"),
        ("field_append", "struct State { items: array } mut x := State { items: [] }", "mut i := 0 while i < N { x.items = push(x.items, i) i += 1 } assert_equal(len(x.items), N)"),
        ("pop", "mut x := range(0, N)", "mut i := 0 while i < N { let pair := pop(x) x = pair[0] i += 1 } assert_equal(len(x), 0)"),
        ("dictionary_remove", "mut x := {} mut j := 0 while j < N { x[\"key\" + str(j)] = j j += 1 }", "mut i := 0 while i < N { let pair := remove(x, \"key\" + str(i)) x = pair[0] i += 1 } assert_equal(len(x), 0)"),
        ("field_pop", "struct State { items: array } mut x := State { items: range(0, N) }", "mut i := 0 while i < N { let pair := pop(x.items) x.items = pair[0] i += 1 } assert_equal(len(x.items), 0)"),
        ("indexed_pop", "mut x := [range(0, N)]", "mut i := 0 while i < N { let pair := pop(x[0]) x[0] = pair[0] i += 1 } assert_equal(len(x[0]), 0)"),
        ("nested_dictionary_remove", "mut x := {\"map\": {}} mut j := 0 while j < N { x[\"map\"][\"key\" + str(j)] = j j += 1 }", "mut i := 0 while i < N { let pair := remove(x[\"map\"], \"key\" + str(i)) x[\"map\"] = pair[0] i += 1 } assert_equal(len(x[\"map\"]), 0)"),
        ("stack", "mut x := Stack(range(0, N))", "mut i := 0 while i < N { let pair := stack_pop(x) x = stack_push(pair[0], i) i += 1 } assert_equal(stack_size(x), N)"),
        ("tree", "struct Node { left: any, right: any } mut x := null", "mut count := N while count > 1 { x = Node { left: x, right: x } count = floor(count / 2) } assert(x != null)"),
        ("string", "let x := repeat(\"a\", N)", "mut i := 0 while i < N { assert_equal(x[i], \"a\") i += 1 }"),
        ("bytes", "let x := decode_base64(repeat(\"AAAA\", N))", "mut i := 0 while i < 20000 { assert_equal(x[i % (N * 3)], 0) i += 1 }"),
    ];
    let filter = std::env::var("KUJO_PROFILE_CASE").ok();
    for (name, setup, body) in cases {
        if filter.as_deref().is_some_and(|wanted| wanted != name) {
            continue;
        }
        for interpreter in [false, true] {
            for n in [1000, 2000, 4000] {
                let source = format!("func work() {{\n{setup}\nlet start := time_ns()\n{body}\nprint(time_ns() - start)\n}}\nwork()")
                    .replace("N)", &format!("{n})"))
                    .replace(":= N ", &format!(":= {n} "))
                    .replace("< N", &format!("< {n}"))
                    .replace("(N *", &format!("({n} *"));
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("profile.kujo");
                std::fs::write(&path, source).unwrap();
                let mut samples = Vec::new();
                for _ in 0..3 {
                    let mut cmd = Command::new(&binary);
                    cmd.arg("run").arg(&path).arg("--allow-clock");
                    if interpreter {
                        cmd.arg("--interpreter");
                    }
                    let out = cmd.output().unwrap();
                    assert!(
                        out.status.success(),
                        "{name}: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                    let text = String::from_utf8(out.stdout).unwrap();
                    samples.push(text.trim().parse::<f64>().expect(&text));
                }
                samples.sort_by(f64::total_cmp);
                println!(
                    "{name},runtime={},n={n},median_ns={}",
                    if interpreter { "interpreter" } else { "vm" },
                    samples[1]
                );
            }
        }
    }
}
