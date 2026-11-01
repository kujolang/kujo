# Repository-maintenance diagnostic: exact DepthCache source, real runtime Values.
require 'json';require 'digest';require 'fileutils'
repo=Dir.pwd;output=File.expand_path(ARGV.fetch(0));library=File.realpath(ARGV.fetch(1));raise 'new directory required' if Dir.exist?(output);FileUtils.mkdir_p(output)
source=File.read('src/interpreter/assignment.rs')
extract=->(first,last){a=source.index(first);b=source.index(last,a||0);raise 'source anchors' unless a&&b;source[a...b]}
cache=extract.call("#[derive(Default)]\npub(crate) struct DepthCache", "fn child<'a>")
cache = cache[0...cache.index("    fn depth(")] + cache[cache.index("    fn remember_path(")..-1]
child=extract.call("fn child<'a>", 'pub(crate) fn update_checked')
lookup=extract.call('fn collection_index(access:', "fn collection_child_mut<'a>")
value_source=File.read('src/interpreter/value.rs')
a=value_source.index("    pub(crate) fn dict_index(");b=value_source.index("    /// Materialize",a);raise 'dict index anchors' unless a&&b
index_helper=value_source[a...b]
lookup=lookup.sub("Value::dict_index(","dict_index(")
probe=File.read('benchmarks/langarena/probes/depth_cache_controls.rs')
# Access variants unrelated to these methods are outside this component probe.
text="#![allow(dead_code)]\nuse kujo::interpreter::{DictMap, IntDictMap};\nmod diagnostic {\nuse kujo::interpreter::Value;\nuse std::sync::Arc;\nenum Access { Index(Value), Field(String) }\n"+cache+child+lookup+index_helper+probe+"\n}\nfn main(){diagnostic::run();}\n"
File.write(output+'/probe.rs',text)
sha=->(p){Digest::SHA256.file(p).hexdigest}
inputs=['src/interpreter/assignment.rs','src/interpreter/value.rs','benchmarks/langarena/probes/depth_cache_controls.rs',File.expand_path(__FILE__),library,'Cargo.lock'].to_h{|p|[File.expand_path(p),sha.call(p)]}
argv=['rustc','--edition=2021','-O','-C','lto=thin',output+'/probe.rs','--extern','kujo='+library,'-L','dependency='+repo+'/target/release/deps','-o',output+'/probe']
r={status:'building',pid:Process.pid,argv:argv,inputs:inputs,extracted_sha256:{cache:Digest::SHA256.hexdigest(cache),child:Digest::SHA256.hexdigest(child),lookup:Digest::SHA256.hexdigest(lookup)},limits:'Exact helper source with real runtime Values; empty-path cache mechanics only, not full assignment or language throughput.'}
save=->{File.write(output+'/build.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 raise 'build failed' unless system(*argv,out:output+'/build.log',err:[:child,:out])
 inputs.each{|p,h|raise 'input drift' unless sha.call(p)==h};r[:status]='built';r[:binary_sha256]=sha.call(output+'/probe');r[:generated_sha256]=sha.call(output+'/probe.rs');save.call
 raise 'probe failed' unless system(output+'/probe',out:output+'/observations.csv',err:output+'/run.stderr')
 r[:status]='measured-review-pending';r[:observations_sha256]=sha.call(output+'/observations.csv');save.call
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
