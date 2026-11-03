# Valid CSV kernel state oracle. No random preparation, competitor execution or
# performance claim. All original parsing/checksum work remains unchanged.
require 'json'
require 'csv'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'existing output' if File.exist?(out)
revision='3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators=%w[rust/src/benchmarks/csv_parse.rs goose/src/csv_parse.goose]
comparators.each do |path|
  pinned,status=Open3.capture2('git','-C',arena,'show',revision+':'+path)
  raise 'upstream source drift' unless status.success? && pinned==File.read(arena+'/'+path)
end
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/10_basic.kujo','src/builtins.rs']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
port=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
kernel=File.read('benchmarks/langarena/src/10_basic.kujo')[/func csv_run\(.*?\n}\n/m]
raise 'kernel extraction' unless kernel && port.include?(kernel)
diagnostic=kernel.sub('func csv_run(', 'func csv_state_probe(')
needle="    return state\n}"
raise 'single return required' unless diagnostic.scan(needle).size==1
diagnostic=diagnostic.sub(needle,"    state[\"observed_rows\"] = rows\n    state[\"observed_points\"] = points\n"+needle)
inputs=[0,1,2,3,10,11,32,128].product(["\n","\r\n"]).map do |count,separator|
  rows=(0...count).map do |i|
    name=["point A\\n, \"#{i}\"","é😀 line\n#{i}","tab\tback\\slash#{i}"][i%3]
    [name,format('%.10f',(i%17-8)*0.125),nil,format('%.10f',(i%7-3)*0.5),
     "[#{i.even?},\n #{i}]",format('%.10f',(i%11)*0.25)]
  end
  text=rows.map{|row|CSV.generate_line(row,row_sep:separator)}.join
  expected=rows.map{|row|row.map{|field|field.nil? ? '' : field}}
  parsed=CSV.parse(text,row_sep:separator).map{|row|row.map{|field|field.nil? ? '' : field}}
  raise 'fixture roundtrip' unless parsed==expected
  {count:count,separator:separator,text:text,fields:expected}
end
source=port+"\n"+diagnostic+"\n"
expected=[]
inputs.each_with_index do |input,index|
  [0,4294967294].each_with_index do |seed,which|
    id=index*2+which
    state={'cfg'=>{'name'=>'CSV::Parse'},'result'=>seed,'text'=>input[:text]}
    pieces=JSON.generate(state).each_char.each_slice(1024).map{|chars|JSON.generate(chars.join)}
    source+="mut original#{id} := parse_json(join([#{pieces.join(',')}],\"\"))\nmut observed#{id} := original#{id}\nmut iteration#{id} := 0\nwhile iteration#{id} < 3 {\n"
    source+="original#{id} = csv_run(original#{id},iteration#{id})\nobserved#{id} = csv_state_probe(observed#{id},iteration#{id})\n"
    source+="assert_equal(original#{id}[\"result\"],observed#{id}[\"result\"])\nassert_equal(original#{id}[\"text\"],observed#{id}[\"text\"])\nmut coordinates := []\n"
    source+="for point in observed#{id}[\"observed_points\"] { coordinates = push(coordinates,[point.x,point.y,point.z]) }\n"
    source+="print(to_json([#{id},iteration#{id},original#{id}[\"result\"],csv_checksum(original#{id}),observed#{id}[\"observed_rows\"],coordinates,original#{id}[\"text\"]]))\niteration#{id} += 1\n}\n"
    3.times{|iteration|expected<<{id:id,input:index,seed:seed,iteration:iteration}}
  end
end
raise 'source limit' if source.bytesize>1048576
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/inputs.json',JSON.pretty_generate(inputs))
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
    input=inputs[e[:input]]
    points=input[:fields].map{|row|[row[1].to_f,row[5].to_f,row[3].to_f]}
    raise 'complete state mismatch' unless actual.size==7 && actual[0,2]==[e[:id],e[:iteration]] && actual[4]==input[:fields] && actual[5]==points && actual[6]==input[:text]
    contribution=points.empty? ? 0 : points.transpose.sum{|axis|checksum.call(format('%.7f',axis.reduce(0.0,:+)/axis.size))}
    result=(e[:seed]+(e[:iteration]+1)*contribution)&0xffffffff
    raise 'checksum' unless actual[2]==result && actual[3]==((result+checksum.call(input[:text]))&0xffffffff)
  end
  rows<<{runtime:mode,argv:argv,exit:status.exitstatus,observations:expected.size,
         stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,cases:expected,rows:rows,
  scope:'288 complete CSV field/point/checksum observations across three runtimes; valid crafted inputs, no random preparation, comparator execution, malformed-input equivalence, production or speed claim.'}))
puts 'Verified288 complete CSV state observations; structural ledger remains partial.'
