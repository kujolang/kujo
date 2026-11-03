# Observe every pixel without changing the renderer's arithmetic or scene.
# Diagnostic only: additional printing makes these runs unsuitable for timing.
require 'json'; require 'digest'; require 'fileutils'; require 'open3'
raise 'usage: BASELINE CANDIDATE NEW_OUTPUT' unless ARGV.size == 3
baseline, candidate, out = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
paths = %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/46_ray.kujo benchmarks/langarena/test.json]
renderer = File.read(paths[1]); needle = '            result += pixel'
raise 'observation anchor' unless renderer.scan(needle).size == 1
renderer = renderer.sub(needle, needle + "\n            print(\"@pixel \" + str(pixel))")
config = JSON.parse(File.read(paths[2])).find { |r| r['name'] == 'Etc::TextRaytracer' }
raise 'official shape changed' unless config.values_at('w','h','iterations','checksum') == [100,100,3,1438660]
cases = [[1,1],[7,5],[100,100]]
fixture = File.read(paths[0]) + "\n" + renderer + "\n"
cases.each do |w,h|
  fixture += <<~KUJO
    print("@frame #{w} #{h}")
    if true {
      let original := basic_state({"name":"Etc::TextRaytracer","w":#{w},"h":#{h}})
      let before := to_json(original)
      let result := ray_run(original,0)
      assert_equal(to_json(original),before)
      print("@result " + str(result["result"]))
    }
  KUJO
end
file = out + '/pixels.kujo'; File.write(file, fixture)
inputs = [baseline,candidate,file,File.expand_path(__FILE__)] + paths.map { |p| File.expand_path(p) }
identities = inputs.to_h { |p| [p, Digest::SHA256.file(p).hexdigest] }
rows = []; reference = nil
%w[baseline candidate].zip([baseline,candidate]).each do |variant,binary|
  %w[vm interpreter jit].each do |mode|
    argv=[binary,'run','--scheduler-timeout-ms','120000']; argv << "--#{mode}" unless mode=='vm'; argv << file
    stdout,stderr,status=Open3.capture3(*argv)
    File.write(out+"/#{variant}-#{mode}.out",stdout); File.write(out+"/#{variant}-#{mode}.err",stderr)
    raise "#{variant} #{mode}: #{stderr}" unless status.success?
    raise 'stderr' unless stderr.empty? || (mode=='jit' && stderr.match?(/\AJIT opt-in requested; unsupported bytecode regions will use the VM \([^\n]+\)\.\n\z/))
    lines=stdout.lines.map(&:strip); frames=[]
    cases.each do |w,h|
      raise 'frame order' unless lines.shift=="@frame #{w} #{h}"
      pixels=Array.new(w*h) { line=lines.shift; raise 'pixel record' unless line && line.match?(/\A@pixel \d+\z/); Integer(line.split.last) }
      raise 'character domain' unless pixels.all? { |p| [32,46,45,43,42,88,77].include?(p) }
      raise 'pixel sum' unless lines.shift=="@result #{pixels.sum}"
      frames << pixels
    end
    raise 'trailing output' unless lines.empty?
    raise 'official checksum' unless frames.last.sum*4==config['checksum']
    reference ||= frames
    raise 'pixel difference' unless frames==reference
    rows << {variant:variant,runtime:mode,argv:argv,exit:status.exitstatus,pixels:frames.sum(&:size)}
  end
end
mutant=reference.last.dup; index=mutant.index { |p| p!=mutant.first }; raise 'missing distinct-pixel control' unless index
mutant[0],mutant[index]=mutant[index],mutant[0]
raise 'checksum-only acceptance' unless mutant.sum==reference.last.sum && mutant!=reference.last
identities.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
report={status:'exact-pixels-and-snapshots',files:identities,rows:rows,negative_control:'Swapping distinct pixels preserves checksum and is rejected by complete pixel equality.',scope:'Unchanged Kujo scene/arithmetic with per-pixel observation; all three runtimes against frozen baseline. No timing, executed Rust/Goose equivalence or production claim.'}
File.write(out+'/verification.json',JSON.pretty_generate(report)); puts JSON.generate(report.reject { |k,_| k==:files })
