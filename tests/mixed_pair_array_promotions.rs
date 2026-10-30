use std::process::Command;

#[test]
fn full_two_entry_promotions_preserve_the_other_slot_and_snapshots() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("owner.kujo"), r#"
mut state := [0, 1]
export func snapshot() { return state }
export func scalars(n) { state[0] = n state[1] = n + 1 state[0] = [n] return n }
export func arrays(n) { state[0] = [n] state[1] = [n + 1] state[1] = {"value": n + 1} return n }
export func dictionaries(n) { state[0] = {"value": n} state[1] = {"value": n + 1} state[0] = n return n }
export func mixed(n, blob) { state[0] = "entry" state[1] = blob state[0] = [n] state[1] = {"value": n + 1} return n }
"#).unwrap();
    let source = directory.path().join("main.kujo");
    std::fs::write(
        &source,
        r#"
from owner import snapshot, scalars, arrays, dictionaries, mixed
let original := snapshot()
let blob := bytes([0, 127, 128, 255])
// Each helper runs twice before taking its result snapshot: the second call
// exercises private journal promotion after the first detaches saved owners.
mut n := 1 while n <= 8 {
 assert_equal(scalars(n), n)
 assert_equal(scalars(n), n)
 let scalar_saved := snapshot()
 assert_equal(scalar_saved, [[n], n + 1])
 assert_equal(arrays(n), n)
 assert_equal(arrays(n), n)
 let array_saved := snapshot()
 assert_equal(array_saved, [[n], {"value": n + 1}])
 assert_equal(scalar_saved, [[n], n + 1])
 assert_equal(dictionaries(n), n)
 assert_equal(dictionaries(n), n)
 let dict_saved := snapshot()
 assert_equal(dict_saved, [n, {"value": n + 1}])
 assert_equal(array_saved, [[n], {"value": n + 1}])
 assert_equal(mixed(n, blob), n)
 assert_equal(mixed(n, blob), n)
 assert_equal(snapshot(), [[n], {"value": n + 1}])
 assert_equal(dict_saved, [n, {"value": n + 1}])
 assert_equal(scalar_saved, [[n], n + 1])
 n += 1
}
assert_equal(original, [0, 1])
print("FULL MIXED PAIR PROMOTION PASS")
"#,
    )
    .unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&source).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"FULL MIXED PAIR PROMOTION PASS\n", "{mode}");
    }
}
