require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena,binary,out=ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
commit='3c9f84ddaeec834c0631747cb789e8cfd602f993'
upstream=%w[rust/src/benchmarks/distance.rs rust/src/helper.rs rust/src/main.rs goose/src/distance.goose]
upstream.each do |path|
  bytes,status=Open3.capture2('git','-C',arena,'show',commit+':'+path)
  raise 'source drift' unless status.success? && bytes==File.read(File.join(arena,path))
end
paths=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/test.json']+upstream.map { |p| File.join(arena,p) }
hashes=paths.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
cases=[[10,5,42,1],[2,500,42,1],[0,1,42,3],[1,1,7,3],[16,32,7,3],[2,4,42,3]]
Dir.mkdir(out)
File.write(out+'/cases.json',JSON.pretty_generate(cases))
rust=<<~RUST
  mod helper { #{File.read(File.join(arena,'rust/src/helper.rs'))} }
  fn config_i64(_: &str,_: &str)->i64 { panic!("audit constructs configured state directly") }
  trait Benchmark { fn name(&self)->String; fn prepare(&mut self); fn run(&mut self,i:i64); fn checksum(&self)->u32; }
  mod benchmarks { pub mod distance {
  #{File.read(File.join(arena,'rust/src/benchmarks/distance.rs'))}
  fn record(id:usize,kind:usize,phase:usize,result:u32,pairs:&[(String,String)]) {
      let text=pairs.iter().map(|(a,b)|format!("[{:?},{:?}]",a,b)).collect::<Vec<_>>().join(",");
      println!("[{},{},{},{},{},[{}]]",id,kind,phase,crate::helper::last(),result,text);
  }
  pub fn audit(id:usize,count:usize,size:usize,seed:i32,iterations:usize) {
      macro_rules! check { ($state:expr,$kind:expr) => {{
          let mut state=$state;
          crate::helper::set_last(seed); state.prepare();
          record(id,$kind,0,state.checksum(),&state.pairs);
          state.run(0); record(id,$kind,1,state.checksum(),&state.pairs);
          crate::helper::reset(); record(id,$kind,2,state.checksum(),&state.pairs);
          for i in 0..iterations { state.run(i as i64); record(id,$kind,3+i,state.checksum(),&state.pairs); }
      }}; }
      check!(Jaro { count,size,pairs:Vec::new(),result_val:0 },0);
      check!(NGram { count,size,pairs:Vec::new(),result_val:0,n:4 },1);
  }
  } }
  fn main() { #{cases.each_with_index.map { |c,i| "benchmarks::distance::audit(#{i},#{c.join(',')});" }.join} }
RUST
File.write(out+'/oracle.rs',rust)
argv=['rustc','--edition=2021','-O',out+'/oracle.rs','-o',out+'/oracle']
stdout,stderr,status=Open3.capture3(*argv)
File.write(out+'/rust-build.log',stdout+stderr);raise 'build' unless status.success?
stdout,stderr,status=Open3.capture3(out+'/oracle')
File.write(out+'/rust.out',stdout);File.write(out+'/rust.err',stderr);raise 'oracle' unless status.success?
reference=stdout.lines.map { |l| JSON.parse(l) }
expected_count=cases.sum { |c| (3+c[3])*2 }
raise 'oracle coverage' unless reference.size==expected_count
# Independently reconstruct every preparation byte and resulting RNG state.
cases.each_with_index do |(count,size,seed,iterations),id|
  rng=seed
  next_int=lambda { |maximum| rng=(rng*3877+29573)%139968; (rng.to_f/139968*maximum).to_i }
  pairs=Array.new(count) do
    lengths=[next_int.call(size)+4,next_int.call(size)+4]
    lengths.map { |length| Array.new(length) { 'abcdefghij'[next_int.call(10)] }.join }
  end
  2.times do |kind|
    rows=reference.select { |r| r[0,2]==[id,kind] }
    raise 'state order' unless rows.map { |r| r[2] }==(0...(3+iterations)).to_a
    raise 'preparation state' unless rows.all? { |r| r[5]==pairs } && rows[0][4]==0
    raise 'rng state' unless rows.all? { |r| r[3]==(r[2]<2 ? rng : 42) }
  end
end
configs=JSON.parse(File.read('benchmarks/langarena/test.json'))
%w[Distance::Jaro Distance::NGram].each_with_index do |name,kind|
  cfg=configs.find { |c| c['name']==name }
  id=kind
  raise 'official input drift' unless cases[id][0,2]==cfg.values_at('count','size') && cases[id][3]==cfg['iterations']
  row=reference.find { |r| r[0,3]==[id,kind,3] }
  raise 'official checksum' unless row[4]==cfg['checksum']
end
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
source += <<~KUJO
  func record_distance(id,kind,phase,state) { print(to_json([id,kind,phase,rng,state["result"],state["pairs"]])) }
  let cases := #{JSON.generate(cases)}
  mut id := 0
  for cfg in cases {
      mut kind := 0
      while kind < 2 {
          mut name := "Distance::Jaro"
          if kind == 1 { name = "Distance::NGram" }
          rng = cfg[2]
          mut state := distance_prepare(basic_state({"name":name,"count":cfg[0],"size":cfg[1]}))
          let snapshot := state
          record_distance(id,kind,0,state)
          state = distance_run(state,0)
          record_distance(id,kind,1,state)
          reset_rng()
          record_distance(id,kind,2,state)
          mut i := 0
          while i < cfg[3] {
              state = distance_run(state,i)
              record_distance(id,kind,i+3,state)
              assert_equal(snapshot["result"],0)
              assert_equal(snapshot["pairs"],state["pairs"])
              i += 1
          }
          kind += 1
      }
      id += 1
  }
KUJO
File.write(out+'/contracts.kujo',source)
%w[oracle.rs oracle contracts.kujo cases.json].each { |name| hashes[out+'/'+name]=Digest::SHA256.file(out+'/'+name).hexdigest }
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,build_argv:argv,upstream_commit:commit}))
runs=[]
%w[vm interpreter jit].each do |mode|
  argv=[binary,'run','--scheduler-timeout-ms','120000'];argv << '--'+mode unless mode=='vm';argv << out+'/contracts.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
  runs << {runtime:mode,argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/runs.json',JSON.pretty_generate(runs));raise 'runtime '+mode unless status.success?
  raise 'full state '+mode unless stdout.lines.map { |l| JSON.parse(l) }==reference
end
hashes.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
summary={status:'verified',cases:cases.size,algorithms:2,runtimes:3,complete_states:expected_count*3,
         scope:'Preparation and explicit warmup/reset/run sequence, full pair lists and RNG; no native participation, production or Goose execution claim'}
File.write(out+'/verification.json',JSON.pretty_generate(summary));puts JSON.generate(summary)
