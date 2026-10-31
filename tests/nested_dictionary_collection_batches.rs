use std::process::Command;

#[test]
fn nested_text_and_bytes_preserve_aliases_and_error_publication() {
    let leaf =
        format!("{{{}}}", (0..32).map(|i| format!("\"{i}\":0")).collect::<Vec<_>>().join(","));
    for (shape, root, path) in [
        ("dict", format!("{{\"inner\":{leaf}}}"), "state[\"inner\"]"),
        ("struct", format!("Holder {{ row:{leaf} }}"), "state.row"),
        ("deep", format!("{{\"outer\":{{\"inner\":{leaf}}}}}"), "state[\"outer\"][\"inner\"]"),
    ] {
        let owner = format!(
            r#"
mut state:={root}
export func snapshot() {{ return {path} }}
export func text() {{ {path}[0]="left" {path}[1]="right" }}
export func raw(value) {{ {path}[0]=value {path}[1]=value }}
export func inserted(value) {{ {path}["new-a"]="added" {path}["new-b"]=value }}
export func repeated() {{ mut j:=0 while j<64 {{ {path}[0]="repeat" {path}["0"]="last" j+=1 }} }}
export func failure(zero) {{ {path}[0]="published" {path}[1]="also" {path}[2]=1/zero }}
"#
        );
        let main = r#"
from owner import snapshot,text,raw,inserted,repeated,failure
let original:=snapshot()
text() let strings:=snapshot()
assert_equal(strings[0],"left") assert_equal(strings[1],"right")
let payload:=bytes([0,127,128,255])
raw(payload) let binary:=snapshot()
assert_equal(binary[0],payload) assert_equal(binary[1],payload)
inserted(payload) let added:=snapshot()
assert_equal(added["new-a"],"added") assert_equal(added["new-b"],payload)
assert_equal(len(binary),32) assert_equal(len(added),34)
repeated() assert_equal(snapshot()[0],"last") assert_equal(binary[0],payload)
mut caught:=false
try { failure(0) } except err { caught=true }
assert(caught) let final:=snapshot()
assert_equal(final[0],"published") assert_equal(final[1],"also") assert_equal(final[2],0)
assert_equal(strings[0],"left") assert_equal(added[0],payload)
mut i:=0 while i<32 { assert_equal(original[i],0) if i>=2 { assert_equal(final[i],0) } i+=1 }
print("NESTED TEXT BYTES PASS")
"#;
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
            assert_eq!(output.stdout, b"NESTED TEXT BYTES PASS\n", "{shape}/{mode}");
        }
    }
}
#[test]
fn nested_dictionary_batches_preserve_snapshots_callbacks_and_replaced_targets() {
    for (shape, owner, main) in [
        (
            "dict",
            r####"mut state:={"inner":{"0":0,"1":0,"2":0,"3":0,"4":0,"5":0,"6":0,"7":0,"8":0,"9":0,"10":0,"11":0,"12":0,"13":0,"14":0,"15":0,"16":0,"17":0,"18":0,"19":0,"20":0,"21":0,"22":0,"23":0,"24":0,"25":0,"26":0,"27":0,"28":0,"29":0,"30":0,"31":0,"32":0,"33":0,"34":0,"35":0,"36":0,"37":0,"38":0,"39":0,"40":0,"41":0,"42":0,"43":0,"44":0,"45":0,"46":0,"47":0,"48":0,"49":0,"50":0,"51":0,"52":0,"53":0,"54":0,"55":0,"56":0,"57":0,"58":0,"59":0,"60":0,"61":0,"62":0,"63":0}}

struct Trigger { value:int, func op_add(other) { assert_equal(state["inner"][0],[other]) state["inner"]["operator"]=other return value+other } }
export func operators(n) { state["inner"][0]=[n] state["inner"][1]=Trigger { value:n } return state["inner"][1]+n }
export func local_operator(n) { mut values:=[0] values[0]=Trigger { value:n } return values[0]+n }
export func snapshot() { return state["inner"] }
export func arrays(n) { state["inner"][0]=[n] state["inner"][1]=[n+1] state["inner"][2]=[n+2] return n }
export func mixed(n) { state["inner"][0]=n state["inner"][1]=[n] state["inner"][2]={"v":n} return n }
export func reverse(n) { state["inner"][0]=[n] state["inner"][1]=n state["inner"][2]={"v":n} return n }
export func structs(n) { state["inner"][0]=Cell { value:n } state["inner"][1]=Cell { value:n+1 } state["inner"][2]=Cell { value:n+2 } return n }
export func same(n) { state["inner"][0]=n state["inner"]["0"]=[n] state["inner"][0]={"v":n} return n }
export func pair(n) { state["inner"][0]=n state["inner"][1]=n+1 state["inner"][2]=[n+2] return n }
export func many(n) { state["inner"][0]=n state["inner"][1]=n+1 state["inner"][2]=n+2 state["inner"][3]=[n+3] return n }
export func inserted(n) { state["inner"]["new-a"]=[n] state["inner"]["new-b"]={"v":n} state["inner"]["new-c"]=Cell { value:n } return n }
export func dense(n,count) { mut j:=0 while j<count { state["inner"][j]=[n+j] j+=1 } state["inner"][0]=[n+99] return n }
export func failure(n,zero) { state["inner"][0]=[n] state["inner"][1]={"v":n} state["inner"][2]=[n/zero] return n }
func observe() { return state["inner"][0] }
export func callback(n) { state["inner"][0]=[n] state["inner"][1]=[n+1] state["inner"][2]=observe() return n }
export func replacement(n) { state["inner"][0]=[n] state["inner"][1]=[n+1] state["inner"]={"0":n} return n }

export func replace_root(n) { state["inner"][0]=[n] state["inner"][1]=[n+1] state={"inner":{"0":n,"1":0}} return n }
func replace_index() { state={"inner":{"0":0,"1":0}} return 0 }
export func replace_during_index(n) { state["inner"][replace_index()]=[n] return n }
"####,
            r####"
from owner import replace_root, replace_during_index, operators, local_operator, snapshot, arrays, mixed, reverse, structs, same, pair, many, inserted, dense, failure, callback, replacement
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
let before_root:=snapshot()
assert_equal(replace_root(140),140) assert_equal(snapshot()[0],140) assert_equal(before_root[0],120)
let before_index:=snapshot()
assert_equal(replace_during_index(150),150) assert_equal(snapshot()[0],[150]) assert_equal(snapshot()[1],0) assert_equal(before_index[0],140)
print("NESTED DICTIONARY BATCH PASS")

"####,
        ),
        (
            "struct",
            r####"mut state:=Holder { row:{"0":0,"1":0,"2":0,"3":0,"4":0,"5":0,"6":0,"7":0,"8":0,"9":0,"10":0,"11":0,"12":0,"13":0,"14":0,"15":0,"16":0,"17":0,"18":0,"19":0,"20":0,"21":0,"22":0,"23":0,"24":0,"25":0,"26":0,"27":0,"28":0,"29":0,"30":0,"31":0,"32":0,"33":0,"34":0,"35":0,"36":0,"37":0,"38":0,"39":0,"40":0,"41":0,"42":0,"43":0,"44":0,"45":0,"46":0,"47":0,"48":0,"49":0,"50":0,"51":0,"52":0,"53":0,"54":0,"55":0,"56":0,"57":0,"58":0,"59":0,"60":0,"61":0,"62":0,"63":0} }

struct Trigger { value:int, func op_add(other) { assert_equal(state.row[0],[other]) state.row["operator"]=other return value+other } }
export func operators(n) { state.row[0]=[n] state.row[1]=Trigger { value:n } return state.row[1]+n }
export func local_operator(n) { mut values:=[0] values[0]=Trigger { value:n } return values[0]+n }
export func snapshot() { return state.row }
export func arrays(n) { state.row[0]=[n] state.row[1]=[n+1] state.row[2]=[n+2] return n }
export func mixed(n) { state.row[0]=n state.row[1]=[n] state.row[2]={"v":n} return n }
export func reverse(n) { state.row[0]=[n] state.row[1]=n state.row[2]={"v":n} return n }
export func structs(n) { state.row[0]=Cell { value:n } state.row[1]=Cell { value:n+1 } state.row[2]=Cell { value:n+2 } return n }
export func same(n) { state.row[0]=n state.row["0"]=[n] state.row[0]={"v":n} return n }
export func pair(n) { state.row[0]=n state.row[1]=n+1 state.row[2]=[n+2] return n }
export func many(n) { state.row[0]=n state.row[1]=n+1 state.row[2]=n+2 state.row[3]=[n+3] return n }
export func inserted(n) { state.row["new-a"]=[n] state.row["new-b"]={"v":n} state.row["new-c"]=Cell { value:n } return n }
export func dense(n,count) { mut j:=0 while j<count { state.row[j]=[n+j] j+=1 } state.row[0]=[n+99] return n }
export func failure(n,zero) { state.row[0]=[n] state.row[1]={"v":n} state.row[2]=[n/zero] return n }
func observe() { return state.row[0] }
export func callback(n) { state.row[0]=[n] state.row[1]=[n+1] state.row[2]=observe() return n }
export func replacement(n) { state.row[0]=[n] state.row[1]=[n+1] state.row={"0":n} return n }

export func replace_root(n) { state.row[0]=[n] state.row[1]=[n+1] state=Holder { row:{"0":n,"1":0} } return n }
func replace_index() { state=Holder { row:{"0":0,"1":0} } return 0 }
export func replace_during_index(n) { state.row[replace_index()]=[n] return n }
"####,
            r####"
from owner import replace_root, replace_during_index, operators, local_operator, snapshot, arrays, mixed, reverse, structs, same, pair, many, inserted, dense, failure, callback, replacement
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
let before_root:=snapshot()
assert_equal(replace_root(140),140) assert_equal(snapshot()[0],140) assert_equal(before_root[0],120)
let before_index:=snapshot()
assert_equal(replace_during_index(150),150) assert_equal(snapshot()[0],[150]) assert_equal(snapshot()[1],0) assert_equal(before_index[0],140)
print("NESTED DICTIONARY BATCH PASS")

"####,
        ),
        (
            "deep-dict",
            r####"mut state:={"inner":{"leaf":{"0":0,"1":0,"2":0,"3":0,"4":0,"5":0,"6":0,"7":0,"8":0,"9":0,"10":0,"11":0,"12":0,"13":0,"14":0,"15":0,"16":0,"17":0,"18":0,"19":0,"20":0,"21":0,"22":0,"23":0,"24":0,"25":0,"26":0,"27":0,"28":0,"29":0,"30":0,"31":0,"32":0,"33":0,"34":0,"35":0,"36":0,"37":0,"38":0,"39":0,"40":0,"41":0,"42":0,"43":0,"44":0,"45":0,"46":0,"47":0,"48":0,"49":0,"50":0,"51":0,"52":0,"53":0,"54":0,"55":0,"56":0,"57":0,"58":0,"59":0,"60":0,"61":0,"62":0,"63":0}}}

struct Trigger { value:int, func op_add(other) { assert_equal(state["inner"]["leaf"][0],[other]) state["inner"]["leaf"]["operator"]=other return value+other } }
export func operators(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=Trigger { value:n } return state["inner"]["leaf"][1]+n }
export func local_operator(n) { mut values:=[0] values[0]=Trigger { value:n } return values[0]+n }
export func snapshot() { return state["inner"]["leaf"] }
export func arrays(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=[n+1] state["inner"]["leaf"][2]=[n+2] return n }
export func mixed(n) { state["inner"]["leaf"][0]=n state["inner"]["leaf"][1]=[n] state["inner"]["leaf"][2]={"v":n} return n }
export func reverse(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=n state["inner"]["leaf"][2]={"v":n} return n }
export func structs(n) { state["inner"]["leaf"][0]=Cell { value:n } state["inner"]["leaf"][1]=Cell { value:n+1 } state["inner"]["leaf"][2]=Cell { value:n+2 } return n }
export func same(n) { state["inner"]["leaf"][0]=n state["inner"]["leaf"]["0"]=[n] state["inner"]["leaf"][0]={"v":n} return n }
export func pair(n) { state["inner"]["leaf"][0]=n state["inner"]["leaf"][1]=n+1 state["inner"]["leaf"][2]=[n+2] return n }
export func many(n) { state["inner"]["leaf"][0]=n state["inner"]["leaf"][1]=n+1 state["inner"]["leaf"][2]=n+2 state["inner"]["leaf"][3]=[n+3] return n }
export func inserted(n) { state["inner"]["leaf"]["new-a"]=[n] state["inner"]["leaf"]["new-b"]={"v":n} state["inner"]["leaf"]["new-c"]=Cell { value:n } return n }
export func dense(n,count) { mut j:=0 while j<count { state["inner"]["leaf"][j]=[n+j] j+=1 } state["inner"]["leaf"][0]=[n+99] return n }
export func failure(n,zero) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]={"v":n} state["inner"]["leaf"][2]=[n/zero] return n }
func observe() { return state["inner"]["leaf"][0] }
export func callback(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=[n+1] state["inner"]["leaf"][2]=observe() return n }
export func replacement(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=[n+1] state["inner"]["leaf"]={"0":n} return n }

export func replace_root(n) { state["inner"]["leaf"][0]=[n] state["inner"]["leaf"][1]=[n+1] state={"inner":{"leaf":{"0":n,"1":0}}} return n }
func replace_index() { state={"inner":{"leaf":{"0":0,"1":0}}} return 0 }
export func replace_during_index(n) { state["inner"]["leaf"][replace_index()]=[n] return n }
"####,
            r####"
from owner import replace_root, replace_during_index, operators, local_operator, snapshot, arrays, mixed, reverse, structs, same, pair, many, inserted, dense, failure, callback, replacement
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
let before_root:=snapshot()
assert_equal(replace_root(140),140) assert_equal(snapshot()[0],140) assert_equal(before_root[0],120)
let before_index:=snapshot()
assert_equal(replace_during_index(150),150) assert_equal(snapshot()[0],[150]) assert_equal(snapshot()[1],0) assert_equal(before_index[0],140)
print("NESTED DICTIONARY BATCH PASS")

"####,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("owner.kujo"), owner).unwrap();
        let source = directory.path().join("main.kujo");
        std::fs::write(&source, main).unwrap();
        for mode in ["vm", "interpreter", "jit"] {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_kujo"));
            cmd.args(["run", "--untrusted", "--scheduler-no-timeout"]);
            if mode != "vm" {
                cmd.arg(format!("--{mode}"));
            }
            let out = cmd.arg(&source).output().unwrap();
            assert!(
                out.status.success(),
                "{shape}/{mode}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
            assert_eq!(out.stdout, b"NESTED DICTIONARY BATCH PASS\n", "{shape}/{mode}");
        }
    }
}
