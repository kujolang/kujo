require 'json'
require 'digest'
require 'base64'
require 'open3'
require_correct = ARGV.delete('--require-correct-jumps')
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT [--require-correct-jumps]' unless ARGV.size == 3
arena,binary,out = ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
comparators = %w[rust/src/benchmarks/brainfuck_array.rs goose/src/brainfuck_array.goose]
comparators.each do |path|
  raw,status = Open3.capture2('git','-C',arena,'show','3c9f84ddaeec834c0631747cb789e8cfd602f993:'+path)
  raise 'upstream drift' unless status.success? && raw == File.read(arena+'/'+path)
end
paths = [binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/20_text_hash.kujo','benchmarks/langarena/src/00_common.kujo'] + comparators.map { |p| arena+'/'+p }
hashes = paths.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
programs = ['.', 'ignored .', '+.', '-.', '<<<+.', '++++[>+++<-]>.',
            '++[>++[>+++<-]<-]>>.', '+[.-]', '+>++>+++<<<.>.>.',
            '+++[->+<]>[-].', '[]+', '[[+]]+.', '+'*256+'.', '>'*32+'+.<.',
            '++++++++++[.-]', '+++++[>+++++[>+<-]<-]>>.']
oracle = programs.map do |text|
  ops = text.chars.select { |c| '+-<>[].'.include?(c) }
  stack = []; jumps = {}
  ops.each_with_index do |op,i|
    stack << i if op == '['
    if op == ']'
      start = stack.pop; raise 'unbalanced' unless start
      jumps[start] = i; jumps[i] = start
    end
  end
  raise 'unbalanced' unless stack.empty?
  tape = Array.new(30000,0); pos = pc = sum = steps = taken = 0
  while pc < ops.size
    steps += 1; raise 'nontermination' if steps > 100000
    case ops[pc]
    when '+' then tape[pos] = (tape[pos]+1)&255
    when '-' then tape[pos] = (tape[pos]-1)&255
    when '>' then pos += 1; tape << 0 if pos == tape.size
    when '<' then pos = [0,pos-1].max
    when '[' then if tape[pos].zero?; pc = jumps[pc]; taken += 1; end
    when ']' then unless tape[pos].zero?; pc = jumps[pc]; taken += 1; end
    when '.' then sum = (sum*4+tape[pos])&0xffffffff
    end
    pc += 1
  end
  {result:sum,position:pos,tape:Base64.strict_encode64(tape.pack('C*')),steps:steps,taken:taken}
end
source = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel = File.read('benchmarks/langarena/src/20_text_hash.kujo')[/func bf_array\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && source.include?(kernel)
old_jump = 'pc = jumps[pc] continue'
new_jump = 'pc = jumps[pc] + 1 continue'
canonical_fixed = kernel.scan(new_jump).size == 2 && kernel.scan(old_jump).empty?
raise 'unexpected jump sites' unless canonical_fixed || (kernel.scan(old_jump).size == 2 && kernel.scan(new_jump).empty?)
raise 'canonical jumps still redispatch brackets' if require_correct && !canonical_fixed
before = kernel.gsub(new_jump,old_jump).sub('func bf_array(', 'func bf_array_before(')
before = before.sub('    mut result := 0',"    mut result := 0\n    mut steps := 0")
before = before.sub('        let op := commands[pc]',"        steps += 1\n        let op := commands[pc]")
before = before.sub('    return result', '    return [result,position,encode_base64(tape),steps]')
after = before.sub('bf_array_before(', 'bf_array_after(').gsub('pc = jumps[pc] continue','pc = jumps[pc] + 1 continue')
source += "\n"+before+"\n"+after+"\nlet programs := #{JSON.generate(programs)}\n"
source += <<~KUJO
mut case_id := 0
for text in programs {
    let original := bf_array(text)
    let before := bf_array_before(text)
    let after := bf_array_after(text)
    assert_equal(original,before[0])
    assert_equal(original,after[0])
    print(to_json([case_id,before,after]))
    case_id += 1
}
KUJO
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source);File.write(out+'/programs.json',JSON.pretty_generate(programs));File.write(out+'/oracle.json',JSON.pretty_generate(oracle))
hashes[out+'/contracts.kujo'] = Digest::SHA256.hexdigest(source)
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,canonical_fixed:canonical_fixed,scope:'baseline/corrected state and command-count comparison; canonical source unchanged by collector'}))
runs = []
%w[vm interpreter jit].each do |mode|
  argv = [binary,'run','--scheduler-timeout-ms','120000'];argv << '--'+mode unless mode == 'vm';argv << out+'/contracts.kujo'
  stdout,stderr,status = Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
  runs << {runtime:mode,argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/runs.json',JSON.pretty_generate(runs));raise 'runtime failure '+mode unless status.success?
  rows = stdout.lines.map { |l| JSON.parse(l) };raise 'coverage' unless rows.size == programs.size
  rows.each_with_index do |row,i|
    ref = oracle[i];raise 'case order' unless row[0] == i
    prefix = ref.values_at(:result,:position,:tape)
    raise 'baseline state/count mismatch' unless row[1] == prefix+[ref[:steps]+ref[:taken]]
    raise 'candidate state/count mismatch' unless row[2] == prefix+[ref[:steps]]
  end
end
hashes.each { |p,h| raise 'source drift' unless Digest::SHA256.file(p).hexdigest == h }
summary = {status:'verified',programs:16,runtimes:3,complete_states:96,
           canonical_fixed:canonical_fixed,
           target_programs:oracle.count { |r| r[:taken]>0 },
           baseline_steps:oracle.sum { |r| r[:steps]+r[:taken] },candidate_steps:oracle.sum { |r| r[:steps] },
           scope:'Counts per one pass of16 fixtures; full tape/position/checksum equivalence; no actual comparator execution or speed claim'}
File.write(out+'/verification.json',JSON.pretty_generate(summary));puts JSON.generate(summary)
