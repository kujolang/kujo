require 'json'
require 'zlib'
require 'digest'
require 'open3'

root = File.expand_path('../../..', __dir__)
Dir.chdir(root)
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
bundle_path = 'benchmarks/langarena/repeated-array-fix-2026-10-31.json.gz'
bundle = Zlib::GzipReader.open(bundle_path) { |gz| JSON.parse(gz.read) }
raise 'runtime identity' unless bundle['runtime_commit'] == '1ed1866'
artifacts = bundle.fetch('artifacts').to_h { |a| [a.fetch('path'), a] }
raise 'duplicate artifact' unless artifacts.size == bundle['artifacts'].size
artifacts.each_value do |a|
  raise "digest #{a['path']}" unless Digest::SHA256.hexdigest(a['content']) == a['sha256']
end
read = ->(name) { JSON.parse(artifacts.fetch(name + '.json').fetch('content')) }
acceptance = read.call('acceptance')
raise 'unaccepted' unless acceptance['status'] == 'accepted'
acceptance['evidence'].each do |path, hash|
  raise "receipt #{path}" unless artifacts.fetch(File.basename(path))['sha256'] == hash
end
raise 'broad scope' unless read.call('broad-verified').values_at('library_passes', 'integration_passes', 'checksums') == [1307, 430, 150]
raise 'combined scope' unless acceptance.values_at('strict_allocation_pairs', 'reviewed_boundary_pairs', 'semantic_observations', 'boundary_contract_executions') == [3832, 72, 5856, 18]
%w[boundary rss].each do |kind|
  review = read.call(kind + '-review')
  raise 'unresolved review' unless review['status'] == 'reviewed' && review['unresolved_groups'] == []
  review['evidence'].each { |path, hash| raise 'review drift' unless artifacts.fetch(path)['sha256'] == hash }
end
artifacts.each do |path, artifact|
  next unless path.start_with?('accepted-source/')
  current = path.delete_prefix('accepted-source/')
  raise "current source #{current}" unless sha.call(current) == artifact['sha256']
end
_, err, status = Open3.capture3('git', 'diff', '--exit-code', '1ed1866', '--', 'src', 'tests', 'Cargo.toml', 'Cargo.lock')
raise 'runtime drift ' + err unless status.success?

test_names = %w[
  repeated_destinations_bound_reservation_without_losing_compact_storage
  reservation_union_covers_sequential_regions_and_varying_alias_endpoints
  smaller_reservations_do_not_admit_uncertain_or_invalid_updates
  capture_dense_capacity_projection_matches_reservation_boundaries
  capture_budget_admission_avoids_unrecyclable_local_name_copies
  shared_capture_cache_drops_stale_mutable_membership_after_host_edits
  small_capture_budget_ignores_public_owner_spare_capacity
  shared_capture_metadata_preserves_host_membership_and_reinserted_kinds
]
log = artifacts.fetch('library-tests.log')['content']
test_names.each { |name| raise "missing test #{name}" unless log.match?(/::#{Regexp.escape(name)} \.\.\. ok$/) }
files = %w[
  src/interpreter/environment.rs
  src/interpreter/array_batch_proof.rs
  src/interpreter/array_batch_publication.rs
  benchmarks/langarena/SECTIONS_3_4_REVIEW_2026_10_31.md
  benchmarks/langarena/REPEATED_ARRAY_FIX_2026_10_31.md
  benchmarks/langarena/CAPTURE_STORAGE_BUDGETS_2026_10_31.md
  benchmarks/langarena/SHARED_CAPTURE_METADATA_2026_10_28.md
  benchmarks/langarena/DENSE_ARRAY_REVIEW_2026_10_30.md
  benchmarks/langarena/CHUNKED_OWNER_PROTOTYPE_2026_10_26.md
]
result = {
  status: 'review-evidence-verified', runtime_commit: '1ed1866',
  scope: 'Source/evidence integrity and stated scoped coverage; not full section3/4 optimization completion.',
  bundle_sha256: sha.call(bundle_path), artifact_count: artifacts.size,
  checked_library_tests: test_names, source_and_report_hashes: files.to_h { |p| [p, sha.call(p)] },
  verifier_sha256: sha.call(__FILE__),
  remaining: ['capture membership traversal', 'fallback materialization attribution',
              'fresh snapshot and payload representation costs', 'bounded proof coverage']
}
puts JSON.pretty_generate(result)
