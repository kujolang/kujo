# Independent oracle: ordered key/value pairs, not linked-node mutation.
require 'json'
require 'digest'
raise 'usage: OUTPUT' unless ARGV.size == 1
out = File.expand_path(ARGV[0])
manifest = JSON.parse(File.read(out + '/manifest.json'))
raise 'configuration drift' unless manifest.values_at('configurations','seeds','iterations','requests') == [[[1,1],[2,5],[8,17]],[42,139967],2,1000]
manifest.fetch('files').each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
runs = JSON.parse(File.read(out + '/runs.json'))
raise 'runtime coverage' unless runs.map { |r| r['runtime'] } == %w[vm interpreter jit]
expected = []
case_id = 0
[[1,1],[2,5],[8,17]].each do |capacity, domain|
  [42,139967].each do |seed|
    rng = seed
    recent = []
    hits = misses = 0
    2.times do |iteration|
      1000.times do |step|
        rng = (rng * 3877 + 29573) % 139968
        key = 'item_' + (rng * domain / 139968).to_s
        found = recent.index { |pair| pair[0] == key }
        if found
          hits += 1
          recent.delete_at(found)
          value = "updated_#{iteration}"
        else
          misses += 1
          recent.pop if recent.size == capacity
          value = "new_#{iteration}"
        end
        recent.unshift([key,value])
        nodes = recent.each_with_index.map do |pair,i|
          [pair[0],pair[1],i.zero? ? nil : recent[i-1][0],recent[i+1] && recent[i+1][0]]
        end
        expected << [case_id,iteration,step+1,rng,recent.first[0],recent.last[0],hits,misses,nodes]
      end
      checksum = (((((5432 << 5) + hits) << 5) + misses) << 5) + recent.size
      expected << ['end',case_id,iteration,checksum & 0xffffffff]
    end
    case_id += 1
  end
end
runs.each do |run|
  raise 'runtime failed' unless run['exit'] == 0 && run['signal'].nil?
  mode = run.fetch('runtime')
  %w[out err].each do |ext|
    key = ext == 'out' ? 'stdout_sha256' : 'stderr_sha256'
    raise 'output drift' unless Digest::SHA256.file(out + '/' + mode + '.' + ext).hexdigest == run.fetch(key)
  end
  actual = File.readlines(out + '/' + mode + '.out').map { |line| JSON.parse(line) }
  raise "transition mismatch: #{mode}" unless actual == expected
end
result = {status: 'verified', transitions: 36000, final_checksums: 36,
          scope: 'Three positive capacities, two RNG seeds, two unchanged 1000-request iterations, all three Kujo runtimes; instrumented/original final states and snapshots checked. No executed Rust/Goose, timing, native participation or production equivalence claim.'}
File.write(out + '/verification.json', JSON.pretty_generate(result))
puts JSON.generate(result)
