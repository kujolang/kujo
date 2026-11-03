require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM GOOSE_COMPILER VERIFIED_STATE_EVIDENCE NEW_OUTPUT' unless ARGV.size==4
arena,compiler,evidence,out=ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
commit='3c9f84ddaeec834c0631747cb789e8cfd602f993'
Dir.mkdir(out)
%w[distance helper bench].each do |name|
  path='goose/src/'+name+'.goose'
  raw,status=Open3.capture2('git','-C',arena,'show',commit+':'+path)
  raise 'upstream drift' unless status.success? && raw==File.read(File.join(arena,path))
  File.write(out+'/'+name+'.goose',raw)
end
cases=JSON.parse(File.read(evidence+'/cases.json'))
reference=File.readlines(evidence+'/rust.out').map { |l| JSON.parse(l) }
raise 'case matrix' unless cases==[[10,5,42,1],[2,500,42,1],[0,1,42,3],[1,1,7,3],[16,32,7,3],[2,4,42,3]]
raise 'state count' unless reference.size==64
source=<<~GOOSE
  import std;
  import helper;
  import distance;
  fn record<B>(id:i64,kind:i64,phase:i64,state:B&) {
      var pairs: u8[>..] = [];
      pairs.append("[");
      var first = true;
      for p in state.pairs {
          if !first { pairs.append(","); }
          pairs.append(str("[\\\"",p.s1,"\\\",\\\"",p.s2,"\\\"]"));
          first = false;
      }
      pairs.append("]");
      print("[",id,",",kind,",",phase,",",rng_last,",",distance::checksum(state),",",pairs,"]");
  }
  fn main() {
GOOSE
cases.each_with_index do |(count,size,seed,iterations),id|
  %w[Jaro NGram].each_with_index do |type,kind|
    extra=kind==1 ? ',n:4' : ''
    source += <<~GOOSE
      {
          var state=distance::#{type} { count:#{count},size:#{size},result_val:0,pairs:[]#{extra} };
          rng_last=#{seed};
          distance::prepare(state);
          record(#{id},#{kind},0,state);
          distance::run(state,0);
          record(#{id},#{kind},1,state);
          reset();
          record(#{id},#{kind},2,state);
          for i in #{iterations} { distance::run(state,i); record(#{id},#{kind},3+i,state); }
      }
    GOOSE
  end
end
source += "}\n"
File.write(out+'/main.goose',source)
paths=[compiler,__FILE__,evidence+'/cases.json',evidence+'/rust.out']+%w[main distance helper bench].map { |name| out+'/'+name+'.goose' }
hashes=paths.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
commands=[]
[[compiler,'-O2','--standalone','-o',out+'/oracle.c',out+'/main.goose'],
 ['/usr/bin/clang','-O2','-w',out+'/oracle.c','-o',out+'/oracle','-lm','-pthread'],
 [out+'/oracle']].each_with_index do |argv,i|
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+"/step#{i}.out",stdout);File.write(out+"/step#{i}.err",stderr)
  commands << {argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/commands.json',JSON.pretty_generate(commands))
  File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,upstream_commit:commit}))
  raise 'command '+i.to_s unless status.success?
end
rows=File.readlines(out+'/step2.out').map { |l| JSON.parse(l) }
raise 'complete state' unless rows==reference
hashes.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
%w[oracle.c oracle].each { |name| hashes[out+'/'+name]=Digest::SHA256.file(out+'/'+name).hexdigest }
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,upstream_commit:commit}))
summary={status:'verified',complete_states:64,cases:6,algorithms:2,scope:'Executed Goose prepare/warmup/reset/run state versus Rust/Kujo; no production or speed claim'}
File.write(out+'/verification.json',JSON.pretty_generate(summary));puts JSON.generate(summary)
