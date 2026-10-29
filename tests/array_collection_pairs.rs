//! Paired publication must preserve fallback writes and observation boundaries.
use std::process::Command;

#[test]
fn array_collection_pairs_preserve_overflow_callbacks_errors_and_type_changes() {
    for dictionary in [false, true] {
        let payload =
            |n: &str| if dictionary { format!("{{\"value\": {n}}}") } else { format!("[{n}]") };
        let render = |source: &str| {
            let mut source = source.to_owned();
            for (key, expression) in [
                ("@zero@", "0"),
                ("@n@", "n"),
                ("@next@", "n + 1"),
                ("@last@", "n + 2"),
                ("@hundred@", "100"),
                ("@final@", "999"),
            ] {
                source = source.replace(key, &payload(expression));
            }
            source.replace("@read@", if dictionary { "[\"value\"]" } else { "[0]" })
        };
        let owner = render(
            r#"
mut state := [@zero@, @zero@, @zero@]
mut seen := null
export func snapshot() { return state }
export func observed_snapshot() { return seen }
export func edit(n) { state[0] = @n@ state[-2] = @next@ state[0] = @last@ return n }
export func overflow(n) { state[0] = @n@ state[1] = @next@ state[2] = @last@ return n }
export func fail(n) { state[0] = @n@ state[1] = @next@ state[99] = @last@ }
export func change_kind(n) { state[0] = @n@ state[1] = @next@ state[0] = "changed" return n }
export func observe(n) {
 state[0] = @n@ state[1] = @next@
 let saved := state
 state[0] = @last@
 assert_equal(saved[0]@read@, n)
 assert_equal(saved[1]@read@, n + 1)
 return n
}
func replace_index() { seen = state state = [@hundred@, @hundred@, @hundred@] return 0 }
export func replace(n) { state[0] = @n@ state[1] = @next@ state[replace_index()] = @final@ }
"#,
        );
        let source = render(
            r#"
from owner import snapshot, observed_snapshot, edit, overflow, fail, change_kind, observe, replace
let original := snapshot()
assert_equal(edit(1), 1) assert_equal(edit(10), 10)
let pair := snapshot()
assert_equal(pair[0]@read@, 12) assert_equal(pair[1]@read@, 11) assert_equal(pair[2]@read@, 0)
overflow(20) overflow(30)
let three := snapshot()
assert_equal(three[0]@read@, 30) assert_equal(three[1]@read@, 31) assert_equal(three[2]@read@, 32)
mut caught := 0
try { fail(40) } except err { caught += 1 }
try { fail(50) } except err { caught += 1 }
assert_equal(caught, 2)
let failed := snapshot()
assert_equal(failed[0]@read@, 50) assert_equal(failed[1]@read@, 51) assert_equal(failed[2]@read@, 32)
change_kind(60) change_kind(70)
let changed := snapshot()
assert_equal(changed[0], "changed") assert_equal(changed[1]@read@, 71)
observe(80) observe(90)
assert_equal(snapshot()[0]@read@, 92)
replace(100) replace(110)
let final := snapshot()
assert_equal(final[0]@read@, 999) assert_equal(final[1]@read@, 100)
assert_equal(observed_snapshot()[0]@read@, 110) assert_equal(observed_snapshot()[1]@read@, 111)
assert_equal(pair[0]@read@, 12) assert_equal(three[0]@read@, 30) assert_equal(failed[0]@read@, 50)
mut j := 0 while j < 3 { assert_equal(original[j]@read@, 0) j += 1 }
print("ARRAY PAIR PASS")
"#,
        );
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
                "dictionary={dictionary},mode={mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"ARRAY PAIR PASS\n");
        }
    }
}
