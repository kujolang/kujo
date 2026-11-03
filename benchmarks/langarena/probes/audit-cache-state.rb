# Diagnostic instrumentation only: retain every official 1,000-request iteration.
require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size == 3
arena, binary, out = ARGV.map { |p| File.expand_path(p) }
raise 'existing output' if File.exist?(out)
revision = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators = %w[rust/src/benchmarks/cache_simulation.rs goose/src/cache_simulation.goose]
comparators.each do |path|
  raw, status = Open3.capture2('git', '-C', arena, 'show', revision + ':' + path)
  raise 'upstream drift' unless status.success? && raw == File.read(arena + '/' + path)
end
paths = [binary, __FILE__, 'benchmarks/langarena/probes/verify-cache-state.rb',
         'benchmarks/langarena/port.kujo', 'benchmarks/langarena/src/47_cache.kujo',
         'benchmarks/langarena/src/00_common.kujo'] + comparators.map { |p| arena + '/' + p }
hashes = paths.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
source = File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel = File.read('benchmarks/langarena/src/47_cache.kujo')[/func cache_run\(.*?\n}\n/m]
raise 'kernel missing' unless kernel && source.include?(kernel)
needle = "        i += 1\n"
raise 'ambiguous instrumentation' unless kernel.scan(needle).size == 1
diag = kernel.sub('func cache_run(input, iteration)', 'func cache_observe(input, iteration, case_id)')
diag = diag.sub(needle, needle + "        cache_dump(state, case_id, iteration, i)\n")
source += <<~KUJO

func cache_rows(state) {
    mut rows := []
    mut cursor := state["head"]
    while cursor != null {
        let node := state["nodes"][cursor]
        rows = push(rows, [cursor,node.value,node.previous,node.next])
        assert(len(rows) <= len(state["nodes"]))
        cursor = node.next
    }
    assert_equal(len(rows),len(state["nodes"]))
    return rows
}
func cache_dump(state, case_id, iteration, step) {
    let rows := cache_rows(state)
    print(to_json([case_id,iteration,step,rng,state["head"],state["tail"],state["hits"],state["misses"],rows]))
}
#{diag}
let configs := [{"size":1,"values":1},{"size":2,"values":5},{"size":8,"values":17}]
mut case_id := 0
for cfg in configs {
    for seed in [42,139967] {
        rng = seed
        mut state := cache_create(cfg)
        mut iteration := 0
        while iteration < 2 {
            let snapshot := state
            let saved := cache_rows(snapshot)
            let start_rng := rng
            let observed := cache_observe(state,iteration,case_id)
            let end_rng := rng
            rng = start_rng
            let original := cache_run(state,iteration)
            assert_equal(rng,end_rng)
            assert_equal(original,observed)
            assert_equal(cache_rows(snapshot),saved)
            assert_equal(cache_rows(state),saved)
            state = observed
            print(to_json(["end",case_id,iteration,cache_checksum(state)]))
            iteration += 1
        }
        case_id += 1
    }
}
KUJO
Dir.mkdir(out)
File.write(out + '/contracts.kujo', source)
hashes[out + '/contracts.kujo'] = Digest::SHA256.hexdigest(source)
manifest = {files: hashes, upstream: revision, configurations: [[1,1],[2,5],[8,17]], seeds: [42,139967], iterations: 2, requests: 1000}
File.write(out + '/manifest.json', JSON.pretty_generate(manifest))
rows = []
%w[vm interpreter jit].each do |mode|
  argv = [binary, 'run', '--scheduler-timeout-ms', '120000']
  argv << '--' + mode unless mode == 'vm'
  argv << out + '/contracts.kujo'
  stdout, stderr, status = Open3.capture3(*argv)
  File.write(out + '/' + mode + '.out', stdout)
  File.write(out + '/' + mode + '.err', stderr)
  rows << {runtime: mode, argv: argv, exit: status.exitstatus, signal: status.termsig,
           stdout_sha256: Digest::SHA256.hexdigest(stdout), stderr_sha256: Digest::SHA256.hexdigest(stderr)}
  File.write(out + '/runs.json', JSON.pretty_generate(rows))
  raise "runtime failed: #{mode}" unless status.success?
end
hashes.each { |p,h| raise 'source drift' unless Digest::SHA256.file(p).hexdigest == h }
puts 'Collected cache transition states; independent verification required.'
