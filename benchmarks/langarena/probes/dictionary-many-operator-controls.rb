# Implicit operator calls must not introduce speculative publication allocations.
require 'json'; require 'digest'; require 'fileutils'; require 'open3'
abort 'Expected BASE_PROBE CAND_PROBE CAND_BINARY NEW_OUTPUT_DIR' unless ARGV.size==4
base,candidate,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'Output exists' if File.exist?(out);FileUtils.mkdir_p(out)
r={status:'running',runner_sha256:Digest::SHA256.file(__FILE__).hexdigest,identities:[base,candidate,binary].to_h{|p|[p,Digest::SHA256.file(p).hexdigest]},rows:[],semantics:[],sources:{}}
save=->{File.write(out+'/report.json',JSON.pretty_generate(r)+"\n")}
begin
 %w[direct local].product([3,2048],[16,32]).each do |pattern,width,calls|
  dir=out+"/#{pattern}-#{width}-#{calls}";FileUtils.mkdir_p(dir)
  ending=pattern=='direct' ? 'return state[2] + n' : 'let value := state[2] return value + n'
  owner=<<~KUJO
   struct Trigger {
     value: int,
     func op_add(other) {
       let saved := state
       state[0] = other + 1
       assert_equal(saved[0], other)
       assert_equal(saved[1], other)
       return saved[0]
     }
   }
   mut state := {"0": 0, "1": 0, "2": Trigger { value: 0 }}
   mut k := 3
   while k < #{width} { state[k] = k k += 1 }
   export func snapshot() { return state }
   export func edit(n) { state[0] = n state[1] = n #{ending} }
  KUJO
  tail=width>3 ? "assert_equal(snapshot()[#{width-1}], #{width-1})" : ''
  source=<<~KUJO
   from owner import snapshot, edit
   let original := snapshot()
   mut n := 1
   while n <= #{calls} {
     assert_equal(edit(n), n)
     assert_equal(snapshot()[0], n + 1)
     assert_equal(snapshot()[1], n)
     n += 1
   }
   assert_equal(original[0], 0)
   assert_equal(original[1], 0)
   #{tail}
   print("OPERATOR ESCAPE PASS")
  KUJO
  File.write(dir+'/owner.kujo',owner);path=dir+'/main.kujo';File.write(path,source)
  [owner,source].each{|s|r[:sources][Digest::SHA256.hexdigest(s)]=s}
  meta={pattern:pattern,width:width,calls:calls,module_sha256:Digest::SHA256.hexdigest(owner),source_sha256:Digest::SHA256.hexdigest(source)}
  %w[vm jit].each do |mode|
   pair=[base,candidate].map do |probe|
    o,e,s=Open3.capture3(probe,mode,path);raise o+e unless s.success?&&o=="OPERATOR ESCAPE PASS\n"
    m=e.match(/requests=(\d+) requested_bytes=(\d+)/);raise e unless m
    {requests:m[1].to_i,bytes:m[2].to_i}
   end
   r[:rows]<<meta.merge(mode:mode,baseline:pair[0],candidate:pair[1]);save.call
  end
  %w[vm interpreter jit].each do |mode|
   args=[binary,'run',path,'--untrusted'];args<<'--'+mode unless mode=='vm'
   o,e,s=Open3.capture3(*args);raise o+e unless s.success?&&o=="OPERATOR ESCAPE PASS\n"
   r[:semantics]<<meta.merge(mode:mode,exit:0);save.call
  end
 end
 raise 'Coverage' unless r[:rows].size==16&&r[:semantics].size==24
 r[:regressions]=r[:rows].select{|x|%i[requests bytes].any?{|m|x[:candidate][m]>x[:baseline][m]}}
 raise 'Operator total regression' unless r[:regressions].empty?
 r[:deltas]=r[:rows].group_by{|x|x.values_at(:pattern,:width,:mode)}.map do |key,rows|
  a,b=rows.sort_by{|x|x[:calls]};delta={pattern:key[0],width:key[1],mode:key[2]}
  %i[baseline candidate].each{|label|delta[label]=%i[requests bytes].to_h{|m|[m,(b[label][m]-a[label][m])/16.0]}}
  raise 'Operator slope regression' unless %i[requests bytes].all?{|m|delta[:candidate][m]<=delta[:baseline][m]}
  delta
 end
 r[:status]='passed';save.call;puts 'Operator controls passed:16 allocation pairs /24 semantic runs'
rescue=>e;r[:status]='failed';r[:error]=e.message;save.call;raise
end
