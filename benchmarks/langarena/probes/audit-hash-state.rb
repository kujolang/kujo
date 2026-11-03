# Full digest/state audit; SHA256 here is LangArena's toy hash, not SHA-256.
require 'json'
require 'digest'
require 'open3'
require 'zlib'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena, binary, out = ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
comparators = %w[rust/src/benchmarks/buffer_hash_benchmark.rs rust/src/benchmarks/buffer_hash_sha256.rs rust/src/benchmarks/buffer_hash_crc32.rs goose/src/buffer_hash.goose]
comparators.each do |path|
  pinned, status = Open3.capture2('git', '-C', arena, 'show', '3c9f84ddaeec834c0631747cb789e8cfd602f993:' + path)
  raise 'upstream drift' unless status.success? && pinned == File.read(arena + '/' + path)
end
files = [binary, __FILE__, 'benchmarks/langarena/port.kujo', 'benchmarks/langarena/src/20_text_hash.kujo'] + comparators.map { |p| arena + '/' + p }
hashes = files.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
inputs = [0,1,2,7,8,9,15,16,31,32,33,255,256,257].flat_map do |n|
  [Array.new(n,0), Array.new(n,255), Array.new(n) { |i| i % 256 }, Array.new(n) { |i| (i*73+19)%256 }]
end
inputs << '123456789'.bytes
source = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel = File.read('benchmarks/langarena/src/20_text_hash.kujo')[/func hash_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && source.include?(kernel)
diagnostic = kernel.sub('func hash_run(', 'func hash_state_probe(')
needle = '        result = digest[0]'
raise 'digest insertion' unless diagnostic.scan(needle).size == 1
diagnostic = diagnostic.sub(needle, "        mut output := []\n        for byte in digest { output = push(output, byte) }\n        state[\"digest\"] = output\n" + needle)
needle = '    state["result"] = u32(state["result"] + result)'
raise 'hash insertion' unless diagnostic.scan(needle).size == 1
diagnostic = diagnostic.sub(needle, "    state[\"hash\"] = result\n" + needle)
source += "\n" + diagnostic + "\nlet hash_inputs := #{JSON.generate(inputs)}\n"
source += <<~KUJO
  mut input_index := 0
  for input in hash_inputs {
      for name in ["Hash::CRC32", "Hash::SHA256"] {
          for seed in [0,4294967294] {
              mut original := {"cfg":{"name":name},"data":bytes(input),"result":seed}
              mut observed := original
              let snapshot := original
              mut iteration := 0
              while iteration < 3 {
                  original = hash_run(original,iteration)
                  observed = hash_state_probe(observed,iteration)
                  assert_equal(original["result"],observed["result"])
                  assert_equal(original["data"],bytes(input))
                  assert_equal(observed["data"],bytes(input))
                  assert_equal(snapshot,{"cfg":{"name":name},"data":bytes(input),"result":seed})
                  mut digest := []
                  if name == "Hash::SHA256" { digest = observed["digest"] }
                  print(to_json([input_index,name,seed,iteration,original["result"],observed["hash"],digest]))
                  iteration += 1
              }
          }
      }
      input_index += 1
  }
KUJO
expected = []
inputs.each_with_index do |input, index|
  lanes = [0x6a09e667,0xbb67ae85,0x3c6ef372,0xa54ff53a,0x510e527f,0x9b05688c,0x1f83d9ab,0x5be0cd19]
  input.each_with_index do |byte, i|
    h = (lanes[i%8]*33+byte)&0xffffffff
    lanes[i%8] = ((h+(h<<10))&0xffffffff)^(h>>6)
  end
  digest = lanes.pack('N*').bytes
  %w[Hash::CRC32 Hash::SHA256].each do |name|
    value = name == 'Hash::CRC32' ? Zlib.crc32(input.pack('C*')) : digest[0,4].pack('C*').unpack1('V')
    [0,4294967294].each do |seed|
      3.times { |i| expected << [index,name,seed,i,(seed+(i+1)*value)&0xffffffff,value,name == 'Hash::SHA256' ? digest : []] }
    end
  end
end
raise 'fixture limits' unless inputs.size == 57 && source.bytesize <= 1048576
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/inputs.json',JSON.pretty_generate(inputs))
%w[contracts.kujo inputs.json].each { |f| hashes[out+'/'+f] = Digest::SHA256.file(out+'/'+f).hexdigest }
rows = %w[vm interpreter jit].map do |mode|
  argv = [binary,'run','--scheduler-timeout-ms','120000']
  argv << '--'+mode unless mode == 'vm'
  argv << out+'/contracts.kujo'
  stdout, stderr, status = Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout)
  File.write(out+'/'+mode+'.err',stderr)
  raise "runtime failure #{mode}: #{stderr}" unless status.success?
  raise "complete digest/hash mismatch #{mode}" unless stdout.lines.map { |line| JSON.parse(line) } == expected
  {runtime:mode,argv:argv,exit:status.exitstatus,observations:expected.size,stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,rows:rows,scope:'2052 full digest/hash/checksum observations; 57 byte inputs, three iterations, two seeds, three runtimes. No comparator execution, production, timing or native-participation claim.'}))
puts 'Verified2052 complete hash-state observations.'
