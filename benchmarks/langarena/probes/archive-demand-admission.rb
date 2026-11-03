# Repository-maintenance evidence packaging; not measured language behavior.
require 'json'
require 'digest'
require 'base64'
require 'zlib'
root = '.audit-evidence/native-performance-campaign-2026-11-02'
v2 = root + '/demand-admission-v2'
output = ARGV.fetch(0)
raise 'output exists' if File.exist?(output)
cases = JSON.parse(File.read('benchmarks/langarena/test.json'))
%w[vm jit].each do |mode|
  rows = JSON.parse(File.read(v2 + "/all50-#{mode}.json")).fetch('results')
  raise 'coverage/configuration' unless rows.map { |r| r['config'] } == cases
  failures = rows.reject { |r| r['status'] == 'passed' }
  expected = mode == 'vm' ? [] : %w[Etc::Words Hash::SHA256]
  raise 'failure membership' unless failures.map { |r| r['name'] } == expected
  rows.each do |r|
    raise 'truncated evidence' if r['stdout_truncated'] || r['stderr_truncated']
    if r['status'] == 'passed'
      raise 'checksum receipt' unless r['exitcode'] == 0 && r['stdout'].lines.map(&:strip).include?("#{r['name']}: OK #{r['config']['checksum']}")
    else
      raise 'missing nonzero outcome' unless r['exitcode'] == 4
    end
  end
end
repro = JSON.parse(File.read(v2 + '/failure-reproduction.json'))
raise 'reproduction coverage' unless repro['rows'].map { |r| [r['label'],r['mode'],r['name']] }.sort == %w[baseline candidate].product(%w[vm interpreter jit loop-only], %w[Etc::Words Hash::SHA256]).sort
repro['rows'].each do |r|
  fail_expected = r['mode'] == 'loop-only' || (r['label'] == 'candidate' && r['mode'] == 'jit')
  raise 'unexpected reproduction' unless r['exit'] == (fail_expected ? 4 : 0) && r['signal'].nil?
end
repro['files'].each { |p,h| raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest == h }
manifest = JSON.parse(File.read(v2 + '/manifest.json'))
manifest['source_hashes'].each do |p,h|
  snapshot = v2 + '/rejected-source/' + p
  actual = File.file?(snapshot) ? snapshot : p
  raise "snapshot drift #{p}" unless Digest::SHA256.file(actual).hexdigest == h
end
paths = Dir[root + '/demand-admission-v{1,2}/**/*'].select { |p| File.file?(p) && %w[.json .log .rb .kujo .rs .patch .out .err .txt].include?(File.extname(p)) }
paths += [__FILE__, 'benchmarks/langarena/DEMAND_ADMISSION_CANDIDATE_2026_11_03.md', 'benchmarks/langarena/test.json', 'benchmarks/langarena/port.kujo']
files = paths.sort.to_h do |p|
  bytes = File.binread(p)
  [p, {sha256: Digest::SHA256.hexdigest(bytes), base64: Base64.strict_encode64(bytes)}]
end
payload = {schema:1, disposition:'rejected-correctness', speed_claim:false, files:files}
Zlib::GzipWriter.open(output) { |g| g.write(JSON.generate(payload)) }
check = JSON.parse(Zlib::GzipReader.open(output, &:read))
raise 'archive count' unless check['files'].size == files.size
check['files'].each { |p,v| raise "corrupt #{p}" unless Digest::SHA256.hexdigest(Base64.strict_decode64(v['base64'])) == v['sha256'] }
puts "Verified 100 sweep outcomes, 16 failure reproductions, candidate source identities and #{files.size} archived files."
