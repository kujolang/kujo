# Full frequency/order/winner audit; comparator tie policies remain separate.
require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'existing output' if File.exist?(out)
comparators=%w[rust/src/benchmarks/words.rs goose/src/words.goose]
comparators.each do |p|
 raw,status=Open3.capture2('git','-C',arena,'show','3c9f84ddaeec834c0631747cb789e8cfd602f993:'+p)
 raise 'upstream drift' unless status.success? && raw==File.read(arena+'/'+p)
end
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/20_text_hash.kujo','benchmarks/langarena/src/00_common.kujo']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
inputs=['a','a a a','a b a','b a b','a b','b a','a b b a','z z a a b','long short long','é 猫 é','😀 x 😀','x'*1024+' y '+ 'x'*1024]
inputs += [1,8,64,257].map{|n|Array.new(n){|i|"word#{i%7}"}.join(' ')}
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel=File.read('benchmarks/langarena/src/20_text_hash.kujo')[/func words_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && source.include?(kernel)
diag=kernel.sub('func words_run(','func words_state_probe(')
needle="    return state\n}"
raise 'insertion point' unless diag.scan(needle).size==1
diag=diag.sub(needle,"    state[\"observed\"] = [frequencies,order,best,count]\n"+needle)
source+="\n"+diag+"\nlet word_inputs := #{JSON.generate(inputs)}\n"
source+=<<~KUJO
 mut index := 0
 for text in word_inputs {
     for seed in [0,4294967294] {
         mut original := {"text":text,"result":seed}
         mut observed := original
         let snapshot := original
         mut iteration := 0
         while iteration < 3 {
             original = words_run(original,iteration)
             observed = words_state_probe(observed,iteration)
             assert_equal(original["result"],observed["result"])
             assert_equal(original["text"],text)
             assert_equal(observed["text"],text)
             assert_equal(snapshot,{"text":text,"result":seed})
             print(to_json([index,seed,iteration,original["result"],observed["observed"]]))
             iteration += 1
         }
     }
     index += 1
 }
KUJO
expected=[]
inputs.each_with_index do |text,index|
 freq={};order=[]
 text.split(/ /,-1).each{|w|order<<w unless freq.key?(w);freq[w]=(freq[w]||0)+1}
 best=order.max_by{|w|freq[w]};count=freq[best]
 checksum=best.bytes.reduce(5381){|a,b|(a*33+b)&0xffffffff}
 [0,4294967294].each{|seed|3.times{|i|expected<<[index,seed,i,(seed+(i+1)*(count+checksum+freq.size))&0xffffffff,[freq,order,best,count]]}}
end
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source);File.write(out+'/inputs.json',JSON.pretty_generate(inputs))
%w[contracts.kujo inputs.json].each{|f|hashes[out+'/'+f]=Digest::SHA256.file(out+'/'+f).hexdigest}
rows=%w[vm interpreter jit].map do |mode|
 argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<'--'+mode unless mode=='vm';argv<<out+'/contracts.kujo'
 stdout,stderr,status=Open3.capture3(*argv)
 File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
 raise "runtime failure #{mode}: #{stderr}" unless status.success?
 raise "full frequency/winner mismatch #{mode}" unless stdout.lines.map{|l|JSON.parse(l)}==expected
 {runtime:mode,argv:argv,exit:status.exitstatus,observations:expected.size,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise 'drift' unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,rows:rows,scope:'288 full frequency/order/winner/checksum observations; 16 single-space inputs, tie controls, three runtimes; comparator tie order, whitespace, production and timing remain open.'}))
puts 'Verified288 Words complete-state observations.'
