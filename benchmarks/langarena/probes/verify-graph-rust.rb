# Execute unchanged pinned Rust A* methods on the retained graph controls.
# This is a correctness oracle, never a timing comparator.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'

arena, report_path, output = ARGV.map { |p| File.expand_path(p) }
raise 'expected upstream, graph audit report and new output directory' unless output
raise 'output exists' if File.exist?(output)
report = JSON.parse(File.read(report_path))
path = arena + '/rust/src/benchmarks/graph_path.rs'
source = File.read(path)
pinned, status = Open3.capture2('git', '-C', arena, 'show', '3c9f84ddaeec834c0631747cb789e8cfd602f993:rust/src/benchmarks/graph_path.rs')
raise 'source mismatch' unless status.success? && pinned == source && Digest::SHA256.hexdigest(source) == report.fetch('sources').fetch(path)
def function(source, signature)
  start = source.index(signature) or raise "missing #{signature}"
  opening = source.index('{', start)
  ending, depth = opening + 1, 1
  while depth > 0
    char = source[ending] or raise 'unterminated function'
    depth += 1 if char == '{'
    depth -= 1 if char == '}'
    ending += 1
  end
  source[start...ending]
end
node_start = source.index('#[derive(Copy, Clone, Eq, PartialEq)]') or raise 'missing Node'
node_end = source.index('pub struct GraphPathAStar', node_start) or raise 'missing AStar'
methods = ['fn heuristic(&self, v: i32, target: i32)', 'fn a_star_shortest_path(&self, start: i32, target: i32)'].map { |s| function(source, s) }
oracle = "// Extracted pinned LangArena methods; MIT, see LICENSE.upstream.\nuse std::cmp::{Ordering, Reverse};\nuse std::collections::BinaryHeap;\n"
oracle += source[node_start...node_end]
oracle += "struct Graph { vertices: i32, adj: Vec<Vec<i32>> }\nstruct GraphPathBenchmark { graph: Graph }\nstruct GraphPathAStar { base: GraphPathBenchmark }\nimpl GraphPathAStar { #{methods.join("\n")} }\nfn main() {\n"
cases = report.fetch('differing_cases').map { |r| [r.fetch('adj'), r.fetch('comparator_model')[0]] }
official = report.fetch('official_test')
cases << [official.fetch('adjacency'), official.fetch('comparator_model')[0]]
cases.each do |adj, expected|
  vectors = adj.map { |row| "vec!#{row.to_json}" }.join(',')
  oracle += "let graph = Graph { vertices: #{adj.size}, adj: vec![#{vectors}] }; let solver = GraphPathAStar { base: GraphPathBenchmark { graph } }; let actual = solver.a_star_shortest_path(0, #{adj.size - 1}); assert_eq!(actual, #{expected}); println!(\"{}\", actual);\n"
end
oracle += "}\n"
FileUtils.mkdir_p(output)
File.write(output + '/oracle.rs', oracle)
out, err, status = Open3.capture3('rustc', '--edition=2021', '-O', output + '/oracle.rs', '-o', output + '/oracle')
File.write(output + '/build.log', out + err)
raise 'oracle build failed' unless status.success?
out, err, status = Open3.capture3(output + '/oracle')
raise 'oracle mismatch' unless status.success? && out.lines.map(&:to_i) == cases.map(&:last)
version, status = Open3.capture2('rustc', '--version')
raise 'missing rustc identity' unless status.success?
File.write(output + '/report.json', JSON.pretty_generate({
  scope: 'Unchanged pinned Rust A* methods; four full adjacency controls, no timing; Goose not executed',
  rustc: version.strip, source_sha256: Digest::SHA256.hexdigest(source),
  input_report_sha256: Digest::SHA256.file(report_path).hexdigest,
  oracle_source_sha256: Digest::SHA256.hexdigest(oracle),
  binary_sha256: Digest::SHA256.file(output + '/oracle').hexdigest,
  cases: cases.map { |adj, expected| {adj: adj, expected: expected} },
  exit: 0, stdout: out, stderr: err
}) + "\n")
puts "Verified #{cases.size} pinned Rust A* results against comparator-policy model."
