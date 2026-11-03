# Fixed-work uninstrumented diagnostic; not an official production score.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO VERIFIED_STATE_DIRECTORY NEW_OUTPUT' unless ARGV.size==3
binary,state,out=ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
manifest=JSON.parse(File.read(state+'/manifest.json'));manifest['files'].each { |p,h| raise 'state input drift' unless Digest::SHA256.file(p).hexdigest==h }
raise 'state acceptance' unless JSON.parse(File.read(state+'/verification.json'))['complete_states']==120
config=JSON.parse(File.read('benchmarks/langarena/run.json')).find { |c| c['name']=='Brainfuck::Array' }
program=config.fetch('warmup_program')
# Independent execution for the expected returned checksum.
ops=program.chars;stack=[];jumps={}
ops.each_with_index { |c,i| stack<<i if c=='[';if c==']';start=stack.pop;jumps[start]=i;jumps[i]=start;end }
tape=Array.new(30000,0);pos=pc=value=0
while pc<ops.size
  case ops[pc]
  when '+' then tape[pos]=(tape[pos]+1)&255
  when '-' then tape[pos]=(tape[pos]-1)&255
  when '>' then pos+=1;tape<<0 if pos==tape.size
  when '<' then pos=[0,pos-1].max
  when '[' then pc=jumps[pc] if tape[pos]==0
  when ']' then pc=jumps[pc] if tape[pos]!=0
  when '.' then value=(value*4+tape[pos])&0xffffffff
  end
  pc+=1
end
FileUtils.mkdir_p(out)
base=File.read('benchmarks/langarena/src/00_common.kujo')+"\n"+File.read('benchmarks/langarena/src/20_text_hash.kujo')+"\n"+File.read(state+'/byte-variant.kujo')
%w[string byte].each do |variant|
  function=variant=='string' ? 'bf_array' : 'bf_byte_array'
  File.write(out+'/'+variant+'.kujo',base+"\nmut result := 0\nmut iteration := 0\nwhile iteration < 20 { result = u32(result + #{function}(#{JSON.generate(program)})) iteration += 1 }\nprint(result)\n")
end
files=[binary,File.expand_path(__FILE__),'benchmarks/langarena/run.json',out+'/string.kujo',out+'/byte.kujo'].to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
report={status:'running',files:files,expected:(value*20)&0xffffffff,iterations:20,rows:[],scope:'Twenty executions of official warmup program per process; process wall includes parse/compile/setup. Seven alternating paired rounds, one excluded warmup. Loaded laptop, no production/competitor claim.'}
save=-> { File.write(out+'/report.json',JSON.pretty_generate(report)) }
begin
  (-1...7).each do |round|
    %w[vm interpreter jit].rotate([round,0].max%3).each_with_index do |mode,index|
      order=(round+index).even? ? %w[string byte] : %w[byte string]
      order.each do |variant|
        files.each { |p,h| raise 'input drift' unless Digest::SHA256.file(p).hexdigest==h }
        argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/'+variant+'.kujo'
        start=Process.clock_gettime(Process::CLOCK_MONOTONIC)
        stdout,stderr,status=Open3.capture3(*argv)
        elapsed=Process.clock_gettime(Process::CLOCK_MONOTONIC)-start
        label="#{round}-#{mode}-#{variant}";File.write(out+'/'+label+'.out',stdout);File.write(out+'/'+label+'.err',stderr)
        passed=status.success? && stdout.strip==report[:expected].to_s
        report[:rows]<<{round:round,warmup:round==-1,runtime:mode,variant:variant,argv:argv,elapsed_seconds:elapsed,exit:status.exitstatus,signal:status.termsig,passed:passed,stdout:stdout,stderr:stderr};save.call
        raise "failed #{label}" unless passed
      end
    end
  end
  files.each { |p,h| raise 'input drift' unless Digest::SHA256.file(p).hexdigest==h }
  report[:comparisons]=%w[vm interpreter jit].map do |mode|
    ratios=(0...7).map do |round|
      pair=report[:rows].select { |r| r[:round]==round && r[:runtime]==mode }.to_h { |r| [r[:variant],r[:elapsed_seconds]] }
      pair['string']/pair['byte']
    end
    {runtime:mode,paired_speedups:ratios,median:ratios.sort[3],range:ratios.minmax}
  end
  report[:status]='collected-review-pending'
rescue => error
  report[:status]='failed';report[:error]=error.message;raise
ensure
  save.call
end
puts JSON.pretty_generate(report[:comparisons])
