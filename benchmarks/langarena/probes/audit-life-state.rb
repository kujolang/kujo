# Full-grid and neighbor-link correctness controls, not a timing harness.
require 'json'
require 'digest'
require 'open3'
raise 'usage: UPSTREAM FROZEN_KUJO NEW_OUTPUT' unless ARGV.size==3
arena,binary,out=ARGV.map{|p|File.expand_path(p)}
raise 'existing output' if File.exist?(out)
revision='3c9f84ddaeec834c0631747cb789e8cfd602f993'
comparators=%w[rust/src/benchmarks/game_of_life.rs goose/src/game_of_life.goose]
comparators.each do |path|
  pinned,status=Open3.capture2('git','-C',arena,'show',revision+':'+path)
  raise 'upstream drift' unless status.success? && pinned==File.read(arena+'/'+path)
end
files=[binary,__FILE__,'benchmarks/langarena/port.kujo','benchmarks/langarena/src/45_life.kujo']+comparators.map{|p|arena+'/'+p}
hashes=files.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
source=File.read('benchmarks/langarena/port.kujo').split('func profile_checkpoint(').first
raise 'source extraction' unless source.include?(File.read('benchmarks/langarena/src/45_life.kujo'))
source+=<<~KUJO
  func life_state_observe(state) {
      mut result := []
      for cell in state["cells"] { result = push(result,[cell.alive,cell.next,cell.neighbors]) }
      return result
  }
KUJO
dimensions=[[1,1],[1,3],[3,1],[2,2],[2,3],[3,2],[5,5],[7,4],[20,20]]
cases=[]
dimensions.each do |w,h|
  patterns={
    'empty'=>[], 'full'=>(0...w*h).to_a,
    'checker'=>(0...w*h).select{|i|(i%w+i/w).even?},
    'edge_glider'=>[[1,0],[2,1],[0,2],[1,2],[2,2]].map{|x,y|((y+h-2)%h)*w+(x+w-2)%w}.uniq,
    'block'=>[[0,0],[1,0],[0,1],[1,1]].map{|x,y|(y%h)*w+x%w}.uniq
  }
  patterns.each do |pattern,alive|
    id=cases.size
    cases<<{id:id,w:w,h:h,pattern:pattern,alive:alive}
    source+="mut state#{id} := life_create({\"w\":#{w},\"h\":#{h}})\n"
    source+="for index in #{JSON.generate(alive)} { state#{id}[\"cells\"][index].alive = true }\n"
    source+="let snapshot#{id} := state#{id}\nlet initial#{id} := life_state_observe(state#{id})\n"
    source+="print(to_json([#{id},0,life_checksum(state#{id}),initial#{id}]))\nmut generation#{id} := 1\n"
    source+="while generation#{id} <= 3 {\nstate#{id} = life_run(state#{id},generation#{id}-1)\n"
    source+="assert_equal(life_state_observe(snapshot#{id}),initial#{id})\nprint(to_json([#{id},generation#{id},life_checksum(state#{id}),life_state_observe(state#{id})]))\ngeneration#{id} += 1\n}\n"
  end
end
raise 'fixture coverage/limit' unless cases.size==45 && source.bytesize<=1048576
Dir.mkdir(out)
File.write(out+'/contracts.kujo',source)
File.write(out+'/cases.json',JSON.pretty_generate(cases))
hashes[out+'/contracts.kujo']=Digest::SHA256.hexdigest(source)
hashes[out+'/cases.json']=Digest::SHA256.file(out+'/cases.json').hexdigest
rows=[]
offsets=(-1..1).to_a.product((-1..1).to_a).reject{|dy,dx|dy==0 && dx==0}
%w[vm interpreter jit].each do |mode|
  argv=[binary,'run','--scheduler-timeout-ms','60000']
  argv<<'--'+mode unless mode=='vm'
  argv<<out+'/contracts.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout)
  File.write(out+'/'+mode+'.err',stderr)
  raise "runtime failure #{mode}: #{stderr}" unless status.success?
  lines=stdout.lines
  raise 'observation coverage' unless lines.size==180
  cases.each do |c|
    w,h=c.values_at(:w,:h)
    neighbors=(0...w*h).map{|i|offsets.map{|dy,dx|((i/w+dy)%h)*w+(i%w+dx)%w}}
    alive=(0...w*h).map{|i|c[:alive].include?(i)}
    4.times do |generation|
      if generation>0
        alive=neighbors.each_with_index.map{|links,i|count=links.count{|j|alive[j]};count==3 || (alive[i] && count==2)}
      end
      state=alive.each_with_index.map{|value,i|[value,generation==0 ? false : value,neighbors[i]]}
      hash=alive.reduce(2166136261){|value,cell|((value^(cell ? 1 : 0))*16777619)&0xffffffff}
      checksum=(hash+alive.count(true))&0xffffffff
      expected=[c[:id],generation,checksum,state]
      actual=JSON.parse(lines[c[:id]*4+generation])
      raise "complete state mismatch #{mode}/#{c[:id]}/#{generation}" unless actual==expected
    end
  end
  rows<<{runtime:mode,argv:argv,exit:status.exitstatus,observations:180,
         stdout_sha256:Digest::SHA256.hexdigest(stdout),stderr_sha256:Digest::SHA256.hexdigest(stderr)}
end
hashes.each{|p,h|raise "identity drift #{p}" unless Digest::SHA256.file(p).hexdigest==h}
File.write(out+'/report.json',JSON.pretty_generate({files:hashes,rows:rows,
  scope:'540 full-grid states with ordered neighbors and retained-snapshot assertions; crafted seeds only, no random preparation, comparator execution, production or timing claim.'}))
puts 'Verified540 complete Life states across three runtimes; structural ledger remains partial.'
