# Diagnostic-only source overlay. Never apply this to the accepted worktree.
require 'digest'
require 'json'
require 'open3'
root = File.realpath(ARGV.fetch(0))
raise 'isolated worktree required' if root == File.realpath('.')
head, status = Open3.capture2('git', '-C', root, 'rev-parse', 'HEAD')
raise 'wrong baseline' unless status.success? && head.strip.start_with?('f58f0a9')
changes = []
edit = lambda do |path, before, after|
  file = File.join(root, path)
  source = File.read(file)
  raise "ambiguous anchor #{path}" unless source.scan(before).size == 1
  original = Digest::SHA256.hexdigest(source)
  source = source.sub(before, after)
  File.write(file, source)
  changes << {path: path, before_sha256: original, after_sha256: Digest::SHA256.hexdigest(source)}
end
edit.call('src/lib.rs', 'pub mod interpreter;', "pub mod interpreter;\npub mod capture_census;")
labels = %w[activation_calls activation_global_bindings activation_local_bindings membership_checks membership_names prefix_snapshot_calls prefix_snapshot_bindings pending_prefix_clones bridge_snapshots reentry_fallback_snapshots]
File.write(root+'/src/capture_census.rs', <<~RS)
  // Diagnostic build only: process-wide counts include worker threads.
  use std::sync::atomic::{AtomicU64, Ordering};
  static COUNTS: [AtomicU64; #{labels.size}] = [const { AtomicU64::new(0) }; #{labels.size}];
  pub fn add(index: usize, count: usize) { COUNTS[index].fetch_add(count as u64, Ordering::Relaxed); }
  pub fn snapshot() -> [u64; #{labels.size}] { std::array::from_fn(|i| COUNTS[i].load(Ordering::Relaxed)) }
RS
p='src/interpreter/environment.rs'
a='        let mut captured = owner.lock().unwrap_or_else(|p| p.into_inner());'
edit.call(p,a,a+"\n        crate::capture_census::add(0, 1);\n        crate::capture_census::add(1, captured.scopes.first().map_or(0, |s| s.len()));\n        crate::capture_census::add(2, captured.scopes.iter().skip(1).map(|s| s.len()).sum());")
a="        if self.pending_name().is_some() {\n            return self.clone().capture_prefix_snapshot(count, pool);"
edit.call(p,a,"        crate::capture_census::add(5, 1);\n        crate::capture_census::add(6, self.scopes[..count].iter().map(|s| s.len()).sum());\n"+a.sub('            return',"            crate::capture_census::add(7, 1);\n            return"))
p='src/interpreter/binding_map.rs'
a='    pub(crate) fn matches_names<T>(&self, names: &HashMap<String, T>) -> bool {'
edit.call(p,a,a+"\n        crate::capture_census::add(3, 1);\n        crate::capture_census::add(4, names.len());")
p='src/interpreter/mod.rs'
a='            let snapshot = self.active_capture_snapshot(index);'
edit.call(p,a,"            crate::capture_census::add(8, 1);\n"+a)
a='            Some(index) => self.active_capture_snapshot(index),'
edit.call(p,a,'            Some(index) => { crate::capture_census::add(9, 1); self.active_capture_snapshot(index) },')
puts JSON.pretty_generate({baseline:head.strip, labels:labels, edits:changes, limits:'Counts are reachability evidence, not time or allocation measurements. Prefix retry counts include pending-publication clone retries. Membership names are submitted names, not exact hash probes.'})
