# Full-state controls for the JSON kernels, not their random-data preparation or
# a timing comparison. Rust/Goose source identity is checked, not execution.
require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'existing output' if File.exist?(out)
revision='3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators=%w[json_generate json_parse_dom json_parse_mapping].flat_map{|n|["rust/src/benchmarks/#{n}.rs","goose/src/#{n}.goose"]}
comparators.each do |path|
  pinned,status=Open3.capture2('git','-C',arena,'show',revision+':'+path)
  raise 'upstream source drift' unless status.success? && pinned==File.read(arena+'/'+path)
end
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/10_basic.kujo']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
port=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel=File.read('benchmarks/langarena/src/10_basic.kujo')[/func json_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && port.include?(kernel)
diagnostic=kernel.sub('func json_run(', 'func json_state_probe(')
needle="    return state\n}"
raise 'return extraction' unless diagnostic.scan(needle).size==1
diagnostic=diagnostic.sub(needle,"    state[\"observed_parsed\"] = parsed\n    state[\"observed_data\"] = data\n"+needle)
documents=[1,2,3,9,10,11,32,128].map do |n|
  {'coordinates'=>(0...n).map do |i|
    {'x'=>(i%17-8)*0.125,'y'=>(i%11)*0.25,'z'=>(i%7-3)*0.5,
     'name'=>["plain #{i}","é😀 \"quoted\" #{i}","line\nslash\\tab\t#{i}"][i%3],
     'opts'=>{'1'=>[1,true],'extra'=>[-i,false]}}
  end,'info'=>'some info'}
end
source=port+"\n"+diagnostic+"\n"
expected=[]
serial=0
documents.each_with_index do |document,index|
  ['Json::Generate','Json::ParseDom','Json::ParseMapping'].each do |name|
    (name=='Json::Generate' ? [0] : [0,4294967294]).each do |seed|
      id=serial;serial+=1
      state={'cfg'=>{'name'=>name},'result'=>seed}
      state[name=='Json::Generate' ? 'data' : 'text']=name=='Json::Generate' ? document['coordinates'] : JSON.generate(document)
      # Keep fixture literals below the lexer limit without reducing input size.
      pieces=JSON.generate(state).each_char.each_slice(1024).map{|chars|JSON.generate(chars.join)}
      source+="mut original#{id} := parse_json(join([#{pieces.join(',')}],\"\"))\n"
      source+="mut observed#{id} := original#{id}\nmut iteration#{id} := 0\nwhile iteration#{id} < 3 {\n"
      source+="original#{id} = json_run(original#{id}, iteration#{id})\n"
      if name=='Json::Generate'
        source+="print(to_json([#{id}, iteration#{id}, original#{id}[\"result\"], original#{id}[\"text\"]]))\n"
      else
        source+="observed#{id} = json_state_probe(observed#{id}, iteration#{id})\nassert_equal(original#{id}[\"result\"],observed#{id}[\"result\"])\n"
        source+="assert_equal(original#{id}[\"text\"],observed#{id}[\"text\"])\nmut normalized := []\n"
        source+="for row in observed#{id}[\"observed_data\"] {\n"
        source+=name=='Json::ParseMapping' ? 'normalized = push(normalized,[row.x,row.y,row.z])' : 'normalized = push(normalized,[row["x"],row["y"],row["z"]])'
        source+="\n}\nprint(to_json([#{id},iteration#{id},original#{id}[\"result\"],observed#{id}[\"observed_parsed\"],normalized]))\n"
      end
      source+="iteration#{id} += 1\n}\n"
      3.times{|iteration|expected<<{id:id,document:index,name:name,seed:seed,iteration:iteration}}
    end
  end
end
raise 'source limit' if source.bytesize>1048576
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/inputs.json',JSON.pretty_generate(documents))
hashes[out+'/contracts.kujo']=Digest::SHA256.hexdigest(source)
hashes[out+'/inputs.json']=Digest::SHA256.file(out+'/inputs.json').hexdigest
checksum=lambda{|text|text.bytes.reduce(5381){|acc,byte|(acc*33+byte)&0xffffffff}}
rows=[]
%w[vm interpreter jit].each do |mode|
  argv=[binary,'run','--scheduler-timeout-ms','60000']
  argv<<'--'+mode unless mode=='vm'
  argv<<out+'/contracts.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout)
  File.write(out+'/'+mode+'.err',stderr)
  raise "runtime failure #{mode}: #{stderr}" unless status.success?
  raise 'coverage' unless stdout.lines.size==expected.size
  stdout.lines.zip(expected).each do |line,e|
    actual=JSON.parse(line)
    document=documents[e[:document]]
    raise 'case identity' unless actual[0,2]==[e[:id],e[:iteration]]
    # Decode generated text with the independent host parser, not Kujo itself.
    full_document=e[:name]=='Json::Generate' ? JSON.parse(actual[3]) : actual[3]
    raise 'complete document mismatch' unless full_document==document
    if e[:name]=='Json::Generate'
      result=e[:iteration]+1
      raise 'generation shape' unless actual.size==4
    else
      coordinates=document['coordinates']
      wanted=coordinates.map{|r|%w[x y z].map{|key|r[key]}}
      raise 'complete coordinate mismatch' unless actual.size==5 && actual[4]==wanted
      sums=%w[x y z].map{|key|coordinates.reduce(0.0){|sum,r|sum+r[key]}}
      contribution=sums.sum{|sum|checksum.call(format('%.7f',sum/coordinates.size))}
      result=(e[:seed]+(e[:iteration]+1)*contribution)&0xffffffff
    end
    raise 'accumulated checksum mismatch' unless actual[2]==result
  end
  rows<<{runtime:mode,argv:argv,exit:status.exitstatus,observations:expected.size,
         stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,cases:expected,rows:rows,
  scope:'360 full-state observations, three runtimes; crafted valid schema controls only. No random preparation, malformed/empty parsing, competitor execution, production equivalence or speed claim.'}))
puts 'Verified360 JSON full-state observations across three runtimes; all three ledger rows remain partial.'
