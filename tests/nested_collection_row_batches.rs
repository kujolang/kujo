use std::process::Command;

#[test]
fn nested_collection_rows_preserve_snapshots_aliases_and_error_publication() {
    for (shape, owner, main) in [
        (
            "dict",
            r###"mut root := {"row": [0,0,0,0,0,0,0,0]}
export func snapshot() { return root["row"] }
export func batch(n) { mut j := 0 while j < 8 { root["row"][j] = [n+j] j += 1 } return n }
export func mixed(n) { root["row"][0] = n root["row"][1] = [n] root["row"][2] = {"v":n} return n }
export func aliases(n) { root["row"][0] = [n] root["row"][-8] = {"v":n} root["row"][-1] = [n+1] return n }
export func fail(n) { root["row"][0] = [n] root["row"][1] = [n+1] root["row"][8] = [999] return n }
func observe() { return root["row"][0] }
export func callback(n) { root["row"][0] = [n] root["row"][1] = observe() root["row"][2] = [n] return n }
export func rewrite(n) { mut j := 0 while j < 8 { root["row"][j] = [n+j] j += 1 } root["row"][-8] = [n+99] return n }
"###,
            r###"from owner import snapshot, batch, mixed, aliases, fail, callback, rewrite
let original := snapshot()
assert_equal(batch(10),10)
let saved := snapshot()
assert_equal(mixed(20),20)
assert_equal(snapshot()[0],20) assert_equal(snapshot()[1],[20]) assert_equal(snapshot()[2],{"v":20})
assert_equal(aliases(30),30)
assert_equal(snapshot()[0],{"v":30}) assert_equal(snapshot()[7],[31])
let before := snapshot()
mut caught := false
try { fail(40) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0],[40]) assert_equal(snapshot()[1],[41]) assert_equal(snapshot()[7],before[7])
assert_equal(callback(50),50) assert_equal(snapshot()[0],[50]) assert_equal(snapshot()[1],[50])
assert_equal(rewrite(60),60) assert_equal(snapshot()[0],[159]) assert_equal(snapshot()[7],[67])
mut j := 0 while j < 8 { assert_equal(original[j],0) assert_equal(saved[j],[10+j]) j += 1 }
print("NESTED ROW BOUNDARY PASS")
"###,
        ),
        (
            "struct",
            r###"struct State { row: array } mut root := State { row: [0,0,0,0,0,0,0,0] }
export func snapshot() { return root.row }
export func batch(n) { mut j := 0 while j < 8 { root.row[j] = [n+j] j += 1 } return n }
export func mixed(n) { root.row[0] = n root.row[1] = [n] root.row[2] = {"v":n} return n }
export func aliases(n) { root.row[0] = [n] root.row[-8] = {"v":n} root.row[-1] = [n+1] return n }
export func fail(n) { root.row[0] = [n] root.row[1] = [n+1] root.row[8] = [999] return n }
func observe() { return root.row[0] }
export func callback(n) { root.row[0] = [n] root.row[1] = observe() root.row[2] = [n] return n }
export func rewrite(n) { mut j := 0 while j < 8 { root.row[j] = [n+j] j += 1 } root.row[-8] = [n+99] return n }
"###,
            r###"from owner import snapshot, batch, mixed, aliases, fail, callback, rewrite
let original := snapshot()
assert_equal(batch(10),10)
let saved := snapshot()
assert_equal(mixed(20),20)
assert_equal(snapshot()[0],20) assert_equal(snapshot()[1],[20]) assert_equal(snapshot()[2],{"v":20})
assert_equal(aliases(30),30)
assert_equal(snapshot()[0],{"v":30}) assert_equal(snapshot()[7],[31])
let before := snapshot()
mut caught := false
try { fail(40) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0],[40]) assert_equal(snapshot()[1],[41]) assert_equal(snapshot()[7],before[7])
assert_equal(callback(50),50) assert_equal(snapshot()[0],[50]) assert_equal(snapshot()[1],[50])
assert_equal(rewrite(60),60) assert_equal(snapshot()[0],[159]) assert_equal(snapshot()[7],[67])
mut j := 0 while j < 8 { assert_equal(original[j],0) assert_equal(saved[j],[10+j]) j += 1 }
print("NESTED ROW BOUNDARY PASS")
"###,
        ),
        (
            "deep",
            r###"mut root := {"outer": [{"row": [0,0,0,0,0,0,0,0]}]}
export func snapshot() { return root["outer"][0]["row"] }
export func batch(n) { mut j := 0 while j < 8 { root["outer"][0]["row"][j] = [n+j] j += 1 } return n }
export func mixed(n) { root["outer"][0]["row"][0] = n root["outer"][0]["row"][1] = [n] root["outer"][0]["row"][2] = {"v":n} return n }
export func aliases(n) { root["outer"][0]["row"][0] = [n] root["outer"][0]["row"][-8] = {"v":n} root["outer"][0]["row"][-1] = [n+1] return n }
export func fail(n) { root["outer"][0]["row"][0] = [n] root["outer"][0]["row"][1] = [n+1] root["outer"][0]["row"][8] = [999] return n }
func observe() { return root["outer"][0]["row"][0] }
export func callback(n) { root["outer"][0]["row"][0] = [n] root["outer"][0]["row"][1] = observe() root["outer"][0]["row"][2] = [n] return n }
export func rewrite(n) { mut j := 0 while j < 8 { root["outer"][0]["row"][j] = [n+j] j += 1 } root["outer"][0]["row"][-8] = [n+99] return n }
"###,
            r###"from owner import snapshot, batch, mixed, aliases, fail, callback, rewrite
let original := snapshot()
assert_equal(batch(10),10)
let saved := snapshot()
assert_equal(mixed(20),20)
assert_equal(snapshot()[0],20) assert_equal(snapshot()[1],[20]) assert_equal(snapshot()[2],{"v":20})
assert_equal(aliases(30),30)
assert_equal(snapshot()[0],{"v":30}) assert_equal(snapshot()[7],[31])
let before := snapshot()
mut caught := false
try { fail(40) } except err { caught = true }
assert(caught) assert_equal(snapshot()[0],[40]) assert_equal(snapshot()[1],[41]) assert_equal(snapshot()[7],before[7])
assert_equal(callback(50),50) assert_equal(snapshot()[0],[50]) assert_equal(snapshot()[1],[50])
assert_equal(rewrite(60),60) assert_equal(snapshot()[0],[159]) assert_equal(snapshot()[7],[67])
mut j := 0 while j < 8 { assert_equal(original[j],0) assert_equal(saved[j],[10+j]) j += 1 }
print("NESTED ROW BOUNDARY PASS")
"###,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let source = directory.path().join("main.kujo");
        std::fs::write(&source, main).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.arg(&source).output().unwrap();
            assert!(
                output.status.success(),
                "{shape}/{mode}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(output.stdout, b"NESTED ROW BOUNDARY PASS\n", "{shape}/{mode}");
        }
    }
}
