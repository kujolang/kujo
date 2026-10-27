# Acceptance gate for foreign-module callback reentry. The underlying runner
# remains available for characterization of runtimes that still have this cost.
require 'json'
require 'digest'
require 'rbconfig'

abort 'Expected MATCHING_PROBE MATCHING_BINARY NEW_OUTPUT_DIR [BASELINE_PROBE]' unless [3, 4].include?(ARGV.size)
root = File.expand_path(ARGV[2])
raise 'Output directory exists' if File.exist?(root)
driver = File.join(__dir__, 'recursive-helper-allocations.rb')
completed = system({'KUJO_RECURSIVE_CAPTURE_ROUTE' => 'callback'}, RbConfig.ruby, driver, *ARGV)
path = File.join(root, 'report.json')
report = File.file?(path) ? JSON.parse(File.read(path)) : {}
passed = completed && report['status'] == 'passed' &&
         report['flat_depth_one_setup'] && report['flat_depth_four_setup']
gate = {
  status: passed ? 'passed' : 'failed',
  runner_sha256: Digest::SHA256.file(__FILE__).hexdigest,
  driver_sha256: Digest::SHA256.file(driver).hexdigest,
  report_sha256: File.file?(path) ? Digest::SHA256.file(path).hexdigest : nil,
  flat_depth_one_setup: report['flat_depth_one_setup'],
  flat_depth_four_setup: report['flat_depth_four_setup']
}
File.write(File.join(root, 'foreign-gate.json'), JSON.pretty_generate(gate) + "\n") if Dir.exist?(root)
abort 'Foreign capture allocation gate failed; inspect report.json and foreign-gate.json' unless passed
puts 'Foreign callback capture setup is flat at depths one and four'
