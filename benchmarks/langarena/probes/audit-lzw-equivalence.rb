# Repository-maintenance differential oracle. Extract the pinned Rust kernels
# unchanged, compare full encoded bytes and decoded outputs, never time this run.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
arena, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'expected upstream, Kujo binary and new output directory' unless output
raise 'output exists' if File.exist?(output)
revision, status = Open3.capture2('git', '-C', arena, 'rev-parse', 'HEAD')
raise 'wrong upstream' unless status.success? && revision.strip == '3c9f84ddaeec834c0631747cb789e8cfd602f993'
rust_path = arena + '/rust/src/benchmarks/compress.rs'
goose_path = arena + '/goose/src/lzw.goose'
%w[rust/src/benchmarks/compress.rs goose/src/lzw.goose].each do |path|
  pinned, status = Open3.capture2('git', '-C', arena, 'show', "HEAD:#{path}")
  raise "modified pinned source: #{path}" unless status.success? && pinned == File.read(arena + '/' + path)
end
sources = [rust_path, goose_path, 'benchmarks/langarena/port.kujo', binary]
hashes = sources.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
rust = File.read(rust_path)
def extract_function(source, signature)
  start = source.index(signature) or raise "missing #{signature}"
  opening = source.index('{', start)
  depth = 1
  ending = opening + 1
  while depth > 0
    c = source[ending] or raise 'unterminated function'
    depth += 1 if c == '{'
    depth -= 1 if c == '}'
    ending += 1
  end
  source[start...ending]
end
encode = extract_function(rust, 'fn lzw_encode(input: &[u8])')
decode = extract_function(rust, 'fn lzw_decode(encoded: &LZWResult)')
definition = rust[rust.index('#[derive(Default)]', rust.index('pub struct LZWEncode'))...rust.index('impl LZWEncode {')]
raise 'unexpected extracted definition' unless definition.include?('pub struct LZWResult')
cases = [[], [65], [255, 255, 255], [0, 255, 128, 0, 255],
         (0..255).to_a, (0..255).to_a * 2,
         Array.new(50) { |i| 'ABRACADABRA'.bytes[i % 11] },
         Array.new(1000) { |i| 'ABRACADABRA'.bytes[i % 11] }]
FileUtils.mkdir_p(output)
oracle = "// Kernels from pinned LangArena; MIT, see LICENSE.upstream.\nuse std::collections::HashMap;\n#{definition}\nstruct LZWEncode;\nimpl LZWEncode { #{encode} }\n#{decode}\nfn main() {\n"
cases.each do |bytes|
  oracle += "let input: Vec<u8> = vec!#{bytes.to_json}; let encoded = LZWEncode::lzw_encode(&input); assert_eq!(lzw_decode(&encoded), input); println!(\"{:?}\", encoded.data);\n"
end
oracle += "}\n"
File.write(output + '/oracle.rs', oracle)
log, err, status = Open3.capture3('rustc', '--edition=2021', '-O', output + '/oracle.rs', '-o', output + '/oracle')
raise err unless status.success?
encoded, err, status = Open3.capture3(output + '/oracle')
raise err unless status.success?
expected = encoded.lines.map { |line| JSON.parse(line) }
raise 'oracle coverage' unless expected.size == cases.size
port = File.read('benchmarks/langarena/port.kujo')
raise 'runner boundary changed' unless port.scan('func profile_checkpoint(').size == 1
script = port.split('func profile_checkpoint(').first
cases.zip(expected).each do |bytes, codes|
  script += "\nassert_equal(lzw_encode(bytes(#{bytes.to_json})), bytes(#{codes.to_json}))\nassert_equal(lzw_decode(bytes(#{codes.to_json})), bytes(#{bytes.to_json}))\n"
end
script += "print(\"lzw-full-bytes: OK\")\n"
File.write(output + '/contracts.kujo', script)
rows = %w[vm interpreter jit].map do |mode|
  argv = [binary, 'run']
  argv << "--#{mode}" unless mode == 'vm'
  argv << output + '/contracts.kujo'
  out, err, status = Open3.capture3(*argv)
  raise "#{mode}: #{err}" unless status.success? && out.strip == 'lzw-full-bytes: OK'
  {mode: mode, argv: argv, exit: status.exitstatus, stdout: out, stderr: err}
end
sources.each { |p| raise 'source drift' unless hashes[p] == Digest::SHA256.file(p).hexdigest }
toolchain, status = Open3.capture2('rustc', '--version')
raise 'missing toolchain identity' unless status.success?
File.write(output + '/report.json', JSON.pretty_generate({scope: 'Eight full-byte Rust/Kujo differential cases in VM/interpreter/JIT; Goose source review only; no timing', rustc: toolchain.strip, sources: hashes, cases: cases.zip(expected).map { |input, codes| {input: input, encoded: codes} }, rows: rows}) + "\n")
puts 'Eight full-byte cases pass Rust and all three Kujo runtimes.'
