// Standalone exploratory probe; not a runtime implementation.
// Compile optimized against the existing non-LTO Kujo rlib plus its native search paths.
use kujo::interpreter::Value;
use std::{hint::black_box, sync::Arc, time::Instant};
#[inline(always)]
fn encode(value: &Value) -> Option<(i64,i64)> {
 match value {
  Value::Int(v)=>Some((*v,0)), Value::Bool(v)=>Some((i64::from(*v),1)),
  Value::Null=>Some((0,2)), Value::Float(v)=>Some((v.to_bits() as i64,3)), _=>None
 }
}
#[inline(never)]
fn original(value: &Value) -> Option<(i64,i64)> { encode(value) }
#[inline(never)]
fn integer_first(value: &Value) -> Option<(i64,i64)> {
 if let Value::Int(v)=value { return Some((*v,0)); }
 encode(value)
}
fn measure(values: &[Value], f: fn(&Value)->Option<(i64,i64)>) -> (u128,u64) {
 let start=Instant::now(); let mut sum=0u64;
 for i in 0..2_000_000 { if let Some((v,k))=black_box(f)(black_box(&values[i%values.len()])) {sum=sum.wrapping_add(v as u64).wrapping_add(k as u64);} }
 (start.elapsed().as_nanos(),black_box(sum))
}
fn main() {
 let edge=vec![Value::Int(i64::MIN),Value::Int(i64::MAX),Value::Float(-0.0),Value::Float(f64::from_bits(0x7ff8000000000042)),Value::Bool(true),Value::Bool(false),Value::Null,Value::Str(Arc::new("x".into())),Value::Array(Arc::new(vec![]))];
 for v in &edge {assert_eq!(original(v),integer_first(v));}
 let cases=vec![("integer",vec![Value::Int(42);64]),("float",vec![Value::Float(1.25);64]),("boolean",vec![Value::Bool(true);64]),("null",vec![Value::Null;64]),("owned",vec![Value::Str(Arc::new("x".into()));64]),("mixed",edge)];
 println!("{{\"value_size\":{},\"samples\":[",std::mem::size_of::<Value>());
 let mut comma="";
 for round in 0..7 {for (name,values) in &cases {
  let (a,b)=if round%2==0 {(measure(values,original),measure(values,integer_first))}else{let b=measure(values,integer_first);(measure(values,original),b)};
  assert_eq!(a.1,b.1);
  print!("{}{{\"round\":{},\"case\":\"{}\",\"before_ns\":{},\"after_ns\":{}}}",comma,round+1,name,a.0,b.0);comma=",";
 }} println!("]}}");
}
