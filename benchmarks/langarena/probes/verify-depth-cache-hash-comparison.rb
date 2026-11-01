require 'json';require 'digest'
o=File.expand_path(ARGV.fetch(0));sha=->(p){Digest::SHA256.file(p).hexdigest};r=JSON.parse(File.read(o+'/build.json'));raise 'unfinished' unless r['status']=='measured-review-pending'
r['inputs'].each{|p,h|raise 'input drift '+p unless sha.call(p)==h}
raise 'baseline verification' unless system('ruby','benchmarks/langarena/probes/verify-depth-cache-controls.rb',r['baseline'])
raise 'binary' unless sha.call(o+'/probe')==r['binary_sha256'];raise 'generated' unless sha.call(o+'/probe.rs')==r['generated_sha256'];raise 'output' unless sha.call(o+'/observations.csv')==r['observations_sha256'];raise 'stderr' unless File.read(o+'/run.stderr').empty?
text=File.read(o+'/probe.rs');prefix="#![allow(dead_code)]\nuse kujo::interpreter::{DictMap, IntDictMap};\nmod baseline {\n";boundary="\n}\nmod candidate {\n";raise 'modules' unless text.start_with?(prefix)&&text.scan(boundary).size==1
base,other=text[prefix.size..-1].split(boundary,2);alias_text="type IdentityMap<K,V> = std::collections::HashMap<K,V,std::hash::BuildHasherDefault<ahash::AHasher>>;\n";raise 'hasher alias' unless other.start_with?(alias_text)
candidate,main=other[alias_text.size..-1].split("\n}\nfn main() {\n",2);raise 'main' unless main
pattern=/std::collections::HashMap<(\s*)usize,/
raise 'four owner maps only' unless base.scan(pattern).size==4&&base.gsub(pattern){"IdentityMap<#{$1}usize,"}==candidate
raise 'core hashes' unless Digest::SHA256.hexdigest(base)==r['core_sha256']&&Digest::SHA256.hexdigest(candidate)==r['candidate_core_sha256']
original=File.read(r['baseline']+'/probe.rs').split("mod diagnostic {\n",2)[1];ending="\n}\nfn main(){diagnostic::run();}\n";raise 'original end' unless original.end_with?(ending);original=original[0...-ending.size]
cold=<<~RS
    println!("layout={},{}", std::mem::size_of::<DepthCache>(), std::mem::align_of::<DepthCache>());
    let cold_start = Instant::now();
    for _ in 0..10_000 { black_box(DepthCache::default()); }
    println!("cold={round},10000,{}", cold_start.elapsed().as_nanos());
RS
expected=original.sub('pub fn run() {','pub fn run(round: usize) {').sub('for round in 0..3 {','{').sub('    // Host mutation',cold+'    // Host mutation');raise 'baseline adaptation' unless base==expected
lines=File.readlines(o+'/observations.csv',chomp:true);rows=[];cold_rows=[];layouts={}
(0..6).each do |round|
 order=round.even? ? %w[baseline candidate] : %w[candidate baseline]
 order.each do |variant|
  raise 'alternating variant order' unless lines.shift=="variant=#{variant}"
  layout=lines.shift.match(/\Alayout=(\d+),(\d+)\z/);raise 'layout' unless layout
  sizes=layout.captures.map(&:to_i);raise 'layout drift' if layouts[variant]&&layouts[variant]!=sizes;layouts[variant]=sizes
  cold_line=lines.shift.match(/\Acold=(\d+),10000,(\d+)\z/);raise 'cold count' unless cold_line&&cold_line[1].to_i==round&&cold_line[2].to_i>0
  cold_rows<<{round:round,variant:variant,iterations:10000,elapsed_ns:cold_line[2].to_i}
  raise 'semantic controls' unless lines.shift=='semantic_controls=passed'
  (0..3).each do |kind|
   [1,64,512].each do |live|
    %w[lookup release_remember].each do |mode|
     parts=lines.shift.to_s.split(',');raise 'row scope' unless parts.size==7&&parts.take(4)==[round.to_s,kind.to_s,live.to_s,mode]
     n,sum,ns=parts.drop(4).map{|x|Integer(x)};raise 'measurement' unless n==100000&&sum==n&&ns>0
     rows<<{round:round,variant:variant,kind:kind,live:live,mode:mode,iterations:n,elapsed_ns:ns}
    end
   end
  end
 end
end
raise 'trailing output' unless lines.empty?
summary=rows.group_by{|x|x.values_at(:kind,:live,:mode)}.map do |key,group|
 ratios=group.group_by{|x|x[:round]}.map{|_,pair|a=pair.find{|x|x[:variant]=='baseline'};b=pair.find{|x|x[:variant]=='candidate'};a[:elapsed_ns].fdiv(b[:elapsed_ns])}
 {kind:key[0],live:key[1],mode:key[2],paired_baseline_over_candidate:ratios,median_ratio:ratios.sort[3]}
end
cold_ratios=cold_rows.group_by{|x|x[:round]}.map{|_,pair|a=pair.find{|x|x[:variant]=='baseline'};b=pair.find{|x|x[:variant]=='candidate'};a[:elapsed_ns].fdiv(b[:elapsed_ns])}
File.write(o+'/verified.json',JSON.pretty_generate({status:'passed',observations:rows.size,cold_observations:cold_rows.size,semantic_control_passes:14,layouts:layouts,summary:summary,cold_paired_ratios:cold_ratios,cold_median_ratio:cold_ratios.sort[3],verifier_sha256:sha.call(File.expand_path(__FILE__)),build_sha256:sha.call(o+'/build.json'),limits:r['limits']+' This is diagnostic component evidence, not runtime acceptance.'})+"\n");puts "#{rows.size} paired component observations verified"
