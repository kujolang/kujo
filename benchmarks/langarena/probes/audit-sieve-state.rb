# Maintenance oracle, not a timing harness. Preserve the original kernel and
# compare a diagnostic clone exposing its final flag buffer against trial division.
require 'json'
require 'digest'
require 'base64'
require 'open3'

raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(output)
revision = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths = %w[rust/src/benchmarks/sieve.rs goose/src/sieve.goose]
paths.each do |path|
  pinned, status = Open3.capture2('git', '-C', arena, 'show', revision + ':' + path)
  raise 'upstream source drift' unless status.success? && pinned == File.read(arena + '/' + path)
end
files = [binary, __FILE__, 'benchmarks/langarena/port.kujo',
         'benchmarks/langarena/src/10_basic.kujo'] + paths.map { |p| arena + '/' + p }
hashes = files.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
port = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel = File.read('benchmarks/langarena/src/10_basic.kujo')[/func sieve_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && port.include?(kernel)
diagnostic = kernel.sub('func sieve_run(', 'func sieve_state_probe(')
raise 'single return required' unless diagnostic.scan('    return state').size == 1
diagnostic = diagnostic.sub('    return state', '    state["flags"] = primes' + "\n    return state")
# The sole kernel change exports final flags after checksum calculation. Running
# both versions also checks that exporting state did not change the result.
limits = [2, 3, 4, 8, 9, 10, 24, 25, 26, 48, 49, 50, 120, 121, 122, 1000, 1024, 4096]
seeds = [0, 4294967294]
cases = limits.product(seeds)
source = port + "\n" + diagnostic
cases.each_with_index do |(limit, seed), i|
  source += <<~KUJO
    mut original#{i} := {"cfg": {"limit": #{limit}}, "result": #{seed}}
    mut observed#{i} := original#{i}
    mut iteration#{i} := 0
    while iteration#{i} < 2 {
        original#{i} = sieve_run(original#{i}, iteration#{i})
        observed#{i} = sieve_state_probe(observed#{i}, iteration#{i})
        assert_equal(original#{i}["result"], observed#{i}["result"])
        print("#{limit}|#{seed}|" + str(iteration#{i}) + "|" + str(original#{i}["result"]) + "|" + encode_base64(observed#{i}["flags"]))
        iteration#{i} += 1
    }
  KUJO
end
raise 'source limit' if source.bytesize > 1048576
Dir.mkdir(output)
File.write(output + '/contracts.kujo', source)
hashes[output + '/contracts.kujo'] = Digest::SHA256.hexdigest(source)
rows = []
%w[vm interpreter jit].each do |mode|
  argv = [binary, 'run', '--scheduler-timeout-ms', '60000']
  argv << '--' + mode unless mode == 'vm'
  argv << output + '/contracts.kujo'
  stdout, stderr, status = Open3.capture3(*argv)
  File.write(output + '/' + mode + '.out', stdout)
  File.write(output + '/' + mode + '.err', stderr)
  raise 'runtime failure: ' + mode unless status.success?
  expected_cases = cases.flat_map { |limit, seed| [0, 1].map { |i| [limit, seed, i] } }
  raise 'coverage' unless stdout.lines.size == expected_cases.size
  results = stdout.lines.zip(expected_cases).map do |line, (limit, seed, iteration)|
    n, initial, index, result, encoded = line.strip.split('|', -1)
    raise 'case identity' unless [n, initial, index] == [limit, seed, iteration].map(&:to_s)
    flags = (0..limit).map { |v| v >= 2 && (2...v).none? { |d| v % d == 0 } ? 1 : 0 }
    actual = Base64.strict_decode64(encoded).bytes
    raise 'full flag mismatch' unless actual == flags
    primes = (0..limit).select { |v| flags[v] == 1 }
    expected = (seed + (iteration + 1) * (primes.last + primes.size)) & 0xffffffff
    raise 'checksum mismatch' unless result == expected.to_s
    {limit: limit, initial: seed, iteration: iteration, result: expected,
     prime_count: primes.size, last_prime: primes.last,
     flags_sha256: Digest::SHA256.hexdigest(actual.pack('C*'))}
  end
  rows << {runtime: mode, argv: argv, exit: status.exitstatus, cases: results,
           stdout_sha256: Digest::SHA256.hexdigest(stdout), stderr_sha256: Digest::SHA256.hexdigest(stderr)}
end
hashes.each { |p, hash| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == hash }
File.write(output + '/report.json', JSON.pretty_generate({files: hashes, rows: rows,
  scope: '216 complete-buffer observations, original/diagnostic checksum agreement, square boundaries and u32 accumulation wrap. Pinned Rust/Goose sources inspected, not executed. No operation counts, production coverage, native-path proof, or timing claim.'}) + "\n")
puts '216 complete Sieve flag buffers and original-kernel checksums verified.'
