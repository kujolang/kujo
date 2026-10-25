// Isolated ownership-helper experiment, not an end-to-end benchmark.
// Copy this file into a temporary directory as probe.rs, then export:
// git show 2f00c5f:src/native_owned.rs > before.rs
// git show 5f22a2a:src/native_owned.rs > after.rs
// git show 5f22a2a:src/native_scalar.rs > scalar.rs
// Build with rustc --edition=2021 -C opt-level=3, --extern kujo=<debug rlib>,
// --extern nohash_hasher=<rlib>, -L dependency=target/debug/deps and native
// library search directories from target/debug/build/**/out. The debug rlib
// supplies Value and its destructors; both ownership implementations are
// compiled together at opt-level=3. Run the resulting executable once.
mod interpreter { pub use kujo::interpreter::Value; }
#[path="scalar.rs"] mod native_scalar;
#[path="before.rs"] mod before;
#[path="after.rs"] mod after;
use interpreter::Value;
use std::{hint::black_box,sync::Arc,time::Instant};
macro_rules! measurement {
 ($name:ident,$store:path) => {
 fn $name(value:&Value,kind:&str)->u128 {
  let mut roots=<$store>::default();let outer=roots.enter_frame().unwrap();
  for _ in 0..64 {let h=roots.insert(value.clone()).unwrap();roots.release(h).unwrap();}
  let start=Instant::now();
  for _ in 0..50_000 {
   if kind=="release" {let h=roots.insert(black_box(value).clone()).unwrap();roots.release(black_box(h)).unwrap();}
   else if kind=="take" {let h=roots.insert(black_box(value).clone()).unwrap();drop(black_box(roots.take(black_box(h)).unwrap()));}
   else {let frame=roots.enter_frame().unwrap();let n=if kind=="frame_four" {4}else{32};for _ in 0..n {black_box(roots.insert(black_box(value).clone()).unwrap());}roots.leave_frame(frame).unwrap();}
  }
  let elapsed=start.elapsed().as_nanos();roots.leave_frame(outer).unwrap();elapsed
 }
 }
}
measurement!(old,before::NativeOwnedValues);
measurement!(new,after::NativeOwnedValues);
fn main(){
 let shared=Arc::new(vec![Value::Int(7),Value::Int(11)]);let value=Value::Array(shared.clone());
 println!("{{\"value_size\":{},\"samples\":[",std::mem::size_of::<Value>());let mut comma="";
 for round in 0..7 {for kind in ["release","take","frame_four","frame_32"] {
  let (a,b)=if round%2==0 {(old(&value,kind),new(&value,kind))}else{let b=new(&value,kind);(old(&value,kind),b)};
  assert_eq!(Arc::strong_count(&shared),2);
  print!("{}{{\"round\":{},\"case\":\"{}\",\"before_ns\":{},\"after_ns\":{}}}",comma,round+1,kind,a,b);comma=",";
 }}println!("]}}");
}
