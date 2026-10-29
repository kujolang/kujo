//! Counted collection batches retain snapshots, error ordering and fallback semantics.
use std::process::Command;

#[test]
fn collection_batches_preserve_values_bounds_callbacks_and_dynamic_arguments() {
    for kind in ["array", "mixed", "struct"] {
        let payload = |n: &str, index: usize| match kind {
            "struct" => format!("Item {{ value: {n} }}"),
            "mixed" if index % 2 == 1 => format!("{{\"value\": {n}}}"),
            _ => format!("[{n}]"),
        };
        let read = |object: &str, index: usize| match kind {
            "struct" => format!("{object}[{index}].value"),
            "mixed" if index % 2 == 1 => format!("{object}[{index}][\"value\"]"),
            _ => format!("{object}[{index}][0]"),
        };
        let update = |index: &str| {
            match kind {
            "struct" => format!("state[{index}] = Item {{ value: n }}"),
            "mixed" => format!("if j % 2 == 0 {{ state[{index}] = [n] }} else {{ state[{index}] = {{\"value\": n}} }}"),
            _ => format!("state[{index}] = [n]"),
        }
        };
        let initial = payload("0", 0);
        let three =
            (0..3).map(|i| format!("state[{i}] = {}\n", payload("n", i))).collect::<String>();
        let owner = format!(
            r#"
struct Item {{ value: int }}
mut state := [] mut key := 0
while key < 64 {{ state = push(state, {initial}) key += 1 }}
export func snapshot() {{ return state }}
export func edit(n, limit) {{ mut j := 0 while j < limit {{ {} j += 1 }} return n }}
export func reverse(n) {{ mut j := 1 while j < 65 {{ {} j += 1 }} return n }}
export func fail(n) {{ {three} state[3] = {} }}
func observe(n) {{ assert_equal({}, n) assert_equal({}, n) }}
export func callback(n) {{ {three} observe(n) return n }}
export func scalar_mix(n) {{ {three} state[0] = n + 0.5 state[1] = true state[2] = "done" return n }}
"#,
            update("j"),
            update("-j"),
            payload("n / 0", 3),
            read("state", 0),
            read("state", 2)
        );
        let all = |object: &str, n: i64| {
            (0..64).map(|i| format!("assert_equal({}, {n})\n", read(object, i))).collect::<String>()
        };
        let original = (0..64)
            .map(|i| match kind {
                "struct" => format!("assert_equal(original[{i}].value, 0)\n"),
                _ => format!("assert_equal(original[{i}][0], 0)\n"),
            })
            .collect::<String>();
        let mut source = format!(
            r#"
from owner import edit, reverse, fail, callback, snapshot, scalar_mix
let original := snapshot()
edit(1, 64) edit(2, 64)
let full := snapshot()
{}
assert_equal(edit(3, 0), 3)
{}
reverse(4) reverse(5)
let reversed := snapshot()
{}
mut caught := 0
try {{ fail(6) }} except err {{ caught += 1 }}
try {{ fail(7) }} except err {{ caught += 1 }}
assert_equal(caught, 2)
let failed := snapshot()
assert_equal({}, 7) assert_equal({}, 7) assert_equal({}, 5)
callback(8) callback(9)
assert_equal({}, 9)
try {{ edit(10, 65) }} except err {{ caught += 1 }}
try {{ edit(11, 65) }} except err {{ caught += 1 }}
assert_equal(caught, 4)
let overflow := snapshot()
{}
{}
{}
{}
"#,
            all("full", 2),
            all("snapshot()", 2),
            all("reversed", 5),
            read("failed", 0),
            read("failed", 2),
            read("failed", 3),
            read("snapshot()", 0),
            all("overflow", 11),
            all("full", 2),
            all("reversed", 5),
            original
        );
        if kind == "array" {
            source.push_str("edit([21], 8) edit([22], 8) assert_equal(snapshot()[0][0][0], 22)\n");
        }
        source.push_str("scalar_mix(12) scalar_mix(13) let scalars := snapshot() assert_equal(scalars[0], 13.5) assert_equal(scalars[1], true) assert_equal(scalars[2], \"done\")\n");
        source.push_str("print(\"ARRAY BATCH CONTRACT PASS\")\n");
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let path = directory.path().join("main.kujo");
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.arg("run").arg(&path).arg("--untrusted");
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{kind}/{mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"ARRAY BATCH CONTRACT PASS\n");
        }
    }
}
