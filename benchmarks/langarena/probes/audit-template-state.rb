# Full-state diagnostic against an independent Ruby model; no competitor execution.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT' unless ARGV.size==3
binary,upstream,out=ARGV.map{|p|File.expand_path(p)}
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
pin='3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths=%w[rust/src/benchmarks/template.rs goose/src/template.goose]
files={binary=>Digest::SHA256.file(binary).hexdigest}
paths.each do |p|
 bytes,err,status=Open3.capture3('git','-C',upstream,'show',"#{pin}:#{p}")
 raise err unless status.success? && bytes==File.binread(upstream+'/'+p)
 File.binwrite(out+'/'+File.basename(p),bytes);files[upstream+'/'+p]=Digest::SHA256.hexdigest(bytes)
end
sources=%w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/35_template.kujo]
sources.each{|p|files[p]=Digest::SHA256.file(p).hexdigest}
files[__FILE__]=Digest::SHA256.file(__FILE__).hexdigest
first=%w[John Jane Bob Alice Charlie Diana Sarah Mike]
last=%w[Smith Johnson Brown Taylor Wilson Davis Miller Jones]
cities=['New York','Los Angeles','Chicago','Houston','Phoenix','San Francisco']
cases=[]
%w[Template::Regex Template::Parse].each do |name|
 [0,1,2,3,8,17,100].each do |count|
  text='<html><body><h1>{{TITLE}}</h1><p>Lorem {ipsum} dolor {sit} amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore {et} dolore magna aliqua. </p><table>'
  vars={'TITLE'=>'Template title'}
  count.times do |i|
   text+='<!-- {comment} -->' if i%3==0
   text+="<tr><td>{{ FIRST_NAME#{i} }}</td><td>{{LAST_NAME#{i}}}</td><td>{{  CITY#{i}  }}</td><td>{balance: #{i%100}}</td></tr>\n"
   vars["FIRST_NAME#{i}"]=first[i%first.size];vars["LAST_NAME#{i}"]=last[i%last.size];vars["CITY#{i}"]=cities[i%cities.size]
  end
  cases<<{label:"#{name}-prepare-#{count}",prepare:true,state:{'cfg'=>{'name'=>name,'count'=>count},'result'=>0,'text'=>text+'</table></body></html>','vars'=>vars}}
 end
 ['', '{{A}}', '{{ MISSING }}', '{{ A }}{{B}}', '{A}', '{{A', '{{{A}}}', '{{A}}}', "{{\nA\n}}", "é{{A}}雪", '{{EMPTY}}', 'prefix {{A}} suffix'].each_with_index do |text,i|
  cases<<{label:"#{name}-boundary-#{i}",prepare:false,state:{'cfg'=>{'name'=>name,'count'=>0},'result'=>4294967290,'text'=>text,'vars'=>{'A'=>'<&é>','B'=>'two','EMPTY'=>''}}}
 end
end
expected=[]
cases.each_with_index do |entry,index|
 state=Marshal.load(Marshal.dump(entry[:state]))
 3.times do |step|
  text=state['text'];vars=state['vars'];rendered=''
  if state['cfg']['name']=='Template::Regex'
   rendered=text.gsub(/\{\{(.*?)\}\}/){vars.fetch(Regexp.last_match(1).strip,'')}
  else
   i=0
   while i<text.length
    ending=text.index('}}',i+2) if text[i,2]=='{{'
    if text[i,2]=='{{' && ending
     rendered+=vars.fetch(text[(i+2)...ending].strip,'');i=ending+2
    else
     rendered+=text[i];i+=1
    end
   end
  end
  state['rendered']=rendered;state['result']=(state['result']+rendered.bytesize)&0xffffffff
  checksum=rendered.bytes.reduce(5381){|a,b|(a*33+b)&0xffffffff}
  expected<<{'case'=>index,'step'=>step,'state'=>Marshal.load(Marshal.dump(state)),'checksum'=>(state['result']+checksum)&0xffffffff}
 end
end
fixture=sources.map{|p|File.read(p)}.join("\n")+"\nlet cases := parse_json(read_file(#{JSON.generate(out+'/cases.json')}))\n"+<<'KUJO'
mut index := 0
for entry in cases {
    mut state := entry["state"]
    if entry["prepare"] { state = template_prepare(basic_state(state["cfg"])) }
    mut step := 0
    while step < 3 {
        state = template_run(state, step)
        print(to_json({"case": index, "step": step, "state": state, "checksum": template_checksum(state)}))
        step += 1
    }
    index += 1
}
KUJO
File.write(out+'/fixture.kujo',fixture);files[out+'/fixture.kujo']=Digest::SHA256.hexdigest(fixture)
File.write(out+'/expected.json',JSON.generate(expected));File.write(out+'/cases.json',JSON.generate(cases))
files[out+'/expected.json']=Digest::SHA256.file(out+'/expected.json').hexdigest
files[out+'/cases.json']=Digest::SHA256.file(out+'/cases.json').hexdigest
manifest={pin:pin,files:files,cases:cases.size,states_per_runtime:expected.size,scope:'Independent full-state model; comparator sources reviewed, not executed; no timing or native coverage claim',commands:[]}
%w[vm interpreter jit].each do |mode|
 files.each{|p,h|raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h}
 argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
 stdout,stderr,status=Open3.capture3(*argv)
 File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
 manifest[:commands]<<{mode:mode,argv:argv,exit:status.exitstatus,signal:status.termsig}
 File.write(out+'/manifest.json',JSON.pretty_generate(manifest))
 raise "#{mode} execution failed: #{stderr}" unless status.success?
 actual=stdout.lines.map{|l|JSON.parse(l)};raise "#{mode} full-state mismatch" unless actual==expected
end
files.each{|p,h|raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h}
puts "#{expected.size*3} complete template state/checksum observations matched"
