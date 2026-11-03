# Full matrix output and task-partition controls; exact dyadic inputs.
require 'json';require 'digest';require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)};raise 'existing output' if File.exist?(out)
comparators=%w[rust/src/benchmarks/matmul.rs goose/src/matmul.goose]
comparators.each{|p|raw,status=Open3.capture2('git','-C',arena,'show','3c9f84ddaeec834c0631747cb789e8cfd602f993:'+p);raise 'upstream drift' unless status.success? && raw==File.read(arena+'/'+p)}
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/25_matmul.kujo','benchmarks/langarena/src/00_common.kujo']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
inputs=[1,2,3,7,16,17].flat_map do |n|
 zero=Array.new(n){Array.new(n,0.0)}
 ident=Array.new(n){|i|Array.new(n){|j|i==j ? 1.0 : 0.0}}
 a=Array.new(n){|i|Array.new(n){|j|((i*3-j*5)%17-8)/8.0}}
 b=Array.new(n){|i|Array.new(n){|j|((i*7+j*2)%13-6)/4.0}}
 [[zero,b],[ident,b],[a,ident],[a,b]]
end
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel=File.read('benchmarks/langarena/src/25_matmul.kujo')[/func mat_run\(.*?\n}\n/m]
raise 'kernel' unless kernel && source.include?(kernel)
diag=kernel.sub('func mat_run(','func mat_state_probe(')
needle="    return state\n}";raise 'return' unless diag.scan(needle).size==1
diag=diag.sub(needle,"    state[\"observed\"] = result\n"+needle)
source+="\n"+diag+"\nlet matrix_inputs := #{JSON.generate(inputs)}\nlet matrix_start_index := 0\n"
source+=<<~KUJO
 mut index := matrix_start_index
 for matrices in matrix_inputs {
     let a := matrices[0]
     let b := matrices[1]
     for name in ["Matmul::Single","Matmul::T4","Matmul::T8","Matmul::T16"] {
         for seed in [0,4294967294] {
             mut original := {"cfg":{"n":len(a),"name":name},"a":a,"b":b,"result":seed}
             mut observed := original
             let snapshot := original
             mut iteration := 0
             while iteration < 2 {
                 original = mat_run(original,iteration)
                 observed = mat_state_probe(observed,iteration)
                 assert_equal(original["result"],observed["result"])
                 assert_equal(original["a"],a)
                 assert_equal(original["b"],b)
                 assert_equal(observed["a"],a)
                 assert_equal(observed["b"],b)
                 assert_equal(snapshot,{"cfg":{"n":len(a),"name":name},"a":a,"b":b,"result":seed})
                 print(to_json([index,name,seed,iteration,original["result"],observed["observed"]]))
                 iteration += 1
             }
         }
     }
     index += 1
 }
KUJO
expected=[]
inputs.each_with_index do |(a,b),index|
 n=a.size
 product=Array.new(n){|i|Array.new(n){|j|(0...n).sum{|k|a[i][k]*b[k][j]}}}
 text=format('%.7f',product[n/2][n/2])
 checksum=text.bytes.reduce(5381){|v,byte|(v*33+byte)&0xffffffff}
 %w[Matmul::Single Matmul::T4 Matmul::T8 Matmul::T16].each{|name|[0,4294967294].each{|seed|2.times{|i|expected<<[index,name,seed,i,(seed+(i+1)*checksum)&0xffffffff,product]}}}
end
raise 'size' unless source.bytesize<=1048576 && inputs.size==24
Dir.mkdir(out);File.write(out+'/contracts.kujo',source);File.write(out+'/inputs.json',JSON.pretty_generate(inputs))
%w[contracts.kujo inputs.json].each{|f|hashes[out+'/'+f]=Digest::SHA256.file(out+'/'+f).hexdigest}
rows=%w[vm interpreter jit].map do |mode|
 argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<'--'+mode unless mode=='vm';argv<<out+'/contracts.kujo'
 stdout='';stderr='';invocations=[]
 inputs.each_with_index do |input,case_index|
   fixture=source.sub('let matrix_inputs := '+JSON.generate(inputs),'let matrix_inputs := '+JSON.generate([input])).sub('let matrix_start_index := 0','let matrix_start_index := '+case_index.to_s)
   path=out+'/case-'+case_index.to_s+'.kujo'
   if File.exist?(path)
     raise 'fixture drift' unless File.read(path)==fixture
   else
     File.write(path,fixture)
   end
   hashes[path]=Digest::SHA256.hexdigest(fixture)
   case_argv=argv[0...-1]+[path]
   text,error,status=Open3.capture3(*case_argv)
   stem=out+'/'+mode+'-'+case_index.to_s
   File.write(stem+'.out',text);File.write(stem+'.err',error)
   invocations<<{argv:case_argv,exit:status.exitstatus,stdout_sha256:Digest::SHA256.hexdigest(text),stderr_sha256:Digest::SHA256.hexdigest(error)}
   raise "runtime failure #{mode}/#{case_index}: #{error}" unless status.success?
   stdout+=text;stderr+=error
 end
 File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
 raise "matrix/checksum mismatch #{mode}" unless stdout.lines.map{|l|JSON.parse(l)}==expected
 {runtime:mode,invocations:invocations,exit:0,observations:expected.size,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise 'source drift' unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,rows:rows,scope:'1152 complete matrix/checksum observations;24 exact-dyadic matrix pairs,4 execution variants,2 seeds,2 iterations,3 runtimes. No preparation, comparator execution, production, timing or native-participation claim.'}))
puts 'Verified1152 complete matrix observations.'
