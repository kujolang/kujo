use std::sync::{Arc, Mutex, Weak};
#[allow(dead_code)] struct Existing { owner: Weak<Mutex<()>>, limit: usize }
#[allow(dead_code)] struct KeyView { owner: Weak<Mutex<()>>, limit: usize, keys: Vec<std::collections::HashSet<String>> }
#[allow(dead_code)] enum Candidate { Existing { owner: Weak<Mutex<()>>, limit: usize }, KeyView(Arc<KeyView>) }
fn main() {
 println!("existing={} candidate={} existing_option={} candidate_option={}",std::mem::size_of::<Existing>(),std::mem::size_of::<Candidate>(),std::mem::size_of::<Option<Existing>>(),std::mem::size_of::<Option<Candidate>>());
}
