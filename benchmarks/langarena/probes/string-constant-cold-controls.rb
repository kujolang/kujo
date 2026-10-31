# Cold and warm literal reads with unreachable constant-pool padding.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
probe, directory = ARGV
raise 'usage: PROBE NEW_DIRECTORY' unless directory
raise 'new directory required' if File.exist?(directory)
probe = File.realpath(probe)
FileUtils.mkdir_p(directory)
directory = File.expand_path(directory)
report = {probe_sha256:Digest::SHA256.file(probe).hexdigest, cases:[], status:'running', limits:'VM allocation requests only. Phase1 first call; phase2 second call. Markers and call overhead remain in both phases. Unreachable literals expose pool-size costs without executing padding.'}
save = -> { File.write(directory+'/results.json',JSON.pretty_generate(report)) }
[0,128,2048].product(%w[int string]).each do |width,kind|
  padding = (0...width).map { |n| kind == 'int' ? (n+100).to_s : ('unused-'+n.to_s+'x'*248).to_json }.join(',')
  source = <<~KUJO
    func literal(unused) {
        if unused { let padding := [#{padding}] return "unreachable" }
        return "live"
    }
    env_set("KUJO_ALLOCATION_PHASE", "1")
    let cold := literal(false)
    env_set("KUJO_ALLOCATION_PHASE", "2")
    let warm := literal(false)
    env_set("KUJO_ALLOCATION_PHASE", "0")
    assert_equal(cold, "live")
    assert_equal(warm, "live")
    print("PASS")
  KUJO
  path = directory+"/#{kind}-#{width}.kujo"
  File.write(path,source)
  3.times do |round|
    out,err,status = Open3.capture3({'KUJO_ALLOCATION_PHASE'=>'0','DEBUG_VM'=>nil,'DEBUG_JIT_CALL'=>nil},probe,'vm',path,'count')
    raise err unless status.success? && out == "PASS\n"
    phases = [1,2].to_h do |phase|
      rows = err.scan(/^PHASE #{phase} requests=(\d+) bytes=(\d+)\s*$/)
      raise 'phase membership' unless rows.size == 1
      [phase,rows.first.map(&:to_i)]
    end
    report[:cases] << {width:width,kind:kind,round:round,source:source,source_sha256:Digest::SHA256.hexdigest(source),phases:phases,stdout:out,stderr:err,exit:status.exitstatus}
    save.call
  end
end
raise 'probe changed' unless Digest::SHA256.file(probe).hexdigest == report[:probe_sha256]
report[:status]='measured-review-pending';save.call
puts '18 cold/warm allocation observations recorded'
