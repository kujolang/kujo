require 'tmpdir'
require 'fileutils'
require 'json'
require 'open3'
raise 'usage: ARTIFACT_DIRECTORY COMMIT' unless ARGV.size==2
source=File.expand_path(ARGV[0]);commit=ARGV[1]
verifier=File.expand_path('verify-owner-all150.rb',__dir__)
controls={'valid'=>nil,'duplicate'=>'checksum membership','checksum'=>'checksum membership','runtime'=>'runtime coverage','source'=>'source coverage','binary'=>'binary identity','build'=>'build outcome'}
results=[]
Dir.mktmpdir('owner-all150-controls-') do |temp|
 controls.each do |name,reason|
  dir=temp+'/'+name;FileUtils.mkdir_p(dir)
  Dir.children(source).each{|f|FileUtils.cp(source+'/'+f,dir+'/'+f) if File.file?(source+'/'+f) && !%w[kujo artifact.zip verification.json].include?(f)}
  File.symlink(source+'/kujo',dir+'/kujo')
  log=File.read(dir+'/checksums.log')
  checksum_lines=log.lines.grep(/: OK \d+$/)
  raise 'fixture checksums' unless checksum_lines.size==150
  log=log.sub(checksum_lines[1],checksum_lines[0]) if name=='duplicate'
  log=log.sub(checksum_lines[0],checksum_lines[0].sub(/\d+$/, '9999999999')) if name=='checksum'
  log=log.sub('Runtime: jit','Runtime: interpreter') if name=='runtime'
  File.write(dir+'/checksums.log',log)
  File.write(dir+'/source-sha256.txt',File.readlines(dir+'/source-sha256.txt').drop(1).join) if name=='source'
  if name=='binary'
   File.unlink(dir+'/kujo');File.write(dir+'/kujo','changed executable bytes')
  end
  if name=='build'
   messages=File.readlines(dir+'/build.jsonl').map{|l|JSON.parse(l)}
   messages.find{|m|m['reason']=='build-finished'}['success']=false
   File.write(dir+'/build.jsonl',messages.map{|m|JSON.generate(m)}.join("\n")+"\n")
  end
  out,err,status=Open3.capture3('ruby',verifier,dir,commit)
  if reason
   raise "#{name}: #{out} #{err}" unless !status.success? && err.include?(reason) && !File.exist?(dir+'/verification.json')
  else
   raise out+err unless status.success?
  end
  results<<{control:name,accepted:status.success?}
 end
end
puts JSON.pretty_generate({scope:'Artifact verifier controls, no benchmark execution',controls:results})
