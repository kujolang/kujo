require 'json'
require 'tmpdir'
require 'open3'
require 'fileutils'
candidate, error, status = Open3.capture3('git','rev-parse','HEAD')
raise error unless status.success?
candidate = candidate.strip
results = []
Dir.mktmpdir('owner-elapsed-verifier') do |directory|
  %w[baseline candidate].each do |variant|
    path = directory + '/' + variant + '-kujo'
    File.write(path,"#!/bin/sh\nprintf 'async-worker-economics: OK\\n'\n")
    File.chmod(0755,path)
    revision = variant == 'baseline' ? 'cd5f2b207a8f907d6c263f5a2e405412965344ca' : candidate
    File.write(directory + '/' + variant + '-source.txt',revision)
    build = [{reason:'compiler-artifact',target:{name:'kujo',kind:['bin']},profile:{opt_level:'3',test:false},executable:path},{reason:'build-finished',success:true}]
    File.write(directory + '/' + variant + '-build.jsonl',build.map { |row| JSON.generate(row) }.join("\n") + "\n")
  end
  out, err, status = Open3.capture3('ruby',File.join(__dir__,'compare-owner-elapsed.rb'),directory+'/baseline-kujo',directory+'/candidate-kujo',directory+'/samples')
  raise err unless status.success?
  File.write(directory+'/comparisons.json',out)
  File.write(directory+'/binary-recheck.txt',"elapsed-evidence/baseline-kujo: OK\nelapsed-evidence/candidate-kujo: OK\n")
  pristine = Dir.glob(directory+'/**/*').select { |p| File.file?(p) }.to_h { |p| [p,File.binread(p)] }
  %w[valid nonalternating duplicate-sample bad-ratio failed-sample instrumented altered-binary wrong-raw].each do |mode|
    pristine.each { |path,bytes| File.binwrite(path,bytes) }
    report = JSON.parse(File.read(directory+'/samples/report.json'))
    if mode == 'nonalternating'
      report['rows'] = report['rows'].each_slice(2).flat_map { |pair| pair.sort_by { |row| row['variant'] } }
    end
    report['rows'][1] = report['rows'][0] if mode == 'duplicate-sample'
    report['comparisons'][0]['median_speedup'] += 1 if mode == 'bad-ratio'
    report['rows'][0]['exit'] = 1 if mode == 'failed-sample'
    report['rows'][0]['argv'] << '--measurements' if mode == 'instrumented'
    File.write(directory+'/candidate-kujo','altered') if mode == 'altered-binary'
    if mode == 'wrong-raw'
      row = report['rows'][0]
      label = row.values_at('round','runtime','mode','workers','variant').join('-')
      File.write(directory+'/samples/'+label+'.out','incorrect')
    end
    File.write(directory+'/samples/report.json',JSON.generate(report))
    out, err, status = Open3.capture3('ruby',File.join(__dir__,'verify-owner-elapsed.rb'),directory,candidate)
    raise "unexpected #{mode}: #{out} #{err}" unless status.success? == (mode == 'valid')
    results << {control:mode,accepted:status.success?}
  end
end
puts JSON.pretty_generate(results)
