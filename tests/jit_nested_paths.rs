use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn native_nested_writes_preserve_values_errors_and_evaluation_order() {
    let mut source = String::from(
        r#"
struct Holder { rows: any }
func write(seed,index,value) { mut state := seed state["rows"][index][0] = value return state }
func field(seed,value) { mut state := seed state.rows[0][0] = value return state }
func immutable(seed,value) { let state := seed state["rows"][0][0] = value }
mut state := {"rows": [[1,2]]}
func global_write(value) { state["rows"][0][0] = value }
const fixed := {"rows": [[1,2]]}
func const_write(value) { fixed["rows"][0][0] = value }
mut order := ""
func value_first() { order += "v" state = {"rows": [[7,8]]} return 9 }
func key_second() { order += "k" return "rows" }
func row_third() { order += "r" return 0 }
func ordered() { state[key_second()][row_third()][0] = value_first() }
let seed := {"rows": [[1,2]]}
let holder := Holder { rows: [[1,2]] }
mut caught := 0
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(write(seed,0,9),{"rows": [[9,2]]})
assert_equal(write(seed,-1,"snow"),{"rows": [["snow",2]]})
assert_equal(field(holder,true),Holder { rows: [[true,2]] })
global_write(5)
assert_equal(state,{"rows": [[5,2]]})
order = ""
ordered()
assert_equal(order,"vkr")
assert_equal(state,{"rows": [[9,8]]})
state = {"rows": [[1,2]]}
try { immutable(seed,9) } except error { caught += 1 }
try { const_write(9) } except error { caught += 1 }
"#,
        );
    }
    source.push_str(
        r#"
mut result := 17
try { result = write(seed,99,9) } except error { caught += 1 }
try { result = write({"rows": [1]},0,9) } except error { caught += 1 }
assert_equal(result,17)
assert_equal(caught,402)
assert_equal(seed,{"rows": [[1,2]]})
assert_equal(holder,Holder { rows: [[1,2]] })
assert_equal(fixed,{"rows": [[1,2]]})
assert_equal(write(seed,0,null),{"rows": [[null,2]]})
print("PASS")
"#,
    );
    check(source, &["write", "field", "immutable", "global_write", "const_write", "ordered"]);
}

#[test]
fn native_nested_loop_writes_preserve_local_global_snapshots() {
    check(
        r#"
mut global := {"rows": [[0,1]]}
let saved_global := global
func once() {
    mut local := {"rows": [[0,1]]}
    let saved := local
    mut i := 0
    while i < 200 {
        local["rows"][0][0] = i
        global["rows"][0][0] = i
        i += 1
    }
    return [local,saved]
}
assert_equal(once(),[{"rows": [[199,1]]},{"rows": [[0,1]]}])
assert_equal(global,{"rows": [[199,1]]})
assert_equal(saved_global,{"rows": [[0,1]]})
print("PASS")
"#
        .into(),
        &[],
    );
}

fn check(source: String, names: &[&str]) {
    check_modes(source, names, true);
}

fn check_modes(source: String, names: &[&str], expect_native: bool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("guarded_collections.kujo");
    std::fs::write(&path, source).unwrap();
    for mode in [Some("--interpreter"), None, Some("--jit")] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--scheduler-no-timeout"]);
        if names.is_empty() {
            command.env("DISABLE_FUNCTION_JIT", "1").env("DEBUG_JIT_LOOP", "1");
        }
        if let Some(mode) = mode {
            command.arg(mode);
        }
        let metrics = directory.path().join("metrics.json");
        if mode == Some("--jit") {
            command.env("DEBUG_JIT", "1").arg("--measurements").arg(&metrics);
        }
        let mut child =
            command.arg(&path).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
        let started = Instant::now();
        while child.try_wait().unwrap().is_none() {
            if started.elapsed() > Duration::from_secs(30) {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("{mode:?} timed out: {}", String::from_utf8_lossy(&output.stderr));
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let output = child.wait_with_output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{mode:?}: {stderr}");
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "PASS");
        if mode == Some("--jit") {
            for name in names {
                assert!(
                    stderr.contains(&format!("Successfully compiled function '{name}'")),
                    "{stderr}"
                );
            }
            let report: serde_json::Value =
                serde_json::from_slice(&std::fs::read(metrics).unwrap()).unwrap();
            if !expect_native {
                assert!(!stderr.contains("Executing compiled loop"), "{stderr}");
                assert_eq!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap(), 0);
            } else if names.is_empty() {
                assert!(stderr.contains("Executing compiled loop"), "{stderr}");
                assert!(report["counters"]["jit_native_boundary_entries"].as_u64().unwrap() > 0);
            } else {
                assert!(report["counters"]["jit_native_function_entries"].as_u64().unwrap() >= 100);
            }
        }
    }
}

#[test]
fn captured_path_loop_does_not_mutate_same_named_global() {
    check_modes(
        r#"
mut state := {"rows": [[999,1]]}
func outer() {
    mut state := {"rows": [[0,1]]}
    func work() {
        mut i := 0
        while i < 200 { state["rows"][0][0] = i i += 1 }
        return state
    }
    return work
}
let work := outer()
assert_equal(work(),{"rows": [[199,1]]})
assert_equal(state,{"rows": [[999,1]]})
print("PASS")
"#
        .into(),
        &[],
        false,
    );
}

#[test]
fn recursive_native_nested_writes_preserve_caller_snapshots() {
    check(
        r#"
        func visit(input,left,right) {
            mut state := input
            if left >= right { state[0][left] = state[0][left] + 1 return state }
            let middle := to_int((left + right) / 2)
            state = visit(state,left,middle)
            state = visit(state,middle + 1,right)
            return state
        }
        let seed := [range(0,512)]
        let result := visit(seed,0,511)
        assert_equal(result,[range(1,513)])
        assert_equal(seed,[range(0,512)])
        print("PASS")
        "#
        .into(),
        &["visit"],
    );
}

#[test]
fn native_rebinding_under_caller_handlers_preserves_failed_targets_and_snapshots() {
    let mut source = String::from(
        r#"
func alter(input, fail) {
    mut state := input
    state[0][0] = state[0][0] + 1
    if fail { return 1 / 0 }
    return state
}
func rewrite(input, fail) {
    mut state := input
    state = alter(state, fail)
    return state
}
func catch_here(input) {
    mut state := input
    try { state = alter(state, true) } except error {}
    return state
}
let seed := [[0,1]]
mut result := [[99]]
mut caught := 0
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
try { assert_equal(rewrite(seed,false),[[1,1]]) } except error { assert(false) }
try { result = rewrite(seed,true) } except error { caught += 1 }
assert_equal(catch_here(seed),seed)
"#,
        );
    }
    source.push_str(
        r#"
assert_equal(caught,200)
assert_equal(result,[[99]])
assert_equal(seed,[[0,1]])
print("PASS")
"#,
    );
    check(source, &["alter", "rewrite"]);
}

#[test]
fn native_field_replacement_preserves_struct_snapshots() {
    let mut source = String::from(
        r#"
struct Box { item: any }
func replace(input,value) { mut state := input state.item = value return state }
let object := Box { item: [1] }
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(replace(object,[9]),Box { item: [9] })
assert_equal(replace(object,null),Box { item: null })
assert_equal(replace(object,"snow"),Box { item: "snow" })
"#,
        );
    }
    source.push_str(
        r#"
mut unchanged := 7
mut caught := 0
try { unchanged = replace(1,9) } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(unchanged,7)
assert_equal(object,Box { item: [1] })
print("PASS")
"#,
    );
    check(source, &["replace"]);
}

#[test]
fn native_field_loop_preserves_retained_struct_snapshot() {
    check(
        r#"
struct Box { item: any }
func once() {
    mut object := Box { item: 0 }
    let saved := object
    mut i := 0
    while i < 200 { object.item = i i += 1 }
    return [object,saved]
}
assert_equal(once(),[Box { item: 199 },Box { item: 0 }])
print("PASS")
"#
        .into(),
        &[],
    );
}

#[test]
fn native_guarded_path_appends_preserve_snapshots_and_shadowed_calls() {
    let mut source = String::from(
        r#"
struct Bag { items: any, data: any }
func field(input,item) { mut state := input state.items = push(state.items,item) return state }
func nested(input,item) { mut state := input state["rows"][0] = push(state["rows"][0],item) return state }
func bytes_field(input,item) { mut state := input state.data = concat(state.data,item) return state }
func custom(input,item) { return [item,input] }
func shadow(input,operation) { let push := operation mut state := input state.items = push(state.items,9) return state }
let seed := Bag { items: [1], data: bytes([1]) }
let rows := {"rows": [[1]]}
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
assert_equal(field(seed,2).items,[1,2])
assert_equal(nested(rows,2),{"rows": [[1,2]]})
assert_equal(bytes_field(seed,bytes([2])).data,bytes([1,2]))
assert_equal(shadow(seed,custom).items,[9,[1]])
"#,
        );
    }
    source.push_str(
        r#"
assert_equal(seed.items,[1])
assert_equal(seed.data,bytes([1]))
assert_equal(rows,{"rows": [[1]]})
print("PASS")
"#,
    );
    check(source, &["field", "nested", "bytes_field", "shadow"]);
}

#[test]
fn native_path_append_order_and_loop_snapshots() {
    let mut source = String::from(
        r#"
mut global := {"rows": [[1]]}
mut order := ""
func item() { order += "v" global = {"rows": [[7]]} return 2 }
func index() { order += "i" global = {"rows": [[9]]} return 0 }
func change() { global["rows"][index()] = push(global["rows"][0],item()) }
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
global = {"rows": [[1]]}
order = ""
change()
assert_equal(global,{"rows": [[1,2]]})
assert_equal(order,"vi")
"#,
        );
    }
    source.push_str("print(\"PASS\")");
    check(source, &["change"]);
    check(
        r#"
struct Bag { items: any }
func once() {
    mut state := Bag { items: [] }
    let saved := state
    mut i := 0
    while i < 200 { state.items = push(state.items,i) i += 1 }
    assert_equal(state.items,range(0,200))
    assert_equal(saved.items,[])
}
once()
print("PASS")
"#
        .into(),
        &[],
    );
}

#[test]
fn native_readonly_stores_preserve_initialization_and_reassignment_errors() {
    let mut source = String::from(
        r#"
func readonly_int(input) { let value := input value = 2 return value }
func readonly_array(input) { const value := [input] value = [2] return value }
func redefined(input) {
    mut sum := 0
    mut i := 0
    while i < 3 { let value := input + i sum += value i += 1 }
    return sum
}
mut caught := 0
mut result := 17
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
try { result = readonly_int(1) } except error { caught += 1 }
try { result = readonly_array(1) } except error { caught += 1 }
assert_equal(redefined(3),12)
"#,
        );
    }
    source.push_str("assert_equal(caught,400) assert_equal(result,17) print(\"PASS\")");
    check(source, &["readonly_int", "readonly_array", "redefined"]);
    check(
        r#"
func loop_readonly() {
    let value := 1
    mut i := 0
    while i < 200 {
        i += 1
        if i == 150 { value = 2 }
    }
    return value
}
func loop_definitions() {
    mut i := 0
    mut sum := 0
    while i < 200 { let value := i sum += value i += 1 }
    return sum
}
mut caught := 0
mut result := 17
try { result = loop_readonly() } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(result,17)
assert_equal(loop_definitions(),19900)
print("PASS")
"#
        .into(),
        &[],
    );
}

#[test]
fn native_generic_index_replacement_preserves_snapshots_and_errors() {
    let mut source = String::from(
        r#"
mut state := [0]
func put(index,value) { state[index] = value return state }
mut order := ""
func rhs() { order += "v" state = [7,8] return 9 }
func key() { order += "k" return 1 }
func ordered_index() { state[key()] = rhs() return state }
let array_seed := [1,2]
let bytes_seed := bytes([1,2])
let dict_seed := {"a": [1]}
mut caught := 0
"#,
    );
    for _ in 0..200 {
        source.push_str(
            r#"
state = array_seed
assert_equal(put(-1,"x"),[1,"x"])
state = bytes_seed
assert_equal(put(1,255),bytes([1,255]))
state = dict_seed
assert_equal(put("a",[2]),{"a": [2]})
assert_equal(put("b",null),{"a": [2],"b": null})
state = {}
put(0,11)
put(2,33)
put(1,22)
assert_equal(state[0],11)
assert_equal(state[1],22)
assert_equal(state[2],33)
put("1","text")
assert_equal(state[1],"text")
put(-1,44)
assert_equal(state[-1],44)
put("name",[5])
assert_equal(state["name"],[5])
order = ""
assert_equal(ordered_index(),[7,9])
assert_equal(order,"vk")
"#,
        );
    }
    source.push_str(
        r#"
state = array_seed
try { put(99,3) } except error { caught += 1 }
assert_equal(state,array_seed)
try { put("wrong",3) } except error { caught += 1 }
assert_equal(state,array_seed)
state = bytes_seed
try { put(0,256) } except error { caught += 1 }
assert_equal(state,bytes_seed)
state = 17
try { put(0,3) } except error { caught += 1 }
assert_equal(state,17)
assert_equal(caught,4)
assert_equal(array_seed,[1,2])
assert_equal(bytes_seed,bytes([1,2]))
assert_equal(dict_seed,{"a": [1]})
print("PASS")
"#,
    );
    check(source, &["put", "ordered_index"]);
}

#[test]
fn native_generic_index_loops_preserve_local_types_and_readonly_errors() {
    check(
        r#"
func work(seed) {
    mut state := seed
    let saved := state
    mut i := 0
    mut total := 0
    while i < 200 {
        state[0] = i
        total += state[0]
        i += 1
    }
    assert_equal(total,19900)
    assert_equal(state[0],199)
    assert_equal(saved,seed)
    return state
}
func changing() {
    mut state := {}
    state[0] = 0
    let saved := state
    mut i := 0
    while i < 200 {
        if i == 150 { state[0] = "changed" } else { state[0] = i }
        if i == 150 { assert_equal(state[0],"changed") } else { assert_equal(state[0],i) }
        i += 1
    }
    assert_equal(saved[0],0)
    assert_equal(state[0],199)
}
func readonly() {
    let state := [1]
    mut i := 0
    while i < 200 {
        i += 1
        if i == 150 { state[0] = 2 }
    }
    return state
}
assert_equal(work([0]),[199])
assert_equal(work(bytes([0])),bytes([199]))
changing()
mut caught := 0
mut result := 17
try { result = readonly() } except error { caught += 1 }
assert_equal(caught,1)
assert_equal(result,17)
print("PASS")
"#
        .into(),
        &[],
    );
}
