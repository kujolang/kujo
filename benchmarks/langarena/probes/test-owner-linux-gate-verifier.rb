require 'tmpdir'
require 'fileutils'
require 'json'
require 'open3'
raise 'usage: EVIDENCE_DIRECTORY COMMIT' unless ARGV.size==2
source=File.expand_path(ARGV[0]);commit=ARGV[1]
verifier=File.expand_path('verify-owner-linux-gate.rb',__dir__)
verified=JSON.parse(File.read(source+'/verification.json'))
owner=verified.fetch('owner_tests').first
ignored=verified.fetch('ignored').first
names=verified.fetch('integration_sources').values.flat_map{|v|v.fetch('tests')}
raise 'fixture names' unless names.size>1
results=[]
Dir.mktmpdir('owner-linux-gate-controls-') do |temp|
 {'valid'=>nil,'owner'=>'owner membership','ignored'=>'ignored membership changed','integration'=>'pinned integration membership'}.each do |control,reason|
  dir=temp+'/'+control;FileUtils.mkdir_p(dir)
  Dir.children(source).each{|f|FileUtils.cp(source+'/'+f,dir+'/'+f) if File.file?(source+'/'+f) && !%w[artifact.zip verification.json].include?(f)}
  log=File.read(dir+'/loop-condition-tests.log')
  log=log.sub('test '+owner+' ... ok','test vm::code_owner_lifetime_tests::fabricated ... ok') if control=='owner'
  log=log.sub('test '+ignored+' ... ignored','test fabricated::ignored ... ignored') if control=='ignored'
  log=log.sub('test '+names[1]+' ... ok','test '+names[0]+' ... ok') if control=='integration'
  File.write(dir+'/loop-condition-tests.log',log)
  out,err,status=Open3.capture3('ruby',verifier,dir,commit)
  if reason
   raise "#{control}: #{out} #{err}" unless !status.success? && err.include?(reason) && !File.exist?(dir+'/verification.json')
  else
   raise out+err unless status.success?
  end
  results<<{control:control,accepted:status.success?}
 end
end
puts JSON.pretty_generate({scope:'Artifact verifier controls, no test execution',controls:results})
