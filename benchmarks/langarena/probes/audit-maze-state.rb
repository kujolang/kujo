# Repository-maintenance oracle: execute original Rust and Kujo algorithms.
# No timing claims; diagnostic dimensions/seeds do not replace official inputs.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'

arena, binary, output = ARGV.map { |path| File.expand_path(path) }
raise 'expected UPSTREAM FROZEN_KUJO NEW_OUTPUT_DIRECTORY' unless output
raise 'output already exists' if File.exist?(output)
revision = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths = %w[rust/src/benchmarks/maze.rs rust/src/helper.rs]
originals = paths.to_h do |path|
  source = File.read(File.join(arena, path))
  pinned, status = Open3.capture2('git', '-C', arena, 'show', "#{revision}:#{path}")
  raise "upstream drift: #{path}" unless status.success? && source == pinned
  [path, source]
end
sources = %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/40_graph.kujo benchmarks/langarena/src/80_maze.kujo]
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
identities = (sources + paths.map { |p| File.join(arena, p) } + [binary, __FILE__]).to_h { |p| [p, sha.call(p)] }
cases = [5, 8, 17].flat_map { |width| [1, 42, 99].map { |seed| [width, width == 17 ? 13 : width, seed] } }
cases << [200, 30, 42]
FileUtils.mkdir_p(output)

# The entire maze module is retained byte-for-byte. A sibling diagnostic function
# in that module can inspect private state. Config constructors are not invoked.
helper = originals.fetch('rust/src/helper.rs').split('pub fn checksum_str', 2).first
rust = <<~RUST
  #![allow(dead_code)]
  mod helper { #{helper} }
  trait Benchmark { fn name(&self)->String; fn prepare(&mut self); fn run(&mut self, i:i64); fn checksum(&self)->u32; }
  fn config_i64(_: &str, _: &str)->i64 { panic!("diagnostic must supply dimensions explicitly") }
  mod benchmarks { pub mod maze {
  #{originals.fetch('rust/src/benchmarks/maze.rs')}
  fn state(case: usize, phase: usize, maze: &Maze, bfs: Vec<usize>, astar: Vec<usize>) {
      let kinds: Vec<u32> = maze.cells.iter().flatten().map(|c| c.kind.value()).collect();
      let neighbors: Vec<Vec<usize>> = maze.cells.iter().flatten().map(|c| {
          if c.kind == CellKind::Border { vec![] }
          else { c.neighbors.iter().map(|&(y,x)| y*maze.width+x).collect() }
      }).collect();
      println!("{{\\\"case\\\":{},\\\"phase\\\":{},\\\"rng\\\":{},\\\"kinds\\\":{:?},\\\"neighbors\\\":{:?},\\\"bfs\\\":{:?},\\\"astar\\\":{:?}}}", case, phase, helper::last(), kinds, neighbors, bfs, astar);
  }
  pub fn run(case: usize, width: usize, height: usize, seed: i32) {
      helper::set_last(seed);
      let mut maze = Maze::new(width, height);
      state(case, 0, &maze, vec![], vec![]);
      for phase in 1..=3 {
          if phase > 1 { maze.reset(); }
          maze.generate();
          let solver = MazeBFS { maze, path: vec![], result_val: 0 };
          let bfs = solver.bfs(solver.maze.get_start(), solver.maze.get_finish()).iter().map(|&(y,x)|y*width+x).collect();
          let solver = MazeAStar { maze: solver.maze, path: vec![], result_val: 0 };
          let astar = solver.astar(solver.maze.get_start(), solver.maze.get_finish()).iter().map(|&(y,x)|y*width+x).collect();
          maze = solver.maze;
          state(case, phase, &maze, bfs, astar);
      }
  }
  }}
  fn main() {
  #{cases.each_with_index.map { |(w,h,s), i| "benchmarks::maze::run(#{i},#{w},#{h},#{s});" }.join("\n")}
  }
RUST
# Rust format braces need escaping, independently from Ruby's interpolation.
rust.sub!('println!("{\\"case', 'println!("{{\\"case')
rust.sub!('\\"astar\\":{:?}}",', '\\"astar\\":{:?}}}",')
File.write(output + '/oracle.rs', rust)

kujo = sources.map { |path| File.read(path) }.join("\n")
kujo += <<~KUJO
  func dump(case_id, phase, state, bfs, astar) {
      mut kinds := []
      mut neighbors := []
      for cell in state["cells"] { kinds = push(kinds, cell.kind) neighbors = push(neighbors, cell.neighbors) }
      print(to_json({"case": case_id, "phase": phase, "rng": rng, "kinds": kinds, "neighbors": neighbors, "bfs": bfs, "astar": astar}))
  }
  let cases := #{cases.to_json}
  mut case_id := 0
  for entry in cases {
      rng = entry[2]
      mut state := maze_create({"name": "Maze::Generator", "w": entry[0], "h": entry[1]})
      dump(case_id, 0, state, [], [])
      mut phase := 1
      while phase <= 3 {
          state = maze_run(state, phase)
          dump(case_id, phase, state, maze_bfs(state), maze_astar(state))
          phase += 1
      }
      case_id += 1
  }
KUJO
File.write(output + '/probe.kujo', kujo)

receipts = []
run = lambda do |label, argv|
  started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  pid = Process.spawn(*argv, out: output + "/#{label}.stdout", err: output + "/#{label}.stderr", pgroup: true)
  status = nil
  deadline = false
  until status
    waited = Process.wait2(pid, Process::WNOHANG)
    if waited
      status = waited[1]
    elsif Process.clock_gettime(Process::CLOCK_MONOTONIC) - started > 600
      deadline = true
      Process.kill('TERM', -pid) rescue nil
      sleep 1
      Process.kill('KILL', -pid) rescue nil
      status = Process.wait2(pid)[1]
    else
      sleep 0.1
    end
  end
  receipts << {label: label, argv: argv, exit: status.exitstatus, signal: status.termsig, deadline: deadline}
  File.write(output + '/commands.json', JSON.pretty_generate(receipts))
  raise "#{label} failed; inspect retained stdout/stderr" unless status.success? && !deadline
  File.read(output + "/#{label}.stdout")
end
run.call('build', ['rustc', '--edition=2021', output + '/oracle.rs', '-o', output + '/oracle'])
rust_rows = run.call('rust', [output + '/oracle']).lines.map { |line| JSON.parse(line) }
all = {}
%w[vm interpreter jit].each do |mode|
  argv = [binary, 'run', '--scheduler-no-timeout']
  argv << "--#{mode}" unless mode == 'vm'
  all[mode] = run.call(mode, argv + [output + '/probe.kujo']).lines.map { |line| JSON.parse(line) }
end
expected_order = cases.each_index.flat_map { |i| (0..3).map { |phase| [i, phase] } }
([rust_rows] + all.values).each do |rows|
  raise 'coverage mismatch' unless rows.map { |r| r.values_at('case', 'phase') } == expected_order
end
raise 'Kujo runtime divergence' unless all.values.all? { |rows| rows == all.fetch('vm') }
differences = rust_rows.zip(all.fetch('vm')).map do |rust_row, kujo_row|
  fields = rust_row.keys.select { |key| rust_row[key] != kujo_row[key] }
  fields.empty? ? nil : {case: rust_row['case'], phase: rust_row['phase'], fields: fields}
end.compact
identities.each { |path, hash| raise "identity drift: #{path}" unless sha.call(path) == hash }
File.write(output + '/report.json', JSON.pretty_generate({
  scope: 'Full cell kinds, ordered neighbors, RNG and BFS/AStar paths; 10 diagnostic cases, 4 phases; no timing or production claim; Goose not executed',
  upstream: revision, sources: identities, cases: cases, commands: receipts,
  generated_sources: %w[oracle.rs probe.kujo].to_h { |p| [p, sha.call(output + '/' + p)] },
  oracle_binary_sha256: sha.call(output + '/oracle'), rust_rows: rust_rows,
  kujo_rows: all, differences: differences
}) + "\n")
puts JSON.generate({cases: cases.size, rows_per_engine: rust_rows.size, kujo_engines: all.keys, differences: differences})
