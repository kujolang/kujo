# Synthetic verifier controls only; never performance evidence.
require 'json'; require 'digest'; require 'tmpdir'; require 'fileutils'; require 'open3'
sha, status = Open3.capture2('git', 'rev-parse', 'HEAD'); raise unless status.success?; sha = sha.strip
source_at = ->(p) { value, status = Open3.capture2('git', 'show', "#{sha}:#{p}"); raise p unless status.success?; value }
config_text = source_at.call('benchmarks/langarena/test.json')
config = JSON.parse(config_text).to_h { |r| [r['name'], r] }
parts = JSON.parse(source_at.call('benchmarks/langarena/build.kujo')[/let parts := (\[[^\n]+\])/,1])
port = parts.map { |p| source_at.call("benchmarks/langarena/src/#{p}.kujo") + "\n" }.join
first='            while i < cfg["iterations"] {'; last='            let actual := u32(checksum(state))'
port=port.sub(first,"            let comparison_started := time_ns()\n"+first).sub(last,"            print(\"@comparison_ns \" + str(time_ns() - comparison_started))\n"+last)
Dir.mktmpdir('kujo-store-verifier-') do |root|
  FileUtils.mkdir_p(root+'/timings')
  File.write(root+'/candidate-source.txt',sha)
  File.write(root+'/baseline-source.txt','cd5f2b207a8f907d6c263f5a2e405412965344ca')
  binaries=%w[baseline candidate].to_h { |v| [v,root+'/'+v+'-kujo'] }
  binaries.each do |v,p|
    File.write(p,"synthetic-#{v}")
    build=[{reason:'compiler-artifact',target:{name:'kujo',kind:['bin']},profile:{opt_level:'3',test:false}},{reason:'build-finished',success:true}]
    File.write(root+'/'+v+'-build.jsonl',build.map { |r| JSON.generate(r) }.join("\n"))
  end
  File.write(root+'/timings/timed-port.kujo',port)
  config_path='/synthetic/benchmarks/langarena/test.json'; collector='/synthetic/benchmarks/langarena/probes/compare-local-store-targets.rb'
  port_path=root+'/timings/timed-port.kujo'
  files=binaries.values.to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }.merge(config_path=>Digest::SHA256.hexdigest(config_text),collector=>Digest::SHA256.hexdigest(source_at.call('benchmarks/langarena/probes/compare-local-store-targets.rb')),port_path=>Digest::SHA256.hexdigest(port))
  names=%w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::TextRaytracer Matmul::T4 Matmul::T8 Matmul::T16 Base64::Encode Base64::Decode Etc::CacheSimulation Maze::Generator Maze::BFS Maze::AStar Compress::BWTEncode Compress::HuffEncode Compress::HuffDecode]
  cells=names.product(%w[vm jit]); rows=[]
  (-1...7).each do |round|
    cells.rotate([round,0].max).each do |name,runtime|
      index=cells.index([name,runtime])
      ((round+index).even? ? %w[baseline candidate] : %w[candidate baseline]).each do |variant|
        label=[round,index,variant].join('-'); stdout="@comparison_ns 100\n#{name}: OK #{config[name]['checksum']}\nSummary: 1, 1, 0\n"
        File.write(root+'/timings/'+label+'.out',stdout); File.write(root+'/timings/'+label+'.err','')
        argv=[binaries[variant],'run','--scheduler-no-timeout']; argv << '--jit' if runtime=='jit'; argv += [port_path,config_path,name]
        rows << {round:round,name:name,runtime:runtime,variant:variant,argv:argv,exit:0,stdout:stdout,stderr:'',kernel_ns:100,wall_seconds:0.1}
      end
    end
  end
  original=JSON.generate({status:'collected-review-pending',binaries:binaries,files:files,cells:cells,rows:rows})
  controls={valid:->(r){}, order:->(r){r['rows'][0],r['rows'][1]=r['rows'][1],r['rows'][0]}, missing:->(r){r['rows'].pop}, output:->(r){r['rows'][0]['stdout']='fake'}, timer:->(r){r['rows'][0]['kernel_ns']=0}, invocation:->(r){r['rows'][0]['argv'] << 'extra'}, binary:->(r){r['files'][binaries['candidate']]='wrong'}, config:->(r){r['files'][config_path]='wrong'}, incomplete:->(r){r['status']='running'}}
  controls.each do |name,mutate|
    r=JSON.parse(original); mutate.call(r); File.write(root+'/timings/report.json',JSON.generate(r))
    _, error, status=Open3.capture3('ruby',File.join(__dir__,'verify-local-store-targets.rb'),root,sha)
    raise "#{name}: #{error}" unless status.success? == (name==:valid)
  end
  File.write(root+'/timings/report.json',original)
  File.write(root+'/baseline-source.txt',sha)
  _, error, status=Open3.capture3('ruby',File.join(__dir__,'verify-local-store-targets.rb'),root,sha,sha,sha)
  raise "explicit baseline: #{error}" unless status.success?
  _, _, status=Open3.capture3('ruby',File.join(__dir__,'verify-local-store-targets.rb'),root,sha)
  raise 'unexpected baseline accepted by default' if status.success?
  puts JSON.generate({scope:'synthetic verifier controls',passed:controls.keys+[:explicit_baseline,:unexpected_baseline]})
end
