use std::process::Command;

#[test]
fn same_capture_reentry_preserves_writes_without_sharing_distinct_snapshots() {
    let cases: &[(&str, &str, &str)] = &[
        (
            "direct_inner_write",
            r###"func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 return 0 } step(0) return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "outer_write_visible_to_inner",
            r###"func factory() {
 mut data := 1
 func step(n) { if n == 0 { return data } data = 9 let seen := step(0) return [seen,data] }
 return step
}
let step := factory()
print(step(1))
"###,
            "[9, 9]\n",
        ),
        (
            "callback_inner_write",
            r###"func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 return 0 } let ignored := map([0],step) return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "computed_callee_inner_write",
            r###"func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 return 0 } let aliases := [step] aliases[0](0) return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "caught_inner_error",
            r###"func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 throw("inner") } try { step(0) } except err {} return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "distinct_factory_and_alias_control",
            r###"func factory() { mut data := 1 func step() { data += 1 return data } return step }
let first := factory()
let alias := first
let second := factory()
print(first())
print(alias())
print(second())
print(first())
"###,
            "2\n3\n2\n4\n",
        ),
        (
            "global_helper_reentry",
            r###"func relay(fn) { return fn(0) }
func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 return 0 } relay(step) return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "generator_reentry",
            r###"func* relay(fn) { yield fn(0) }
func factory() {
 mut data := 1
 func step(n) {
  if n == 0 { data = 9 return 0 }
  let generator := relay(step)
  for value in generator { assert_equal(value,0) }
  return data
 }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "mixed_global_captured_reentry",
            r###"func relay(fn) { return fn(0) }
func make_dispatch() { let marker := 5 func dispatch(fn) { return marker + relay(fn) } return dispatch }
let dispatch := make_dispatch()
func bridge(fn) { return dispatch(fn) }
func factory() {
 mut data := 1
 func step(n) { if n == 0 { data = 9 return 0 } assert_equal(bridge(step),5) return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "9\n9\n",
        ),
        (
            "shared_array_snapshot",
            r###"func factory() {
 mut data := [1,2]
 func step(n) { if n == 0 { data[0] = 9 return 0 } if n < 0 { return data } step(0) return data }
 return step
}
let step := factory()
let saved := step(-1)
print(step(1))
print(saved)
"###,
            "[9, 2]\n[1, 2]\n",
        ),
        (
            "immutable_capture_reentry",
            r###"func factory() {
 let data := 7
 func step(n) { if n == 0 { data = 9 } try { step(0) } except error {} return data }
 return step
}
let step := factory()
print(step(1))
print(step(1))
"###,
            "7\n7\n",
        ),
        (
            "read_only_recursive_snapshots",
            r###"func factory() {
 mut items := [1,2]
 func read(n) { if n == 0 { return items } return read(n - 1) }
 return read
}
let read := factory()
mut copy := read(32)
copy[0] = 9
print(read(32))
print(copy)
"###,
            "[1, 2]\n[9, 2]\n",
        ),
        (
            "writes_through_global_helper",
            r###"mut data := 1
func change() { data = 9 }
func factory() {
 func step(n) { if n == 0 { return data } change() return step(0) }
 return step
}
let step := factory()
print(step(1))
print(step(0))
"###,
            "9\n9\n",
        ),
    ];
    let dir = tempfile::tempdir().unwrap();
    let mut failures = Vec::new();
    for (name, source, expected) in cases {
        let path = dir.path().join(format!("{name}.kujo"));
        std::fs::write(&path, source).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
            command.args(["run", "--untrusted"]);
            if mode != "vm" {
                command.arg(format!("--{mode}"));
            }
            let output = command.arg(&path).output().unwrap();
            if !output.status.success() || String::from_utf8_lossy(&output.stdout) != *expected {
                failures.push(format!(
                    "{name}/{mode}: stdout={:?}, stderr={}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
