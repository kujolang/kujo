# Measures fresh literal append and bound-value append without changing semantics.
require 'json'; require 'digest'; require 'open3'; require 'fileutils'
probe, directory = ARGV
raise 'usage: PROBE NEW_DIRECTORY' unless directory
raise 'new directory required' if File.exist?(directory)
probe=File.realpath(probe); FileUtils.mkdir_p(directory); directory=File.expand_path(directory)
report={probe_sha256:Digest::SHA256.file(probe).hexdigest,cases:[],status:'running'}
save=-> {File.write(directory+'/results.json',JSON.pretty_generate(report))}
[8,256].product([200,2000],%w[literal bound]).each do |width,reads,form|
  key='a'*width
  expression=form=='literal' ? key.to_json : 'key'
  source=<<~KUJO
    func scan(key, reads) {
        mut sum := 0
        mut i := 0
        while i < reads {
            mut value := #{expression}
            value += "!"
            sum += len(value)
            i += 1
        }
        return sum
    }
    let key := #{key.to_json}
    assert_equal(scan(key, 2), #{2*(width+1)})
    env_set("KUJO_ALLOCATION_PHASE", "1")
    let result := scan(key, #{reads})
    env_set("KUJO_ALLOCATION_PHASE", "0")
    assert_equal(result, #{reads*(width+1)})
    assert_equal(key, #{key.to_json})
    print("PASS")
  KUJO
  path=directory+"/#{form}-#{width}-#{reads}.kujo"; File.write(path,source)
  3.times do |round|
    %w[vm jit].each do |mode|
      out,err,status=Open3.capture3({'KUJO_ALLOCATION_PHASE'=>'0','DISABLE_FUNCTION_JIT'=>nil,'DEBUG_VM'=>nil,'DEBUG_JIT_CALL'=>nil},probe,mode,path,'count')
      raise err unless status.success? && out=="PASS\n"
      phase=err.scan(/^PHASE 1 requests=(\d+) bytes=(\d+)\s*$/)
      raise 'phase membership' unless phase.size==1
      counters=err.lines.grep(/^COUNTERS /)
      raise "counter membership" unless counters.size==1
      entries=JSON.parse(counters.first.delete_prefix("COUNTERS "))["counters"].fetch("jit_native_function_entries")
      raise "native coverage" unless entries==(mode=="jit" ? 2 : 0)
      report[:cases]<<{width:width,reads:reads,form:form,round:round,mode:mode,source:source,phase:phase.first.map(&:to_i),stdout:out,stderr:err,exit:status.exitstatus}
      save.call
    end
  end
end
raise 'probe drift' unless Digest::SHA256.file(probe).hexdigest==report[:probe_sha256]
report[:status]='measured-review-pending';save.call
puts '48 append allocation observations recorded'
