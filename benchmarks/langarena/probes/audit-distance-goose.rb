require 'json'
require 'digest'
require 'open3'
require 'fileutils'
raise 'usage: UPSTREAM GOOSE_COMPILER VERIFIED_PAIR_EVIDENCE NEW_OUTPUT' unless ARGV.size==4
arena,compiler,pair_evidence,out=ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
commit='3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths=%w[goose/src/distance.goose goose/src/helper.goose goose/src/bench.goose]
Dir.mkdir(out)
paths.each do |path|
  raw,status=Open3.capture2('git','-C',arena,'show',commit+':'+path)
  raise 'upstream drift' unless status.success? && raw==File.read(File.join(arena,path))
  File.write(File.join(out,File.basename(path)),raw)
end
pairs=JSON.parse(File.read(File.join(pair_evidence,'pairs.json')))
reference=File.readlines(File.join(pair_evidence,'rust.out')).map { |l| JSON.parse(l) }
raise 'pair count' unless pairs.size==400 && reference.size==400
source=<<~GOOSE
  import std;
  import helper;
  import distance;
  fn main() {
      let pairs: distance::Pair[>..] = [#{pairs.map { |a,b| "distance::Pair { s1: [#{a.bytes.join(',')}], s2: [#{b.bytes.join(',')}] }" }.join(',')}];
      var state = distance::NGram { count:0,size:0,result_val:0,n:4,pairs:[] };
      var i = 0;
      for p in pairs {
          let j = distance::jaro(p.s1,p.s2);
          let n = distance::ngram(state,p.s1,p.s2);
          var jtext: u8[>..] = [];
          var ntext: u8[>..] = [];
          format_fixed(jtext,j,15);
          format_fixed(ntext,n,15);
          print("[",i,",",jtext,",",ntext,",",(j*1000.0) as u32,",",(n*1000.0) as u32,"]");
          i++;
      }
  }
GOOSE
File.write(out+'/main.goose',source)
files=[compiler,__FILE__,pair_evidence+'/pairs.json',pair_evidence+'/rust.out',out+'/main.goose']+paths.map { |p| File.join(out,File.basename(p)) }
hashes=files.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
commands=[]
[[compiler,'-O2','--standalone','-o',out+'/oracle.c',out+'/main.goose'],
 ['/usr/bin/clang','-O2','-w',out+'/oracle.c','-o',out+'/oracle','-lm','-pthread'],
 [out+'/oracle']].each_with_index do |argv,i|
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+"/step#{i}.out",stdout);File.write(out+"/step#{i}.err",stderr)
  commands << {argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/commands.json',JSON.pretty_generate(commands))
  File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,upstream_commit:commit}))
  raise 'command failed '+i.to_s unless status.success?
end
rows=File.readlines(out+'/step2.out').map { |l| JSON.parse(l) }
raise 'coverage' unless rows.size==400
rows.each_with_index do |row,i|
  expected=reference[i]
  raise 'score '+i.to_s unless row[0]==i && (row[1]-expected[1]).abs<=1e-12 && (row[2]-expected[2]).abs<=1e-12 && row[3,2]==expected[3,2]
end
hashes.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
%w[oracle.c oracle].each { |name| hashes[out+'/'+name]=Digest::SHA256.file(out+'/'+name).hexdigest }
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,upstream_commit:commit}))
summary={status:'verified',pairs:400,score_observations:800,scope:'Executed unchanged Goose kernels versus recorded Rust pair scores; no preparation, production or speed claim'}
File.write(out+'/verification.json',JSON.pretty_generate(summary));puts JSON.generate(summary)
