# Paired component experiment; never changes the runtime or its accepted library.
require 'json';require 'digest';require 'fileutils'
repo=Dir.pwd;output=File.expand_path(ARGV.fetch(0));baseline=File.realpath(ARGV.fetch(1));library=File.realpath(ARGV.fetch(2));hasher=File.realpath(ARGV.fetch(3))
raise 'new directory required' if Dir.exist?(output);FileUtils.mkdir_p(output)
sha=->(p){Digest::SHA256.file(p).hexdigest};receipt=JSON.parse(File.read(baseline+'/build.json'));raise 'baseline not measured' unless receipt['status']=='measured-review-pending'
receipt['inputs'].each{|p,h|raise 'baseline drift' unless sha.call(p)==h};raise 'baseline source' unless sha.call(baseline+'/probe.rs')==receipt['generated_sha256']
s=File.read(baseline+'/probe.rs');start="mod diagnostic {\n";ending="\n}\nfn main(){diagnostic::run();}\n";raise 'module shape' unless s.include?(start)&&s.end_with?(ending)
core=s.split(start,2)[1][0...-ending.size]
raise 'run anchors' unless core.scan('pub fn run() {').size==1&&core.scan('for round in 0..3 {').size==1
core=core.sub('pub fn run() {','pub fn run(round: usize) {').sub('for round in 0..3 {','{')
cold=<<~RS
    println!("layout={},{}", std::mem::size_of::<DepthCache>(), std::mem::align_of::<DepthCache>());
    let cold_start = Instant::now();
    for _ in 0..10_000 { black_box(DepthCache::default()); }
    println!("cold={round},10000,{}", cold_start.elapsed().as_nanos());
RS
core=core.sub('    // Host mutation',cold+'    // Host mutation')
pattern=/std::collections::HashMap<(\s*)usize,/
raise 'four owner maps' unless core.scan(pattern).size==4
candidate=core.gsub(pattern){"IdentityMap<#{$1}usize,"}
alias_text="type IdentityMap<K,V> = std::collections::HashMap<K,V,std::hash::BuildHasherDefault<ahash::AHasher>>;\n"
main=<<~RS
fn main() {
    for round in 0..7 {
        if round % 2 == 0 {
            println!("variant=baseline"); baseline::run(round);
            println!("variant=candidate"); candidate::run(round);
        } else {
            println!("variant=candidate"); candidate::run(round);
            println!("variant=baseline"); baseline::run(round);
        }
    }
}
RS
text="#![allow(dead_code)]\nuse kujo::interpreter::{DictMap, IntDictMap};\nmod baseline {\n"+core+"\n}\nmod candidate {\n"+alias_text+candidate+"\n}\n"+main
File.write(output+'/probe.rs',text)
inputs=receipt['inputs'].merge([baseline+'/build.json',baseline+'/probe.rs',File.expand_path(__FILE__),library,hasher].to_h{|p|[p,sha.call(p)]})
argv=['rustc','--edition=2021','-O','-C','lto=thin',output+'/probe.rs','--extern','kujo='+library,'--extern','ahash='+hasher,'-L','dependency='+repo+'/target/release/deps','-o',output+'/probe']
r={status:'building',pid:Process.pid,argv:argv,inputs:inputs,baseline:baseline,core_sha256:Digest::SHA256.hexdigest(core),candidate_core_sha256:Digest::SHA256.hexdigest(candidate),generated_sha256:sha.call(output+'/probe.rs'),limits:'Seven alternating same-process component pairs. Only four internal-identity map hash builders differ. Empty-path cache maintenance, not full assignment or workload throughput; cold construction and layout reported separately.'}
save=->{File.write(output+'/build.json',JSON.pretty_generate(r)+"\n")};save.call
begin
 raise 'build failed' unless system(*argv,out:output+'/build.log',err:[:child,:out])
 inputs.each{|p,h|raise 'input drift' unless sha.call(p)==h};r[:status]='built';r[:binary_sha256]=sha.call(output+'/probe');save.call
 raise 'probe failed' unless system(output+'/probe',out:output+'/observations.csv',err:output+'/run.stderr')
 r[:status]='measured-review-pending';r[:observations_sha256]=sha.call(output+'/observations.csv');save.call
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
