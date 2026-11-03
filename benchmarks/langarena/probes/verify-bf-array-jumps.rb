require 'json'
require 'digest'
raise 'usage: FOCUSED_EVIDENCE' unless ARGV.size==1
directory=ARGV[0]
manifest=JSON.parse(File.read(File.join(directory,'manifest.json')))
raise 'canonical gate' unless manifest['canonical_fixed']==true
manifest.fetch('files').each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
programs=JSON.parse(File.read(File.join(directory,'programs.json')))
oracle=JSON.parse(File.read(File.join(directory,'oracle.json')))
raise 'fixture coverage' unless programs.size==16 && oracle.size==16
runs=JSON.parse(File.read(File.join(directory,'runs.json')))
raise 'runtimes' unless runs.map { |r| r['runtime'] }.sort==%w[interpreter jit vm]
runs.each do |run|
  raise 'exit' unless run['exit']==0 && run['signal'].nil?
  %w[stdout stderr].each do |stream|
    suffix=stream=='stdout' ? 'out' : 'err'
    raise 'raw hash' unless Digest::SHA256.file(File.join(directory,run['runtime']+'.'+suffix)).hexdigest==run.fetch(stream+'_sha256')
  end
  rows=File.readlines(File.join(directory,run['runtime']+'.out')).map { |l| JSON.parse(l) }
  raise 'coverage' unless rows.size==16
  rows.each_with_index do |row,index|
    ref=oracle[index];prefix=ref.values_at('result','position','tape')
    raise 'case order' unless row[0]==index
    raise 'old state/count' unless row[1]==prefix+[ref['steps']+ref['taken']]
    raise 'corrected state/count' unless row[2]==prefix+[ref['steps']]
  end
  index=oracle.index { |ref| ref['taken']>0 }
  wrong=rows[index][2].dup;wrong[3]+=oracle[index]['taken']
  raise 'count negative accepted' if wrong==rows[index][2]
  raise 'negative state changed' unless wrong[0,3]==rows[index][2][0,3]
end
result={status:'verified',complete_states:96,count_negative_controls:3,canonical_fixed:true,
        baseline_steps:oracle.sum { |r| r['steps']+r['taken'] },corrected_steps:oracle.sum { |r| r['steps'] },
        scope:'Complete diagnostic state and Brainfuck command counts, not CPU instructions or timing'}
File.write(File.join(directory,'independent-verification.json'),JSON.pretty_generate(result));puts JSON.generate(result)
