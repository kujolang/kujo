require 'json'; require 'digest'; require 'open3'; require 'fileutils'
probe, directory = ARGV
raise 'usage: PROBE NEW_DIRECTORY' unless directory
raise 'new output directory required' if File.exist?(directory)
probe=File.realpath(probe);FileUtils.mkdir_p(directory);directory=File.expand_path(directory)
report={status:'running',probe_sha256:Digest::SHA256.file(probe).hexdigest,cases:[],limits:'Cold phase1 includes first call and JIT compilation when requested; warm phase2 excludes compilation. Counts are requests/requested bytes, not retained memory or speed.'}
save=->{File.write(directory+'/results.json',JSON.pretty_generate(report))}
[1,4,16].product([8,256],[200,2000]).each do |keys,width,reads|
  names=(0...keys).map{|i|"k#{i}".ljust(width,'x')}
  body=names.map{|s|"sum += data[#{s.to_json}]"}.join("\n")
  dict=names.map{|s|"#{s.to_json}: 7"}.join(', ')
  source=<<~KUJO
    func scan(data, reads) {
        mut sum := 0
        mut i := 0
        while i < reads { #{body}
            i += 1
        }
        return sum
    }
    let data := {#{dict}}
    env_set("KUJO_ALLOCATION_PHASE", "1")
    let cold := scan(data, 2)
    env_set("KUJO_ALLOCATION_PHASE", "2")
    let warm := scan(data, #{reads})
    env_set("KUJO_ALLOCATION_PHASE", "0")
    assert_equal(cold, #{keys*14})
    assert_equal(warm, #{keys*7*reads})
    assert_equal(data, {#{dict}})
    print("PASS")
  KUJO
  path=directory+"/#{keys}-#{width}-#{reads}.kujo";File.write(path,source)
  3.times do |round|
    %w[vm jit].each do |mode|
      out,err,status=Open3.capture3({'KUJO_ALLOCATION_PHASE'=>'0','DISABLE_FUNCTION_JIT'=>nil,'DEBUG_JIT_CALL'=>nil,'DEBUG_VM'=>nil},probe,mode,path,'count')
      raise err unless status.success? && out=="PASS\n"
      counters=err.lines.grep(/^COUNTERS /);raise 'counter membership' unless counters.size==1
      entries=JSON.parse(counters.first.delete_prefix('COUNTERS ')).fetch('counters').fetch('jit_native_function_entries')
      raise 'native coverage' unless entries==(mode=='jit' ? 2 : 0)
      phases=[1,2].to_h{|p|v=err.scan(/^PHASE #{p} requests=(\d+) bytes=(\d+)\s*$/);raise 'phase membership' unless v.size==1;[p,v.first.map(&:to_i)]}
      report[:cases]<<{keys:keys,width:width,reads:reads,round:round,mode:mode,source:source,phases:phases,native_entries:entries,stdout:out,stderr:err,exit:status.exitstatus}
      save.call
    end
  end
end
raise 'probe drift' unless Digest::SHA256.file(probe).hexdigest==report[:probe_sha256]
report[:status]='measured-review-pending';save.call
puts '72 spill allocation observations recorded'
