require 'json'
require 'digest'
raise 'usage: ORIGINAL_REPOSITORY PRODUCTION_DIRECTORY' unless ARGV.size==2
root,out=ARGV.map { |p| File.expand_path(p) }
manifest=JSON.parse(File.read(out+'/manifest.json'))
Dir.chdir(root) do
  manifest.fetch('files').each { |path,hash| raise "identity drift #{path}" unless Digest::SHA256.file(path).hexdigest==hash }
end
state=JSON.parse(File.read(out+'/state.json'))
raise 'collector unfinished' unless state['status']=='outcomes-collected-not-yet-independently-verified' && state['finished_at'] && !state.key?('child_pid')
raise 'mode coverage' unless state['commands'].map { |c| c['mode'] }==%w[jit vm interpreter]
config=JSON.parse(File.read(root+'/benchmarks/langarena/run.json')).find { |c| c['name']=='Brainfuck::Array' }
rows=state['commands'].map do |command|
  mode=command['mode'];report=JSON.parse(File.read(out+'/'+mode+'.json'))
  raise 'driver outcome' unless command['exit']==4 && command['signal'].nil? && command['outcome']=='timeout'
  raise 'report scope' unless report['runtime']==mode && report['results'].size==1 && report['config']=='benchmarks/langarena/run.json'
  row=report['results'][0]
  raise 'case' unless row['name']=='Brainfuck::Array' && row['runtime']==mode && row['config']==config
  raise 'timeout' unless row['status']=='timeout' && row['exitcode']==-1 && row['timeout_ms']==3600000 && row['elapsed_ns']>=3_600_000_000_000
  raise 'raw output' unless File.read(out+'/'+mode+'.json.current.stdout')==row['stdout'] && File.read(out+'/'+mode+'.json.current.stderr')==row['stderr'] && !row['stdout_truncated'] && !row['stderr_truncated']
  raise 'completed score mixed with timeout' if row['stdout'].include?('Brainfuck::Array: OK') || row['stdout'].include?('Summary:') || row['stdout'].include?('@profile run-1 ')
  warmup=row['stdout'].scan(/^@profile warmup-done ns=(\d+)$/).flatten
  raise 'warmup coverage' unless warmup.size==1 && row['stdout'].scan(/^@profile warmup-1000 ns=\d+$/).size==1
  if mode!='interpreter'
    raise 'partial measurements' unless row.dig('measurements','status')=='unavailable'
  end
  {runtime:mode,status:'timeout',deadline_seconds:3600,elapsed_seconds:row['elapsed_ns']/1e9,warmup_done_seconds:warmup.first.to_i/1e9,checksum_completed:false}
end
files=Dir.glob(out+'/*').select { |p| File.file?(p) && !p.end_with?('/verified-timeouts.json') }.to_h { |p| [File.basename(p),Digest::SHA256.file(p).hexdigest] }
File.write(out+'/verified-timeouts.json',JSON.pretty_generate({status:'verified-incomplete-production',frozen_identities:manifest['files'].size,rows:rows,files:files,scope:'Three one-hour timeouts on loaded laptop; no completed checksum, speed score or native-coverage attribution'}))
puts JSON.pretty_generate(rows)
