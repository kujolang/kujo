# Repository-maintenance oracle, not a benchmark or runtime implementation.
# Model the two queue policies, then verify the Kujo result in all engines.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'

arena, binary, output = ARGV.map { |p| File.expand_path(p) }
raise 'expected upstream, Kujo binary and new output directory' unless output
raise 'output exists' if File.exist?(output)
revision, status = Open3.capture2('git', '-C', arena, 'rev-parse', 'HEAD')
raise 'wrong upstream' unless status.success? && revision.strip == '3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths = %w[rust/src/benchmarks/graph_path.rs goose/src/graph_path.goose crystal/main.cr]
paths.each do |path|
  pinned, status = Open3.capture2('git', '-C', arena, 'show', "HEAD:#{path}")
  raise "modified pinned source: #{path}" unless status.success? && pinned == File.read(arena + '/' + path)
end
sources = paths.map { |p| arena + '/' + p } +
  %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/40_graph.kujo benchmarks/langarena/test.json] + [binary]
hashes = sources.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }

class PortHeap
  def initialize
    @items = []
    @size = 0
  end
  def push(vertex, priority)
    i = @size
    @items[i] = [vertex, priority]
    @size += 1
    while i > 0
      parent = (i - 1) / 2
      break if @items[parent][1] <= priority
      @items[i] = @items[parent]
      i = parent
    end
    @items[i] = [vertex, priority]
  end
  def pop
    return nil if @size == 0
    first = @items[0]
    @size -= 1
    if @size > 0
      last = @items[@size]
      i = 0
      loop do
        left, right, smallest = 2 * i + 1, 2 * i + 2, i
        smallest = left if left < @size && @items[left][1] < @items[smallest][1]
        smallest = right if right < @size && @items[right][1] < @items[smallest][1]
        break if smallest == i
        @items[i] = @items[smallest]
        i = smallest
      end
      @items[i] = last
    end
    first
  end
end

def astar(adj, port_order)
  target = adj.size - 1
  return [0, []] if target == 0
  scores = Array.new(adj.size, 2_147_483_647)
  best = scores.dup
  scores[0], best[0] = 0, target
  heap = port_order ? PortHeap.new : []
  add = lambda do |v, p|
    port_order ? heap.push(v, p) : heap.push([v, p])
  end
  add.call(0, target)
  trace = []
  loop do
    heap.sort_by! { |v, p| [p, v] } unless port_order
    item = port_order ? heap.pop : heap.shift
    break unless item
    current = item[0]
    trace << item
    return [scores[current], trace] if current == target
    adj[current].each do |neighbor|
      score = scores[current] + 1
      next unless score < scores[neighbor]
      scores[neighbor] = score
      priority = score + target - neighbor
      if priority < best[neighbor]
        best[neighbor] = priority
        add.call(neighbor, priority)
      end
    end
  end
  [-1, trace]
end

heap = PortHeap.new
4.times { |v| heap.push(v, v + 1) }
heap_order = Array.new(4) { heap.pop[0] }
raise 'unexpected model' unless heap_order == [0, 3, 1, 2]

# Fixed-seed undirected graphs; preserve insertion order and record full inputs.
random = Random.new(42)
differences = []
1000.times do |number|
  n = 5 + number % 8
  adj = Array.new(n) { [] }
  (1...n).each do |i|
    parent = random.rand(i)
    adj[i] << parent
    adj[parent] << i
  end
  n.times do |v|
    n.times do |u|
      next if u == v || random.rand(20) != 0
      adj[v] << u
      adj[u] << v
    end
  end
  port, comparator = astar(adj, true), astar(adj, false)
  if port != comparator
    differences << {case: number, adj: adj, port: port, comparator_model: comparator}
  end
  break if differences.any? { |row| row[:port][0] != row[:comparator_model][0] }
end
raise 'no trace discrepancy reproduced' if differences.empty?
# The official test graph retains the chain backbone. Do not infer an official
# checksum difference from the deliberately broader graph controls above.
config = JSON.parse(File.read('benchmarks/langarena/test.json')).find { |c| c['name'] == 'Graph::AStar' }
adj = Array.new(config.fetch('vertices')) { [] }
(1...adj.size).each { |i| adj[i] << i - 1; adj[i - 1] << i }
seed = 42
next_int = lambda do |maximum|
  seed = (seed * 3877 + 29573) % 139968
  (seed.to_f / 139968.0 * maximum).to_i
end
adj.size.times do |v|
  next_int.call(config.fetch('jumps')).times do
    u = v + next_int.call(config.fetch('jump_len')) - config.fetch('jump_len') / 2
    next unless u >= 0 && u < adj.size && u != v
    adj[v] << u
    adj[u] << v
  end
end
official = {config: config, adjacency: adj, port: astar(adj, true), comparator_model: astar(adj, false)}
FileUtils.mkdir_p(output)
script = File.read('benchmarks/langarena/src/00_common.kujo') + "\n" +
  File.read('benchmarks/langarena/src/40_graph.kujo') + "\n"
script += "mut heap := [[], 0]\nmut i := 0\nwhile i < 4 { heap = graph_heap_push(heap, i, i + 1) i += 1 }\nmut order := []\nwhile heap[1] > 0 { let pair := graph_heap_pop(heap) heap = pair[0] order = push(order, pair[1][0]) }\nassert_equal(order, #{heap_order.to_json})\n"
differences.each do |row|
  script += "assert_equal(graph_astar(#{row[:adj].to_json}), #{row[:port][0]})\n"
end
script += "reset_rng()\nlet prepared := graph_prepare(basic_state(#{config.to_json}))\nassert_equal(prepared[\"adj\"], #{adj.to_json})\nassert_equal(graph_astar(prepared[\"adj\"]), #{official[:port][0]})\n"
script += "print(\"graph-order-audit: OK\")\n"
File.write(output + '/contracts.kujo', script)
rows = %w[vm interpreter jit].map do |mode|
  argv = [binary, 'run']
  argv << "--#{mode}" unless mode == 'vm'
  argv << output + '/contracts.kujo'
  out, err, status = Open3.capture3(*argv)
  raise "#{mode}: #{err}" unless status.success? && out.strip == 'graph-order-audit: OK'
  {mode: mode, argv: argv, exit: status.exitstatus, stdout: out, stderr: err}
end
sources.each { |p| raise 'source drift' unless hashes[p] == Digest::SHA256.file(p).hexdigest }
File.write(output + '/report.json', JSON.pretty_generate({
  scope: 'Kujo heap/order reproduction in three engines; Rust/Goose policy source review plus Ruby model, not executed competitors or timing',
  sources: hashes, heap_order: heap_order, priority_order: [0, 1, 2, 3],
  differing_cases: differences, official_test: official, rows: rows
}) + "\n")
puts "Verified heap discrepancy and #{differences.size} graph trace differences; #{differences.count { |r| r[:port][0] != r[:comparator_model][0] }} result differences."
