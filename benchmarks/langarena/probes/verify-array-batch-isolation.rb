require 'json'
require 'digest'
abort 'Expected OUTPUT_DIRECTORY' unless ARGV.size == 1
o = File.expand_path(ARGV[0])
r = JSON.parse(File.read(o + '/report.json'))
raise 'Incomplete' unless r['status'] == 'characterized'
sha = ->(p) { Digest::SHA256.file(p).hexdigest }
probe = r.fetch('rows').first.fetch('argv')[0]
binary = r.fetch('semantics').first.fetch('argv')[0]
runner = File.expand_path('array-batch-isolation.rb', __dir__)
identities = r['identities'].to_h { |path, hash| [File.expand_path(path), hash] }
raise 'Identity set' unless r['identities'].size == 3 &&
                           identities == [probe, binary, runner].to_h { |p| [p, sha.call(p)] }
keys = %w[payload writes width snapshot calls mode]
cases = %w[array mixed struct].product([3, 8, 64], [64, 2048], %w[once fresh], [1, 2, 16, 32])
raise 'Allocation coverage' unless r['rows'].map { |x| x.values_at(*keys) }.sort == cases.product(%w[vm jit]).map { |a, b| a + [b] }.sort
raise 'Semantic coverage' unless r['semantics'].map { |x| x.values_at(*keys) }.sort == cases.product(%w[vm interpreter jit]).map { |a, b| a + [b] }.sort
r['sources'].each { |h, source| raise 'Embedded source drift' unless Digest::SHA256.hexdigest(source) == h }
(r['rows'] + r['semantics']).each do |x|
  main = o + '/' + x.values_at(*keys.first(5)).join('-') + '/main.kujo'
  owner = File.dirname(main) + '/owner.kujo'
  raise 'Fixture drift' unless sha.call(main) == x['source_sha256'] && sha.call(owner) == x['module_sha256']
  [x['source_sha256'], x['module_sha256']].each { |h| raise 'Missing source' unless r['sources'].key?(h) }
  raise 'Nonzero exit' unless x['exit'] == 0
  if x.key?('requests')
    raise 'Allocation command' unless x['argv'] == [probe, x['mode'], main]
    raise 'Invalid counts' unless %w[requests bytes].all? { |k| x[k].is_a?(Integer) && x[k] >= 0 }
  else
    args = [binary, 'run', main, '--untrusted', '--scheduler-no-timeout']
    args << '--' + x['mode'] unless x['mode'] == 'vm'
    raise 'Semantic command' unless x['argv'] == args
  end
end
slopes = r['rows'].group_by { |x| x.values_at(*keys.first(4), 'mode') }.flat_map do |group, rows|
  rows.sort_by { |x| x['calls'] }.each_slice(2).map do |a, b|
    {group: group, calls: [a['calls'], b['calls']], requests: (b['requests'] - a['requests']).fdiv(b['calls'] - a['calls']), bytes: (b['bytes'] - a['bytes']).fdiv(b['calls'] - a['calls'])}
  end
end
penalties = slopes.select { |s| s[:group][2] == 2048 }.map do |large|
  group = large[:group].dup
  group[2] = 64
  small = slopes.find { |s| s[:group] == group && s[:calls] == large[:calls] }
  raise 'Missing small width' unless small
  {group: large[:group], calls: large[:calls], small_bytes_per_call: small[:bytes], large_bytes_per_call: large[:bytes], byte_width_penalty_per_call: large[:bytes] - small[:bytes]}
end
agreement = slopes.select { |s| s[:group][-1] == 'vm' }.all? do |vm|
  group = vm[:group].dup
  group[-1] = 'jit'
  jit = slopes.find { |s| s[:group] == group && s[:calls] == vm[:calls] }
  raise 'Missing JIT slope' unless jit
  %i[requests bytes].all? { |k| vm[k] == jit[k] }
end
v = {status: 'verified', allocation_observations: r['rows'].size, semantic_checks: r['semantics'].size, slopes: slopes, width_penalties: penalties, vm_jit_slopes_agree: agreement, report_sha256: sha.call(o + '/report.json'), verifier_sha256: sha.call(__FILE__), limits: 'Allocation characterization only. Fresh snapshots require isolation; no speed or fix acceptance.'}
File.write(o + '/verified.json', JSON.pretty_generate(v) + "\n")
puts JSON.generate(v.reject { |k, _| %i[slopes width_penalties].include?(k) })
