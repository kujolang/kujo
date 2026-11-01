require 'json';require 'digest'
o=File.expand_path(ARGV.fetch(0));sha=->(p){Digest::SHA256.file(p).hexdigest};r=JSON.parse(File.read(o+'/build.json'))
raise 'unfinished' unless r['status']=='measured-review-pending'
r['inputs'].each{|p,h|raise 'input drift '+p unless sha.call(p)==h}
raise 'binary' unless sha.call(o+'/probe')==r['binary_sha256']
raise 'generated source' unless sha.call(o+'/probe.rs')==r['generated_sha256']
raise 'observation digest' unless sha.call(o+'/observations.csv')==r['observations_sha256']
raise 'stderr' unless File.read(o+'/run.stderr').empty?
source=File.read('src/interpreter/assignment.rs');extract=->(a,b){i=source.index(a);j=source.index(b,i||0);raise 'anchors' unless i&&j;source[i...j]}
cache=extract.call("#[derive(Default)]\npub(crate) struct DepthCache", "fn child<'a>")
cache=cache[0...cache.index('    fn depth(')]+cache[cache.index('    fn remember_path(')..-1]
child=extract.call("fn child<'a>",'pub(crate) fn update_checked')
lookup=extract.call('fn collection_index(access:',"fn collection_child_mut<'a>").sub('Value::dict_index(','dict_index(')
v=File.read('src/interpreter/value.rs');a=v.index('    pub(crate) fn dict_index(');b=v.index('    /// Materialize',a);raise 'index anchors' unless a&&b
expected="#![allow(dead_code)]\nuse kujo::interpreter::{DictMap, IntDictMap};\nmod diagnostic {\nuse kujo::interpreter::Value;\nuse std::sync::Arc;\nenum Access { Index(Value), Field(String) }\n"+cache+child+lookup+v[a...b]+File.read('benchmarks/langarena/probes/depth_cache_controls.rs')+"\n}\nfn main(){diagnostic::run();}\n"
raise 'component extraction' unless File.read(o+'/probe.rs')==expected
lines=File.readlines(o+'/observations.csv',chomp:true);raise 'controls' unless lines.shift=='semantic_controls=passed'
rows=lines.map do |line|
 parts=line.split(',');raise 'columns' unless parts.size==7
 round,kind,live=parts.take(3).map{|x|Integer(x)};mode=parts[3];n,sum,ns=parts.drop(4).map{|x|Integer(x)}
 raise 'measurement' unless n==100_000&&sum==n&&ns>0
 {round:round,kind:kind,live:live,mode:mode,iterations:n,checksum:sum,elapsed_ns:ns}
end
expected_keys=(0..2).flat_map{|round|(0..3).flat_map{|kind|[1,64,512].flat_map{|live|%w[lookup release_remember].map{|mode|[round,kind,live,mode]}}}}
raise 'coverage' unless rows.map{|x|x.values_at(:round,:kind,:live,:mode)}==expected_keys
summary=rows.group_by{|x|x.values_at(:kind,:live,:mode)}.map do |key,group|
 values=group.map{|x|x[:elapsed_ns].fdiv(x[:iterations])}.sort
 {kind:key[0],live:key[1],mode:key[2],ns_per_operation:values,median_ns_per_operation:values[1]}
end
result={status:'passed',observations:rows.size,operations:rows.sum{|x|x[:iterations]},build_receipt_sha256:sha.call(o+'/build.json'),verifier_sha256:sha.call(File.expand_path(__FILE__)),semantic_scope:'Array host COW invalidation and retained snapshot; release removes weak reference; mixed-type cap clearing; repeated owner replacement; exact cache entry counts. Full depth traversal and assignment are excluded.',summary:summary,limits:r['limits']+' Local timings are diagnostic. No runtime candidate or end-to-end performance claim.'}
File.write(o+'/verified.json',JSON.pretty_generate(result)+"\n");puts "#{rows.size} exact component observations verified"
