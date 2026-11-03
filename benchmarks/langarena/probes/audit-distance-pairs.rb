require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena, binary, out = ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
commit = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators = %w[rust/src/benchmarks/distance.rs goose/src/distance.goose]
comparators.each do |path|
  bytes, status = Open3.capture2('git', '-C', arena, 'show', commit+':'+path)
  raise 'comparator drift' unless status.success? && bytes == File.read(File.join(arena,path))
end
paths = [binary, __FILE__, 'benchmarks/langarena/port.kujo'] + comparators.map { |p| File.join(arena,p) }
hashes = paths.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
strings = ['', 'a', 'b', 'ab', 'ba', 'abc', 'abcd', 'abdc', 'aaaa', 'aaaaaa',
           'abcdefghij', 'jihgfedcba', 'abababab', 'babababa', 'martha', 'marhta',
           "a\u0000bcd", 'éabc', 'éabc', '🐕abcd']
pairs = strings.product(strings)
# Independent multiset oracle. Preserve the benchmark's distinct-key denominator;
# repeated grams can legitimately produce a score above one.
ngram = lambda do |a,b|
  left = a.bytes.each_cons(4).to_a.group_by(&:itself).transform_values(&:size)
  right = b.bytes.each_cons(4).to_a.group_by(&:itself).transform_values(&:size)
  total = left.size + right.size
  total.zero? ? 0.0 : left.sum { |key,count| [count,right.fetch(key,0)].min }.to_f / total
end
Dir.mkdir(out)
File.write(out+'/pairs.json', JSON.pretty_generate(pairs))
rust_source = <<~RUST
  mod helper { pub fn next_int(_: i32) -> i32 { panic!("preparation is outside this kernel audit") } }
  fn config_i64(_: &str, _: &str) -> i64 { panic!("configuration is outside this kernel audit") }
  trait Benchmark { fn name(&self)->String; fn prepare(&mut self); fn run(&mut self,i:i64); fn checksum(&self)->u32; }
  mod benchmarks { pub mod distance {
  #{File.read(File.join(arena,comparators[0]))}
  pub fn audit(a:&str,b:&str)->(f64,f64) {
      let n = NGram { count:0,size:0,pairs:Vec::new(),result_val:0,n:4 };
      (Jaro::jaro(a,b),n.ngram(a,b))
  }
  } }
  fn main() {
      let pairs: Vec<(Vec<u8>,Vec<u8>)> = vec![#{pairs.map { |a,b| "(vec![#{a.bytes.join(',')}],vec![#{b.bytes.join(',')}])" }.join(',')}];
      for (i,(a,b)) in pairs.iter().enumerate() {
          let (j,n)=benchmarks::distance::audit(std::str::from_utf8(a).unwrap(),std::str::from_utf8(b).unwrap());
          println!("[{}, {:.17}, {:.17}, {}, {}]",i,j,n,(j*1000.0) as u32,(n*1000.0) as u32);
      }
  }
RUST
File.write(out+'/oracle.rs',rust_source)
argv = ['rustc','--edition=2021','-O',out+'/oracle.rs','-o',out+'/oracle']
stdout,stderr,status = Open3.capture3(*argv)
File.write(out+'/rust-build.log',stdout+stderr)
raise 'Rust build' unless status.success?
stdout,stderr,status = Open3.capture3(out+'/oracle')
File.write(out+'/rust.out',stdout); File.write(out+'/rust.err',stderr)
raise 'Rust execution' unless status.success?
reference = stdout.lines.map { |l| JSON.parse(l) }
raise 'Rust coverage' unless reference.size == pairs.size
reference.each_with_index do |row,i|
  raise 'Rust order/multiset oracle' unless row[0]==i && (row[2]-ngram.call(*pairs[i])).abs <= 1e-12
end
source = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
pair_literals = pairs.map { |pair| '['+pair.map { |s| "utf8_decode(bytes(#{JSON.generate(s.bytes)}))" }.join(',')+']' }
source += "\nlet pairs := [#{pair_literals.join(',')}]\nmut repeat := 0\nwhile repeat < 2 {\nmut i := 0\nfor pair in pairs {\nlet j := jaro(pair[0],pair[1])\nlet n := ngram(pair[0],pair[1])\nprint(to_json([repeat,i,j,n,to_int(j*1000),to_int(n*1000)]))\ni += 1\n}\nrepeat += 1\n}\n"
File.write(out+'/contracts.kujo',source)
%w[oracle.rs contracts.kujo pairs.json oracle].each { |name| hashes[out+'/'+name] = Digest::SHA256.file(out+'/'+name).hexdigest }
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,build_argv:argv,upstream_commit:commit,scope:'direct unchanged kernels; preparation/wrapper work and Goose execution excluded'}))
runs=[]
%w[vm interpreter jit].each do |mode|
  argv=[binary,'run','--scheduler-timeout-ms','120000']; argv << '--'+mode unless mode=='vm'; argv << out+'/contracts.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout); File.write(out+'/'+mode+'.err',stderr)
  runs << {runtime:mode,argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/runs.json',JSON.pretty_generate(runs))
  raise 'runtime '+mode unless status.success?
  rows=stdout.lines.map { |l| JSON.parse(l) }
  raise 'coverage '+mode unless rows.size==pairs.size*2
  rows.each_with_index do |row,index|
    expected=reference[index%pairs.size]
    raise 'order' unless row[0,2]==[index/pairs.size,index%pairs.size]
    raise 'score mismatch '+mode+' '+index.to_s unless (row[2]-expected[1]).abs <= 1e-12 && (row[3]-expected[2]).abs <= 1e-12 && row[4,2]==expected[3,2]
  end
end
hashes.each { |path,hash| raise 'identity drift' unless Digest::SHA256.file(path).hexdigest==hash }
summary={status:'verified',pairs:pairs.size,repetitions:2,runtimes:3,score_observations:pairs.size*2*3*2,rust_pair_observations:reference.size,
         scope:'pair scores and quantized checksum contributions only; no production, Goose execution, native participation or speed claim'}
File.write(out+'/verification.json',JSON.pretty_generate(summary)); puts JSON.generate(summary)
