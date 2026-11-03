# Phase observations from the existing unchanged runner; never a timing score.
require 'json'
raise 'usage: RESOURCE_DIRECTORY' unless ARGV.size == 1
root = ARGV.fetch(0)
names = %w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::TextRaytracer]
config = JSON.parse(File.read('benchmarks/langarena/test.json')).to_h { |r| [r.fetch('name'), r] }
rows = %w[baseline candidate].product(%w[vm jit], names).map do |variant, runtime, name|
  label = [variant, runtime, name.gsub('::', '-')].join('-')
  stdout = File.read("#{root}/#{label}.out")
  raise 'checksum' unless stdout.lines.include?("#{name}: OK #{config.fetch(name).fetch('checksum')}\n") && stdout.lines.last == "Summary: 1, 1, 0\n"
  checkpoints = stdout.lines.grep(/^@profile /).map do |line|
    match = /\A@profile ([a-z0-9-]+) ns=(\d+)\n\z/.match(line)
    raise 'invalid checkpoint' unless match
    [match[1], Integer(match[2])]
  end
  warmups = config.fetch(name).fetch('warmup_iterations', [(config.fetch(name).fetch('iterations') * 0.2).to_i, 1].max)
  expected = %w[create-start create-done prepare-done] + (1..warmups).map { |i| "warmup-#{i}" } + ['warmup-done'] + (1..config.fetch(name).fetch('iterations')).map { |i| "run-#{i}" }
  raise 'phase membership' unless checkpoints.map(&:first) == expected
  raise 'non-monotonic checkpoints' unless checkpoints.map(&:last).each_cons(2).all? { |a, b| b >= a }
  times = checkpoints.to_h
  measured = JSON.parse(File.read("#{root}/#{label}.measurements.json"))
  raise 'runtime outcome' unless measured['outcome'] == 'success'
  {
    variant: variant, runtime: runtime, name: name, checkpoints: checkpoints,
    create_ns: times.fetch('create-done') - times.fetch('create-start'),
    prepare_ns: times.fetch('prepare-done') - times.fetch('create-done'),
    warmup_ns: times.fetch('warmup-done') - times.fetch('prepare-done'),
    configured_iterations_ns: checkpoints.last.last - times.fetch('warmup-done'),
    runtime_measurements: measured
  }
end
puts JSON.pretty_generate({rows: rows, scope: 'Instrumented diagnostic spans include checkpoint overhead. JIT compilation is inclusive and overlaps execution phases; do not add spans or infer compilation from process-minus-kernel time. Not paired performance scores.'})
