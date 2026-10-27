#![allow(dead_code)]
use std::sync::Arc;
use std::mem::{size_of,align_of};
use kujo::interpreter::Value;
#[derive(Clone,Copy)] enum Scalar { Int(i64),Float(f64),Bool(bool),Null }
enum Selector { Field(String),String(Arc<String>),Integer(i64) }
struct Existing { selector:Selector,index:usize,value:Scalar }
#[derive(Clone,Copy)] struct Entry { index:usize,value:Scalar }
struct FourSlots { selector:Selector,entries:[Option<Entry>;4] }
fn main() {
 println!("Value={}/{} existing_row={}/{} entry={}/{} option_entry={}/{} four_slots={}/{}",size_of::<Value>(),align_of::<Value>(),size_of::<Existing>(),align_of::<Existing>(),size_of::<Entry>(),align_of::<Entry>(),size_of::<Option<Entry>>(),align_of::<Option<Entry>>(),size_of::<FourSlots>(),align_of::<FourSlots>());
 assert!(size_of::<FourSlots>()<=size_of::<Value>());
 assert!(align_of::<FourSlots>()<=align_of::<Value>());
}
