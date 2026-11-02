require 'json';require 'digest'
d=File.expand_path(ARGV.fetch(0));results=[]
Dir["#{d}/samples/*.sample.txt"].sort.each do |path|
 text=File.read(path);graph=text.split("Call graph:\n",2)[1]&.split("Total number in stack",2)&.first
 raise 'missing graph' unless graph
 nodes=[];stack=[];active=false;total=nil
 graph.each_line do |line|
  if line.match?(/^    \d+ Thread_/)
   active=line.include?(': kujo-kernel-profile');stack=[]
   raise 'multiple target threads' if active && total
  end
  next unless active
  m=line.match(/^[ +!:\|]*(\d+) (.+)$/);next unless m
  depth=m.begin(1);count=m[1].to_i;name=m[2].sub(/\s+\(in .*$/,'');total||=count
  stack.pop while stack.any? && nodes[stack.last][:depth]>=depth
  parent=stack.last
  node={depth:depth,count:count,children:0,name:name,parent:parent}
  nodes[parent][:children]+=count if parent
  nodes<<node;stack<<nodes.size-1
 end
 raise 'no target samples' unless total
 hist=Hash.new(0);movement=Hash.new(0)
 nodes.each do |n|
  weight=n[:count]-n[:children];raise "invalid tree #{n}" if weight<0
  hist[n[:name]]+=weight if weight>0
  if weight>0 && n[:name].match?(/memmove|memcpy|bzero/)
   ancestors=[];parent=n[:parent]
   while parent
    ancestors<<nodes[parent][:name];parent=nodes[parent][:parent]
   end
   nearest=ancestors.find{|name|name.start_with?('kujo::','jit_')}
   movement[nearest||'unresolved']+=weight
  end
 end
 raise 'weight conservation' unless hist.values.sum==total
 row={file:File.basename(path),sha256:Digest::SHA256.file(path).hexdigest,target_thread_samples:total,self_samples:hist.sort_by{|name,count|-count}.to_h,memory_movement_nearest_kujo_ancestor:movement.sort_by{|name,count|-count}.to_h}
 results<<row
 puts "#{row[:file]} samples=#{total} self-weight verified; memory movement: #{movement.sort_by{|k,v|-v}.first(7).inspect}"
end
raise 'six profiles required' unless results.size==6
File.write("#{d}/stack-attribution.json",JSON.pretty_generate({scope:'Exclusive sampled stacks on kujo-kernel-profile thread only; waiting parent excluded. Sampling occupancy is not CPU self time; unresolved generated native frames retained. Ancestor attribution is descriptive, not causal proof.',rows:results})+"\n")
