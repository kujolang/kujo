# Complete output plus wrapper/RNG controls. No comparator execution or timing.
require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'existing output' if File.exist?(out)
revision='3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators=%w[rust/src/helper.rs rust/src/benchmarks/sort_benchmark.rs rust/src/benchmarks/sort_quick.rs rust/src/benchmarks/sort_merge.rs rust/src/benchmarks/sort_self.rs goose/src/sort.goose]
comparators.each do |path|
  pinned,status=Open3.capture2('git','-C',arena,'show',revision+':'+path)
  raise 'upstream drift' unless status.success? && pinned==File.read(arena+'/'+path)
end
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/30_sort.kujo','benchmarks/langarena/src/00_common.kujo']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
inputs=[1,2,3,31,64,257].flat_map do |n|
  ordered=(0...n).map{|i|i-n/2}
  random=[1,42,99].map do |seed|
    rng=seed
    Array.new(n){rng=(rng*3877+29573)%139968;rng%41-20}
  end
  [ordered,ordered.reverse,Array.new(n,7),Array.new(n){|i|i.even? ? -9 : 9},ordered.sort_by(&:abs)]+random
end
inputs<<[-2147483648,2147483647,0,-1,1,-2147483648,2147483647]
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel=File.read('benchmarks/langarena/src/30_sort.kujo')[/func sort_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && source.include?(kernel)
diagnostic=kernel.sub('func sort_run(', 'func sort_state_probe(')
needle="    return state\n}"
raise 'return extraction' unless diagnostic.scan(needle).size==1
diagnostic=diagnostic.sub(needle,"    state[\"observed_sorted\"] = sorted\n"+needle)
source+="\n"+diagnostic+"\nlet sort_inputs := #{JSON.generate(inputs)}\nlet sort_names := [\"Sort::Quick\",\"Sort::Merge\",\"Sort::Self\"]\n"
source+=<<~KUJO
  mut input_index := 0
  for input in sort_inputs {
      for name in sort_names {
          for seed in [0,4294967294] {
              reset_rng()
              mut original := {"cfg":{"name":name},"data":input,"result":seed}
              mut observed := original
              let snapshot := original
              mut iteration := 0
              while iteration < 2 {
                  let before_rng := rng
                  original = sort_run(original,iteration)
                  let after_rng := rng
                  rng = before_rng
                  observed = sort_state_probe(observed,iteration)
                  assert_equal(rng,after_rng)
                  assert_equal(original["result"],observed["result"])
                  assert_equal(original["data"],input)
                  assert_equal(observed["data"],input)
                  assert_equal(snapshot,{"cfg":{"name":name},"data":input,"result":seed})
                  print(to_json([input_index,name,seed,iteration,original["result"],rng,observed["observed_sorted"],original["data"]]))
                  iteration += 1
              }
          }
      }
      input_index += 1
  }
KUJO
raise 'input coverage/limit' unless inputs.size==49 && source.bytesize<=1048576
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/inputs.json',JSON.pretty_generate(inputs))
hashes[out+'/contracts.kujo']=Digest::SHA256.hexdigest(source)
hashes[out+'/inputs.json']=Digest::SHA256.file(out+'/inputs.json').hexdigest
expected=[]
inputs.each_with_index do |input,index|
  %w[Sort::Quick Sort::Merge Sort::Self].each do |name|
    [0,4294967294].each do |seed|
      rng=42;result=seed;sorted=input.sort
      2.times do |iteration|
        rng=(rng*3877+29573)%139968
        result=(result+input[(rng.to_f/139968*input.size).to_i])&0xffffffff
        rng=(rng*3877+29573)%139968
        result=(result+sorted[(rng.to_f/139968*input.size).to_i])&0xffffffff
        expected<<[index,name,seed,iteration,result,rng,sorted,input]
      end
    end
  end
end
rows=[]
%w[vm interpreter jit].each do |mode|
  argv=[binary,'run','--scheduler-timeout-ms','120000']
  argv<<'--'+mode unless mode=='vm'
  argv<<out+'/contracts.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout)
  File.write(out+'/'+mode+'.err',stderr)
  raise "runtime failure #{mode}: #{stderr}" unless status.success?
  actual=stdout.lines.map{|line|JSON.parse(line)}
  raise "complete output/RNG/checksum mismatch #{mode}" unless actual==expected
  rows<<{runtime:mode,argv:argv,exit:status.exitstatus,observations:actual.size,
         stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,rows:rows,
  scope:'1764 complete sorted-output/input/RNG/checksum observations across three runtimes; 49 nonempty crafted integer inputs, no comparator execution, production, stable-object-order or speed claim.'}))
puts 'Verified1764 complete sort-wrapper observations; all three structural rows remain partial.'
