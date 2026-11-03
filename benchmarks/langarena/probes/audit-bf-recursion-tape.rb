require 'json'
require 'digest'
require 'base64'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena, binary, out = ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
comparators = %w[rust/src/benchmarks/brainfuck_recursion.rs goose/src/brainfuck_recursion.goose]
comparators.each do |path|
  raw, status = Open3.capture2('git','-C',arena,'show','3c9f84ddaeec834c0631747cb789e8cfd602f993:' + path)
  raise 'upstream drift' unless status.success? && raw == File.read(arena + '/' + path)
end
paths = [binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/20_text_hash.kujo','benchmarks/langarena/src/00_common.kujo'] + comparators.map { |p| arena + '/' + p }
hashes = paths.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
programs = ['', 'comment', '.', '+.', '-.', '+,.-.', '<<<+.', '++++[>+++<-]>.',
            '++[>++[>+++<-]<-]>>.', '+[.-]', '+>++>+++<<<.>.>.',
            '+++[->+<]>[-].', '[]+', '[[+]]+.', '+'*256+'.', '>'*32+'+.<.']
# Independent iterative interpreter; does not use Kujo's recursive AST algorithm.
oracle = programs.map do |text|
  ops = text.chars.select { |c| '+-<>[].'.include?(c) }
  jumps = {}; stack = []
  ops.each_with_index do |c,i|
    stack << i if c == '['
    if c == ']'
      start = stack.pop; raise 'unbalanced' unless start
      jumps[i] = start; jumps[start] = i
    end
  end
  raise 'unbalanced' unless stack.empty?
  tape = [0]; pos = pc = sum = steps = 0
  while pc < ops.length
    steps += 1; raise 'nonterminating fixture' if steps > 100000
    case ops[pc]
    when '+' then tape[pos] = (tape[pos]+1)&255
    when '-' then tape[pos] = (tape[pos]-1)&255
    when '>' then pos += 1; tape << 0 if pos == tape.size
    when '<' then pos = [0,pos-1].max
    when '[' then pc = jumps[pc] if tape[pos].zero?
    when ']' then pc = jumps[pc] unless tape[pos].zero?
    when '.' then sum = (sum*4+tape[pos])&0xffffffff
    end
    pc += 1
  end
  [sum,pos,tape]
end
source = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel = File.read('benchmarks/langarena/src/20_text_hash.kujo')[/func bf_text\(.*?\n}\n/m]
raise 'kernel missing' unless kernel && source.include?(kernel)
raise 'initial tape changed' unless kernel.scan('bytes(vector(30000, 0))').size == 1
baseline = kernel.sub('func bf_text(', 'func bf_tape_baseline(').sub('return machine["result"]','return [machine["result"],machine["position"],encode_base64(machine["tape"])]')
candidate = baseline.sub('bf_tape_baseline(', 'bf_tape_candidate(').sub('bytes(vector(30000, 0))','bytes([0])')
source += "\n" + baseline + "\n" + candidate + "\nlet programs := #{JSON.generate(programs)}\n"
source += <<~KUJO
mut case_id := 0
for text in programs {
    let cfg := {"name":"Brainfuck::Recursion"}
    let original := bf_text(cfg,text)
    let before := bf_tape_baseline(cfg,text)
    let after := bf_tape_candidate(cfg,text)
    assert_equal(original,before[0])
    assert_equal(original,after[0])
    print(to_json([case_id,before,after]))
    case_id += 1
}
KUJO
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/programs.json',JSON.pretty_generate(programs))
File.write(out+'/oracle.json',JSON.pretty_generate(oracle))
hashes[out+'/contracts.kujo'] = Digest::SHA256.hexdigest(source)
File.write(out+'/manifest.json',JSON.pretty_generate({files:hashes,scope:'isolated one-cell port candidate only; original port unchanged'}))
runs = []
%w[vm interpreter jit].each do |mode|
  argv = [binary,'run','--scheduler-timeout-ms','120000']; argv << '--'+mode unless mode == 'vm'; argv << out+'/contracts.kujo'
  stdout,stderr,status = Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout); File.write(out+'/'+mode+'.err',stderr)
  runs << {runtime:mode,argv:argv,exit:status.exitstatus,signal:status.termsig,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
  File.write(out+'/runs.json',JSON.pretty_generate(runs))
  raise 'runtime failure '+mode unless status.success?
  rows = stdout.lines.map { |l| JSON.parse(l) }; raise 'case coverage' unless rows.size == programs.size
  rows.each_with_index do |row,i|
    raise 'case order' unless row[0] == i
    sum,pos,tape = oracle[i]
    before,after = row[1],row[2]
    raise 'checksum/position mismatch' unless before[0,2] == [sum,pos] && after[0,2] == [sum,pos]
    raise 'baseline tape mismatch' unless Base64.strict_decode64(before[2]).bytes == tape + [0]*(30000-tape.size)
    raise 'candidate full tape mismatch' unless Base64.strict_decode64(after[2]).bytes == tape
  end
end
hashes.each { |p,h| raise 'source drift' unless Digest::SHA256.file(p).hexdigest == h }
File.write(out+'/verification.json',JSON.pretty_generate({status:'verified',programs:programs.size,runtimes:3,complete_tapes:96,baseline_shape_mismatches:48,candidate_shape_mismatches:0,scope:'Independent final tape/position/checksum oracle for isolated one-cell variant; no runtime or canonical port changes, executed comparator or timing claim.'}))
puts 'Verified96 complete tapes; baseline has48 shape mismatches versus one-cell model, isolated candidate zero.'
