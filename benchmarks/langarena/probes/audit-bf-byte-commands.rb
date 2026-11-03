# Diagnostic representation variant only; canonical port is not edited.
require 'json'
require 'digest'
require 'base64'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==2
binary,out=ARGV.map { |p| File.expand_path(p) };raise 'output exists' if File.exist?(out)
common='benchmarks/langarena/src/00_common.kujo';path='benchmarks/langarena/src/20_text_hash.kujo'
text=File.read(path);kernel=text[/func bf_array\(.*?\n}\n/m];raise 'kernel' unless kernel
byte_kernel=kernel.sub('func bf_array(', 'func bf_byte_array(').sub('bf_commands(text)','bf_byte_commands(text)')
{'+'=>43,'-'=>45,'>'=>62,'<'=>60,'['=>91,']'=>93,'.'=>46}.each { |op,code| byte_kernel=byte_kernel.gsub('== "'+op+'"','== '+code.to_s) }
helper=<<'KUJO'
func bf_byte_commands(text) {
    mut commands := []
    for c in utf8_encode(text) {
        if c == 43 || c == 45 || c == 62 || c == 60 || c == 91 || c == 93 || c == 46 || c == 44 {
            commands = push(commands, c)
        }
    }
    return bytes(commands)
}
KUJO
programs=['','.', 'ignored .','é雪 ignored +.','+.', '-.', '<<<+.', '++++[>+++<-]>.',
 '++[>++[>+++<-]<-]>>.', '+[.-]', '+>++>+++<<<.>.>.', '+++[->+<]>[-].','[]+','[[+]]+.',
 '+'*256+'.','>'*32+'+.<.','++++++++++[.-]','+++++[>+++++[>+<-]<-]>>.',',+.', '>'*30000+'+.<.']
oracle=programs.map do |program|
  ops=program.bytes.select { |b| '+-<>[].,'.bytes.include?(b) };stack=[];jumps={}
  ops.each_with_index do |op,i|
    stack<<i if op==91
    if op==93;start=stack.pop;raise 'unbalanced' unless start;jumps[start]=i;jumps[i]=start;end
  end
  raise 'unbalanced' unless stack.empty?
  tape=Array.new(30000,0);pos=pc=result=steps=0
  while pc<ops.size
    steps+=1;raise 'nonterminating' if steps>100000
    case ops[pc]
    when 43 then tape[pos]=(tape[pos]+1)&255
    when 45 then tape[pos]=(tape[pos]-1)&255
    when 62 then pos+=1;tape<<0 if pos==tape.size
    when 60 then pos=[0,pos-1].max
    when 91 then pc=jumps[pc] if tape[pos]==0
    when 93 then pc=jumps[pc] if tape[pos]!=0
    when 46 then result=(result*4+tape[pos])&0xffffffff
    end
    pc+=1
  end
  [result,pos,Base64.strict_encode64(tape.pack('C*')),steps]
end
trace=lambda do |source,name|
  source.sub(/func \w+\(/,'func '+name+'(')
    .sub('    mut result := 0',"    mut result := 0\n    mut steps := 0")
    .sub('        let op := commands[pc]',"        steps += 1\n        let op := commands[pc]")
    .sub('    return result','    return [result,position,encode_base64(tape),steps]')
end
FileUtils.mkdir_p(out)
source=File.read(common)+"\n"+text+"\n"+helper+byte_kernel+trace.call(kernel,'bf_string_trace')+trace.call(byte_kernel,'bf_byte_trace')
source+="\nlet programs := parse_json(read_file(#{JSON.generate(out+'/programs.json')}))\n"
source+=<<'KUJO'
mut index := 0
for program in programs {
    let before := bf_string_trace(program)
    let after := bf_byte_trace(program)
    assert_equal(bf_array(program), before[0])
    assert_equal(bf_byte_array(program), after[0])
    print(to_json([index,before,after]))
    index += 1
}
KUJO
{'fixture.kujo'=>source,'programs.json'=>JSON.generate(programs),'oracle.json'=>JSON.generate(oracle),'byte-variant.kujo'=>helper+byte_kernel}.each { |name,bytes| File.write(out+'/'+name,bytes) }
paths=[binary,File.expand_path(__FILE__),common,path]+Dir.glob(out+'/*')
files=paths.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
manifest={files:files,runs:[],scope:'Diagnostic command-byte representation; full tape/position/checksum and equal command counts, canonical semantics including comma no-op; no comparator execution or speed claim'}
%w[vm interpreter jit].each do |mode|
  files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
  argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
  stdout,stderr,status=Open3.capture3(*argv);File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
  manifest[:runs]<<{mode:mode,argv:argv,exit:status.exitstatus,signal:status.termsig};File.write(out+'/manifest.json',JSON.pretty_generate(manifest))
  raise "#{mode}: #{stderr}" unless status.success?
  rows=stdout.lines.map { |line| JSON.parse(line) };raise 'coverage' unless rows.size==programs.size
  rows.each_with_index { |row,i| raise "state/count mismatch #{mode}/#{i}" unless row==[i,oracle[i],oracle[i]] }
end
files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',programs:programs.size,complete_states:programs.size*2*3,scope:manifest[:scope]}))
puts "#{programs.size*2*3} full states and equal command counts verified"
