# Repository-maintenance generator; the executed diagnostics are Kujo source.
require 'fileutils'
root = File.expand_path(ARGV.fetch(0))
arities = [0, 1, 8, 32, 128]
paths = arities.to_h { |arity| [arity, File.join(root, "call-slots-#{arity}.kujo")] }
abort 'Refusing to overwrite existing diagnostic sources' if paths.values.any? { |path| File.exist?(path) }
FileUtils.mkdir_p(root)
arities.each do |arity|
  calls = arity <= 1 ? 200000 : 20000
  result = arity.zero? ? '7' : "p#{arity - 1}"
  expected = calls * (arity.zero? ? 7 : arity)
  params = (0...arity).map { |index| "p#{index}" }.join(',')
  args = (1..arity).to_a.join(',')
  source = "func leaf(#{params}) { return #{result} }\nmut total := 0 mut i := 0\nenv_set(\"KUJO_ALLOCATION_PHASE\", \"4\")\nwhile i < #{calls} { total += leaf(#{args}) i += 1 }\nenv_set(\"KUJO_ALLOCATION_PHASE\", \"5\")\nassert_equal(total, #{expected})\nprint(\"CALL SLOTS PASS\")\n"
  File.write(paths.fetch(arity), source)
end
puts "Generated #{arities.length} parameter-slot diagnostic sources"
