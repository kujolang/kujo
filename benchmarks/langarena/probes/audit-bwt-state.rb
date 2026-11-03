# Rotation-table oracle deliberately differs from the port's rank-doubling algorithm.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT' unless ARGV.size == 3
binary, upstream, out = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
pin = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
sources = %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/50_bwt.kujo]
files = ([binary,File.expand_path(__FILE__)] + sources).to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
%w[rust/src/benchmarks/compress.rs goose/src/bwt.goose goose/src/compress_data.goose].each do |path|
  bytes, err, status = Open3.capture3('git','-C',upstream,'show',"#{pin}:#{path}")
  raise err unless status.success? && bytes == File.binread(upstream+'/'+path)
  files[upstream+'/'+path] = Digest::SHA256.hexdigest(bytes)
  File.write(out+'/'+File.basename(path),bytes)
end
inputs = [0,1,2,3,7,8,9,10,11,16,31,32,33,64].map do |size|
  {prepare:true,data:Array.new(size) { |i| 'ABRACADABRA'.bytes[i%11] }}
end
[[],[0],[255],[0,255,0],[255,0,255,0],[65]*17,[1,2]*9,(0..255).to_a,(0..255).to_a.reverse,Array.new(65) { |i| (i*73+19)%256 }].each do |data|
  inputs << {prepare:false,data:data}
end
cases = %w[Compress::BWTEncode Compress::BWTDecode].flat_map do |name|
  inputs.map { |entry| entry.merge(name:name) }
end
expected = cases.each_with_index.flat_map do |entry,index|
  data = entry[:data]
  order = (0...data.size).sort_by { |offset| [data.rotate(offset),offset] }
  transformed = order.map { |offset| data[(offset-1)%data.size] }
  original = order.index(0) || 0
  3.times.map do |step|
    result = (4294967290+(step+1)*data.size)&0xffffffff
    {'case'=>index,'step'=>step,'name'=>entry[:name],'data'=>data,'encoded'=>transformed,'original'=>original,
     'decoded'=>entry[:name].end_with?('Decode') ? data : [],'result'=>result,
     'checksum'=>entry[:name].end_with?('Decode') ? (result+100000)&0xffffffff : result}
  end
end
fixture = sources.map { |p| File.read(p) }.join("\n") + "\nlet cases := parse_json(read_file(#{JSON.generate(out+'/cases.json')}))\n" + <<'KUJO'
func audit_bytes(value) {
    mut result := []
    for item in value { result = push(result, item) }
    return result
}
mut index := 0
for entry in cases {
    mut state := basic_state({"name":entry["name"],"size":len(entry["data"])})
    if entry["prepare"] { state = bwt_prepare(state) }
    else {
        state["data"] = bytes(entry["data"])
        if entry["name"] == "Compress::BWTDecode" { state["encoded"] = bwt_encode(state["data"]) }
    }
    state["result"] = 4294967290
    let original_data := state["data"]
    mut step := 0
    while step < 3 {
        state = bwt_run(state, step)
        assert_equal(state["data"], original_data)
        mut decoded := []
        mut checksum := result_checksum(state)
        if entry["name"] == "Compress::BWTDecode" {
            decoded = audit_bytes(state["decoded"])
            checksum = compress_decode_checksum(state)
        }
        print(to_json({"case":index,"step":step,"name":entry["name"],"data":audit_bytes(state["data"]),"encoded":audit_bytes(state["encoded"][0]),"original":state["encoded"][1],"decoded":decoded,"result":state["result"],"checksum":checksum}))
        step += 1
    }
    index += 1
}
KUJO
{'fixture.kujo'=>fixture,'cases.json'=>JSON.generate(cases),'expected.json'=>JSON.generate(expected)}.each do |name,bytes|
  File.write(out+'/'+name,bytes);files[out+'/'+name]=Digest::SHA256.hexdigest(bytes)
end
manifest = {pin:pin,files:files,cases:cases.size,states_per_runtime:expected.size,commands:[],scope:'Complete bytes/index/decoded data and wrapper accumulators against independent rotation ordering; comparator sources pinned, not executed; no speed claim'}
%w[vm interpreter jit].each do |mode|
  files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
  argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
  manifest[:commands]<<{mode:mode,argv:argv,exit:status.exitstatus,signal:status.termsig}
  File.write(out+'/manifest.json',JSON.pretty_generate(manifest))
  raise "#{mode} failed: #{stderr}" unless status.success?
  actual=stdout.lines.map { |line| JSON.parse(line) }
  mismatch=actual.each_index.find { |i| actual[i]!=expected[i] }
  raise "#{mode} full-state mismatch at #{mismatch}: #{actual[mismatch]} expected #{expected[mismatch]}" if mismatch
  raise 'state count' unless actual.size==expected.size
end
files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
puts "#{expected.size*3} complete BWT wrapper states verified"
