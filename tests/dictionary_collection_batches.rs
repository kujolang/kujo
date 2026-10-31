use std::process::Command;

#[test]
fn dictionary_collection_batches_preserve_snapshots_promotions_and_publication() {
    let literal =
        format!("{{{}}}", (0..64).map(|i| format!("\"{i}\":0")).collect::<Vec<_>>().join(","));
    let owner = format!("mut state := {literal}\n")
        + r###"
struct Trigger { value:int, func op_add(other) { assert_equal(state[0],[other]) state["operator"]=other return value+other } }
export func operators(n) { state[0]=[n] state[1]=Trigger { value:n } return state[1]+n }
export func local_operator(n) { mut values:=[0] values[0]=Trigger { value:n } return values[0]+n }
export func snapshot() { return state }
export func arrays(n) { state[0]=[n] state[1]=[n+1] state[2]=[n+2] return n }
export func mixed(n) { state[0]=n state[1]=[n] state[2]={"v":n} return n }
export func reverse(n) { state[0]=[n] state[1]=n state[2]={"v":n} return n }
export func structs(n) { state[0]=Cell { value:n } state[1]=Cell { value:n+1 } state[2]=Cell { value:n+2 } return n }
export func same(n) { state[0]=n state["0"]=[n] state[0]={"v":n} return n }
export func pair(n) { state[0]=n state[1]=n+1 state[2]=[n+2] return n }
export func many(n) { state[0]=n state[1]=n+1 state[2]=n+2 state[3]=[n+3] return n }
export func inserted(n) { state["new-a"]=[n] state["new-b"]={"v":n} state["new-c"]=Cell { value:n } return n }
export func dense(n,count) { mut j:=0 while j<count { state[j]=[n+j] j+=1 } state[0]=[n+99] return n }
export func failure(n,zero) { state[0]=[n] state[1]={"v":n} state[2]=[n/zero] return n }
func observe() { return state[0] }
export func callback(n) { state[0]=[n] state[1]=[n+1] state[2]=observe() return n }
export func replacement(n) { state[0]=[n] state[1]=[n+1] state={"0":n} return n }
"###;
    let main = r###"
from owner import operators, local_operator, snapshot, arrays, mixed, reverse, structs, same, pair, many, inserted, dense, failure, callback, replacement
let original:=snapshot()
arrays(10) let saved:=snapshot()
mixed(20) assert_equal(snapshot()[0],20) assert_equal(snapshot()[1],[20]) assert_equal(snapshot()[2],{"v":20})
reverse(30) assert_equal(snapshot()[0],[30]) assert_equal(snapshot()[1],30)
structs(40) assert_equal(snapshot()[0].value,40) assert_equal(snapshot()[2].value,42)
same(50) assert_equal(snapshot()[0],{"v":50})
pair(60) assert_equal(snapshot()[0],60) assert_equal(snapshot()[1],61) assert_equal(snapshot()[2],[62])
many(70) assert_equal(snapshot()[2],72) assert_equal(snapshot()[3],[73])
inserted(80) assert_equal(snapshot()["new-a"],[80]) assert_equal(snapshot()["new-b"],{"v":80}) assert_equal(snapshot()["new-c"].value,80)
let before:=snapshot() mut caught:=false
try { failure(90,0) } except err { caught=true }
assert(caught) assert_equal(snapshot()[0],[90]) assert_equal(snapshot()[1],{"v":90}) assert_equal(snapshot()[2],before[2])
callback(100) assert_equal(snapshot()[2],[100])
dense(110,64) let final:=snapshot()
assert_equal(final[0],[209]) mut j:=1 while j<64 { assert_equal(final[j],[110+j]) j+=1 }
assert_equal(final["new-a"],[80])
assert_equal(operators(130),260) assert_equal(snapshot()["operator"],130)
assert_equal(local_operator(130),260)
replacement(120) assert_equal(snapshot(),{"0":120})
assert_equal(final[0],[209]) assert_equal(saved[0],[10]) assert_equal(saved[1],[11]) assert_equal(saved[2],[12])
j=0 while j<64 { assert_equal(original[j],0) if j>2 { assert_equal(saved[j],0) } j+=1 }
print("DICTIONARY BATCH PASS")
"###;
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
    let path = directory.path().join("main.kujo");
    std::fs::write(&path, main).unwrap();
    for mode in ["vm", "interpreter", "jit"] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_kujo"));
        command.args(["run", "--untrusted", "--scheduler-no-timeout"]);
        if mode != "vm" {
            command.arg(format!("--{mode}"));
        }
        let output = command.arg(&path).output().unwrap();
        assert!(output.status.success(), "{mode}: {}", String::from_utf8_lossy(&output.stderr));
        assert_eq!(output.stdout, b"DICTIONARY BATCH PASS\n", "{mode}");
    }
}
