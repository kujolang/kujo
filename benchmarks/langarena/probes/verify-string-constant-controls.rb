require 'json'
require 'digest'
require 'zlib'
path = ARGV.fetch(0)
bundle = JSON.parse(Zlib::GzipReader.open(path, &:read))
bundle.fetch('artifacts').each_value do |entry|
  raise 'artifact digest' unless Digest::SHA256.hexdigest(entry.fetch('content')) == entry.fetch('sha256')
end
r = JSON.parse(bundle.fetch('artifacts').fetch('controls/results.json').fetch('content'))
raise 'incomplete' unless r['status'] == 'measured-review-pending'
expected = [8,256].product([200,2000], %w[literal bound])
raise 'fixture membership' unless r['fixtures'].map { |x| x.values_at('width','reads','form') }.sort == expected.sort
raise 'semantic membership' unless r['semantics'].map { |x| x.values_at('width','reads','form','mode') }.sort == expected.flat_map { |x| %w[vm interpreter jit].map { |m| x+[m] } }.sort
raise 'allocation membership' unless r['allocations'].map { |x| x.values_at('width','reads','form','mode','round') }.sort == expected.flat_map { |x| %w[vm jit].flat_map { |m| (0..2).map { |n| x+[m,n] } } }.sort
(r['semantics']+r['allocations']).each do |x|
  raise 'execution failed' unless x['result']['exit'] == 0 && x['result']['stdout'] == "PASS\n"
end
r['allocations'].each do |x|
  raise 'native coverage' unless x['native_entries'] == (x['mode'] == 'jit' ? 2 : 0)
  phase = x['result']['stderr'].scan(/^PHASE 1 requests=(\d+) bytes=(\d+)\s*$/)
  raise 'raw count mismatch' unless phase == [[x['requests'].to_s,x['bytes'].to_s]]
end
r['allocations'].group_by { |x| x.values_at('mode','width','reads') }.each do |key, rows|
  literal = rows.select { |x| x['form'] == 'literal' }.map { |x| x.values_at('requests','bytes') }.uniq
  bound = rows.select { |x| x['form'] == 'bound' }.map { |x| x.values_at('requests','bytes') }.uniq
  raise 'unstable counts' unless literal.size == 1 && bound.size == 1
  mode,width,reads = key
  delta = literal.first.zip(bound.first).map { |a,b| a-b }
  raise 'unexpected measured delta' unless delta == (mode == 'vm' ? [2*reads,(width+40)*reads] : [0,0])
end
puts 'Verified 24 semantic runs and 48 allocation observations; exact membership, raw counts and repeated slopes pass.'
