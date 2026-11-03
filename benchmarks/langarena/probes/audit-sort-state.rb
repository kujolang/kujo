# Execute unchanged pinned sort kernels and compare every output element.
# Diagnostic inputs; no performance or equal-work claim.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
abort 'usage: ruby audit-sort-state.rb UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.length == 3
arena, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(output)
revision = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths = %w[rust/src/helper.rs rust/src/benchmarks/sort_benchmark.rs rust/src/benchmarks/sort_quick.rs rust/src/benchmarks/sort_merge.rs]
originals = paths.to_h do |path|
  source = File.read(File.join(arena, path))
  pinned, status = Open3.capture2('git', '-C', arena, 'show', "#{revision}:#{path}")
  raise "upstream drift #{path}" unless status.success? && source == pinned
  [path, source]
end
sources = %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/30_sort.kujo]
identities = (sources + paths.map { |p| File.join(arena, p) } + [binary, __FILE__]).to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
cases = [1, 2, 3, 31, 64, 257].flat_map do |n|
  ordered = (0...n).map { |i| i - n / 2 }
  random = [1, 42, 99].map do |seed|
    rng = seed
    Array.new(n) { rng = (rng * 3877 + 29573) % 139968; rng % 41 - 20 }
  end
  [ordered, ordered.reverse, Array.new(n, 7), Array.new(n) { |i| i.even? ? -9 : 9 }, ordered.sort_by(&:abs)] + random
end
cases << [-2147483648, 2147483647, 0, -1, 1, -2147483648, 2147483647]
FileUtils.mkdir_p(output)
helper = originals.fetch(paths[0]).split('pub fn checksum_str', 2).first
rust = <<~RUST
  #![allow(dead_code)]
  mod helper { #{helper} }
  trait Benchmark { fn name(&self)->String; fn prepare(&mut self); fn run(&mut self,i:i64); fn checksum(&self)->u32; }
  fn config_i64(_: &str,_: &str)->i64 { panic!("diagnostic supplies inputs directly") }
  mod benchmarks {
    pub mod sort_benchmark { #{originals.fetch(paths[1])} }
    pub mod sort_quick {
      #{originals.fetch(paths[2])}
      pub fn diagnostic(input:&[i32])->Vec<i32> { let mut a=input.to_vec(); SortQuick::quick_sort(&mut a); a }
    }
    pub mod sort_merge {
      #{originals.fetch(paths[3])}
      pub fn diagnostic(input:&[i32])->Vec<i32> { let mut a=input.to_vec(); SortMerge::merge_sort_inplace(&mut a); a }
    }
  }
  fn main() {
    let cases:Vec<Vec<i32>>=vec![#{cases.map { |a| "vec![#{a.join(',')}]" }.join(',')}];
    for input in cases {
      println!("{:?}",benchmarks::sort_quick::diagnostic(&input));
      println!("{:?}",benchmarks::sort_merge::diagnostic(&input));
    }
  }
RUST
File.write(output + '/oracle.rs', rust)
kujo = sources.map { |p| File.read(p) }.join("\n")
kujo += <<~KUJO
  let cases := #{cases.to_json}
  for input in cases {
      let snapshot := input
      let quick := quick_sort(input, 0, len(input) - 1)
      assert_equal(input, snapshot)
      let initial := [input, vector(len(input), 0)]
      let initial_snapshot := initial
      let merged := merge_sort(initial, 0, len(input) - 1)
      assert_equal(input, snapshot)
      assert_equal(initial, initial_snapshot)
      print(to_json(quick))
      print(to_json(merged[0]))
  }
KUJO
File.write(output + '/probe.kujo', kujo)
identities[output + '/oracle.rs'] = Digest::SHA256.file(output + '/oracle.rs').hexdigest
identities[output + '/probe.kujo'] = Digest::SHA256.file(output + '/probe.kujo').hexdigest
File.write(output + '/input-identities.json', JSON.pretty_generate(identities) + "\n")
verify = -> { identities.each { |p,h| raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest == h } }
receipts = []
run = lambda do |label, argv|
  verify.call
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  pid = Process.spawn(*argv, out: output + "/#{label}.out", err: output + "/#{label}.err", pgroup: true)
  result = nil
  timed_out = false
  loop do
    if (waited = Process.wait2(pid, Process::WNOHANG))
      result = waited.last
      break
    end
    if Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 120
      timed_out = true
      Process.kill('TERM', -pid) rescue nil
      sleep 0.2
      Process.kill('KILL', -pid) rescue nil
      result = Process.wait2(pid).last
      break
    end
    sleep 0.02
  end
  receipts << {label: label, argv: argv, exit: result.exitstatus, signal: result.termsig, timed_out: timed_out}
  File.write(output + '/commands.json', JSON.pretty_generate(receipts) + "\n")
  raise "#{label} failed" unless result.success? && !timed_out
  verify.call
  File.read(output + "/#{label}.out")
end
run.call('rust-build', ['rustc', '--edition=2021', '-O', output + '/oracle.rs', '-o', output + '/oracle'])
identities[output + '/oracle'] = Digest::SHA256.file(output + '/oracle').hexdigest
expected = cases.flat_map { |a| [a.sort, a.sort] }
observed = {}
observed['rust'] = run.call('rust', [output + '/oracle']).lines.map { |l| JSON.parse(l) }
%w[vm interpreter jit].each do |mode|
  argv = [binary, 'run', '--scheduler-no-timeout'] + (mode == 'vm' ? [] : ["--#{mode}"]) + [output + '/probe.kujo']
  observed[mode] = run.call(mode, argv).lines.map { |l| JSON.parse(l) }
end
observed.each { |mode, rows| raise "full-output mismatch #{mode}" unless rows == expected }
raise 'unexpected diagnostics' unless %w[rust vm interpreter].all? { |mode| File.zero?(output + "/#{mode}.err") }
jit_diagnostic = File.read(output + '/jit.err')
expected_admission = "JIT opt-in requested; unsupported bytecode regions will use the VM (unsupported opcode in chunk '<script>' at instruction 0: MakeClosure(0)).\n"
raise 'unexpected JIT diagnostics' unless jit_diagnostic.empty? || jit_diagnostic == expected_admission
verify.call
report = {status: 'passed', upstream: revision, cases: cases, sources: identities, jit_diagnostic: jit_diagnostic,
          arrays_per_engine: expected.size, engines: observed.keys,
          scope: 'Full QuickSort/MergeSort output arrays and Kujo input snapshots on 49 nonempty diagnostic inputs; not production, operation-count equivalence, Goose coverage, native-kernel participation proof or timing. JIT script admission warning is retained; it is not an execution failure. Empty input omitted because pinned Rust MergeSort wrapper subtracts one from length.'}
File.write(output + '/report.json', JSON.pretty_generate(report) + "\n")
puts "Verified #{expected.size} complete sort outputs per engine across Rust and three Kujo runtimes."
