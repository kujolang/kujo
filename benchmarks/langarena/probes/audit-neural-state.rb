# Execute pinned Rust training code and compare complete Kujo network snapshots.
require 'json'; require 'digest'; require 'fileutils'; require 'open3'
raise 'usage: FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT' unless ARGV.size == 3
binary, upstream, out = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
pin = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths = %w[rust/src/benchmarks/neural_net.rs rust/src/helper.rs goose/src/neural_net.goose]
originals = paths.map do |path|
 bytes, err, status = Open3.capture3('git', '-C', upstream, 'show', "#{pin}:#{path}")
 raise err unless status.success? && bytes == File.binread(upstream+'/'+path)
 File.write(out+'/'+File.basename(path),bytes); bytes
end
core = originals[0].split('pub struct NeuralNet {',2).first.sub('use super::super::{helper, Benchmark};','')
helper = originals[1].split('pub fn checksum_str',2).first
rust = "mod helper {\n#{helper}\n}\n#{core}\n" + <<'RS'
fn dump(seed: i32, phase: &str, step: usize, n: &NeuralNetwork) {
 print!("{{\"seed\":{},\"phase\":\"{}\",\"step\":{},\"neurons\":[",seed,phase,step);
 for (i, v) in n.input_layer.iter().chain(n.hidden_layer.iter()).chain(n.output_layer.iter()).enumerate() {
  if i>0 {print!(",");}
  print!("[{:.17e},{:.17e},{:.17e},{:.17e},{:?},{:?}]",v.threshold,v.prev_threshold,v.output,v.error,v.synapses_in,v.synapses_out);
 }
 print!("],\"synapses\":[");
 for (i,s) in n.synapses.iter().enumerate() {
  if i>0 {print!(",");}
  let source=s.source_neuron+if i<20 {0} else {2};
  let dest=s.dest_neuron+if i<20 {2} else {12};
  print!("[{},{},{:.17e},{:.17e}]",source,dest,s.weight,s.prev_weight);
 }
 println!("]}}");
}
fn main() {
 for seed in [42,1,139967] {
  helper::set_last(seed); let mut n=NeuralNetwork::new(2,10,1); dump(seed,"prepare",0,&n);
  for i in 0..128 {
   let (a,b,t)=match i%4 {0=>(0.0,0.0,0.0),1=>(1.0,0.0,1.0),2=>(0.0,1.0,1.0),_=>(1.0,1.0,0.0)};
   n.train(&[a,b],&[t]); dump(seed,"train",i,&n);
  }
  for (i,(a,b)) in [(0.0,0.0),(0.0,1.0),(1.0,0.0),(1.0,1.0)].iter().enumerate() {
   n.feed_forward(&[*a,*b]);dump(seed,"forward",i,&n);
  }
 }
}
RS
File.write(out+'/oracle.rs',rust)
stdout,stderr,status=Open3.capture3('rustc','--edition=2021','-O',out+'/oracle.rs','-o',out+'/oracle');File.write(out+'/build.log',stdout+stderr);raise stderr unless status.success?
stdout,stderr,status=Open3.capture3(out+'/oracle');File.write(out+'/rust.out',stdout);File.write(out+'/rust.err',stderr);raise stderr unless status.success?
expected=stdout.lines.map{|l|JSON.parse(l)};raise 'oracle coverage' unless expected.size==399
sources=%w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/49_neural.kujo]
fixture=sources.map{|p|File.read(p)}.join("\n")+"\n"+<<'KUJO'
func audit_flat(state) {
 mut neurons := []
 for n in state["neurons"] { neurons = push(neurons, [n.threshold,n.previous,n.output,n.error,n.incoming,n.outgoing]) }
 mut synapses := []
 for s in state["synapses"] { synapses = push(synapses, [s.source,s.destination,s.weight,s.previous]) }
 return {"neurons":neurons,"synapses":synapses}
}
func audit_dump(seed, phase, step, state) {
 let flat := audit_flat(state)
 print(to_json({"seed":seed,"phase":phase,"step":step,"neurons":flat["neurons"],"synapses":flat["synapses"]}))
}
for seed in [42,1,139967] {
 rng = seed
 mut state := neural_prepare(basic_state({"name":"Etc::NeuralNet"}))
 audit_dump(seed,"prepare",0,state)
 let inputs := [[0,0,0],[1,0,1],[0,1,1],[1,1,0]]
 mut i := 0
 while i < 128 {
  let item := inputs[i % 4]
  let previous := state
  let snapshot := to_json(audit_flat(previous))
  state = neural_train(state,item[0],item[1],item[2])
  assert_equal(to_json(audit_flat(previous)),snapshot)
  audit_dump(seed,"train",i,state)
  i += 1
 }
 i = 0
 for pair in [[0,0],[0,1],[1,0],[1,1]] {
  let previous := state
  let snapshot := to_json(audit_flat(previous))
  state = neural_forward(state,pair[0],pair[1])
  assert_equal(to_json(audit_flat(previous)),snapshot)
  audit_dump(seed,"forward",i,state)
  i += 1
 }
}
KUJO
File.write(out+'/fixture.kujo',fixture)
files=([binary,File.expand_path(__FILE__)]+sources.map{|p|File.expand_path(p)}+paths.map{|p|upstream+'/'+p}+%w[oracle.rs oracle fixture.kujo].map{|p|out+'/'+p}).to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
# Floating point arithmetic/exp may differ across engines; links/indices remain exact.
max_error=0.0; float_checks=0
compare=nil
compare=->(a,b,path) do
 if a.is_a?(Float) && b.is_a?(Numeric)
  delta=(a-b).abs;raise "float mismatch #{path}: #{a} #{b}" unless a.finite? && b.finite? && delta<=1e-12*[1.0,a.abs,b.abs].max
  max_error=[max_error,delta].max;float_checks+=1
 elsif a.is_a?(Array)
  raise "array shape #{path}" unless b.is_a?(Array)&&a.size==b.size;a.zip(b).each_with_index{|(x,y),i|compare.call(x,y,path+[i])}
 elsif a.is_a?(Hash)
  raise "object shape #{path}" unless b.is_a?(Hash)&&a.keys.sort==b.keys.sort;a.each{|k,v|compare.call(v,b[k],path+[k])}
 else
  raise "exact mismatch #{path}" unless a==b
 end
end
rows=[]
%w[vm interpreter jit].each do |mode|
 argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
 stdout,stderr,status=Open3.capture3(*argv);File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr);raise "#{mode}: #{stderr}" unless status.success?
 actual=stdout.lines.map{|l|JSON.parse(l)};compare.call(expected,actual,[])
 rows<<{runtime:mode,argv:argv,exit:status.exitstatus,states:actual.size}
end
files.each{|p,h|raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h}
report={status:'verified',pin:pin,files:files,rows:rows,states:1197,float_checks:float_checks,max_absolute_error:max_error,tolerance:'1e-12 * max(1,abs(expected),abs(actual))',scope:'Pinned Rust train/feed-forward/new methods executed; full normalized state and Kujo snapshot checks. Goose source reviewed only. 128 training steps per seed, not full production, work counts or speed.'}
File.write(out+'/verification.json',JSON.pretty_generate(report));puts JSON.pretty_generate(report.reject{|k,_|k==:files})
