# Negative controls for off-host evidence review; does not run Kujo.
require 'tmpdir'
require 'fileutils'
require 'json'
require 'open3'
raise 'usage: ARTIFACT_DIRECTORY COMMIT GENERATED_PORT' unless ARGV.size==3
source,commit,port=ARGV.map{|x|x.dup}
verifier=File.expand_path('verify-jit-async-economics.rb',__dir__)
results=[]
Dir.mktmpdir('archived-async-controls-') do |temporary|
  %w[valid source-drift port-drift outcome invocation].each do |control|
    dir=temporary+'/'+control
    FileUtils.cp_r(source,dir)
    FileUtils.rm_f(dir+'/archived-verified.json')
    report=JSON.parse(File.read(dir+'/report.json'))
    report['files']['src/vm.rs']='0'*64 if control=='source-drift'
    report['files']['benchmarks/langarena/port.kujo']='0'*64 if control=='port-drift'
    report['rows'][0]['exit']=124 if control=='outcome'
    report['rows'].find{|r|r['runtime']=='jit'}['argv'][0]='/wrong/binary' if control=='invocation'
    File.write(dir+'/report.json',JSON.generate(report))
    out,err,status=Open3.capture3('ruby',verifier,dir,'reuse','archive',commit,port)
    reason={'source-drift'=>'archive input drift src/vm.rs','port-drift'=>'archive input drift benchmarks/langarena/port.kujo','outcome'=>'failed outcome','invocation'=>'invocation'}[control]
    if control=='valid'
      raise out+err unless status.success?
      result=JSON.parse(File.read(dir+'/archived-verified.json'))
      raise 'unbounded identity claim' unless result['status']=='archived-counters-verified' && result['input_identity_scope'].include?('executable bytes unavailable')
    else
      raise "#{control}: #{out} #{err}" unless !status.success? && err.include?(reason) && !File.exist?(dir+'/archived-verified.json')
    end
    results<<{control:control,accepted:status.success?}
  end
end
puts JSON.pretty_generate({scope:'Archive verifier controls; no new runtime execution',controls:results})
