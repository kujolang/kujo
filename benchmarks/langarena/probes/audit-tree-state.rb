# Maintenance differential oracle: unchanged pinned Rust kernels and complete
# Kujo tree state. No timing or production-equivalence claim.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
arena,binary,output=ARGV.map{|p|File.expand_path(p)}
raise 'expected upstream, frozen Kujo CLI, new output' unless ARGV.size==3 && !File.exist?(output)
revision,status=Open3.capture2('git','-C',arena,'rev-parse','HEAD')
raise 'wrong upstream' unless status.success? && revision.strip=='3c9f84ddaeec834c0631747cb789e8cfd602f993'
paths=%w[rust/src/benchmarks/binarytrees.rs goose/src/binarytrees.goose]
paths.each do |path|
 pinned,status=Open3.capture2('git','-C',arena,'show',"HEAD:#{path}")
 raise 'modified upstream source' unless status.success? && pinned==File.read(arena+'/'+path)
end
sources=paths.map{|p|arena+'/'+p}+[binary,'benchmarks/langarena/port.kujo',__FILE__]
hashes=sources.to_h{|p|[p,Digest::SHA256.file(p).hexdigest]}
rust=File.read(arena+'/'+paths.first)
raise 'imports changed' unless rust.start_with?("use super::super::config_i64;\nuse super::super::Benchmark;\n")
rust=rust.sub("use super::super::config_i64;\nuse super::super::Benchmark;\n",'')
cases=[0,1,2,4,7].product([-17,0,23])+[[10,0]]
FileUtils.mkdir_p(output)
oracle=<<~RS
  // Unchanged kernels from pinned LangArena; MIT, see LICENSE.upstream.
  fn config_i64(_: &str, _: &str) -> i64 { 4 }
  trait Benchmark { fn name(&self)->String; fn run(&mut self, i:i64); fn checksum(&self)->u32; }
  #{rust}
  // Inspection only: serialize actual pointer-tree links in preorder.
  fn flatten(node: &TreeNode, out: &mut Vec<[i32;3]>) -> i32 {
      let index=out.len() as i32; out.push([node.item,-1,-1]);
      let left=node.left.as_ref().map_or(-1,|n|flatten(n,out));
      let right=node.right.as_ref().map_or(-1,|n|flatten(n,out));
      out[index as usize]=[node.item,left,right]; index
  }
  fn main() {
RS
cases.each do |depth,item|
 oracle+="let root=TreeNode::new(#{item},#{depth});let mut nodes=Vec::new();flatten(&root,&mut nodes);let mut arena=BinarytreesArena{arena:Vec::new(),n:#{depth},result_val:0};assert_eq!(arena.build_tree(#{item},#{depth}),0);let rows:Vec<[i32;3]>=arena.arena.iter().map(|n|[n.item,n.left,n.right]).collect();assert_eq!(nodes,rows);assert_eq!(root.sum(),arena.sum(0));println!(\"{:?}\",nodes);println!(\"{}\",root.sum());\n"
end
oracle+="}\n";File.write(output+'/oracle.rs',oracle)
argv=['rustc','--edition=2021',output+'/oracle.rs','-o',output+'/oracle']
build_argv=argv.dup
out,err,status=Open3.capture3(*argv);File.write(output+'/build.out',out);File.write(output+'/build.err',err);raise 'oracle build failed' unless status.success?
out,err,status=Open3.capture3(output+'/oracle');File.write(output+'/oracle.out',out);File.write(output+'/oracle.err',err);raise 'oracle execution failed' unless status.success?
lines=out.lines;raise 'coverage' unless lines.size==cases.size*2
expected=cases.each_index.map{|i|{nodes:JSON.parse(lines[i*2]),sum:Integer(lines[i*2+1])}}
port=File.read('benchmarks/langarena/port.kujo');raise 'runner boundary' unless port.scan('func profile_checkpoint(').size==1
script=port.split('func profile_checkpoint(').first+<<~KUJO
  func inspect_tree(node, input) {
      mut rows := input
      let index := len(rows)
      rows = push(rows,[node.item,-1,-1])
      mut left := -1
      mut right := -1
      if node.left != null { left = len(rows) rows = inspect_tree(node.left,rows) }
      if node.right != null { right = len(rows) rows = inspect_tree(node.right,rows) }
      rows[index] = [node.item,left,right]
      return rows
  }
KUJO
cases.zip(expected).each do |(depth,item),e|
 script+="\nlet root_#{depth}_#{item.to_s.tr('-','m')} := tree_new(#{item},#{depth})\n"
 name="root_#{depth}_#{item.to_s.tr('-','m')}"
 script+="assert_equal(inspect_tree(#{name},[]),#{e[:nodes].to_json})\nassert_equal(u32(tree_sum(#{name})),#{e[:sum]})\nassert_equal(arena_build([],#{item},#{depth}),#{e[:nodes].to_json})\nassert_equal(u32(arena_sum(arena_build([],#{item},#{depth}),0)),#{e[:sum]})\n"
end
script+="print(\"tree-full-state: OK\")\n";File.write(output+'/contracts.kujo',script)
raise 'source size limit' unless script.bytesize<=1_048_576
rows=%w[vm interpreter jit].map do |mode|
 argv=[binary,'run','--scheduler-timeout-ms','60000'];argv<<"--#{mode}" unless mode=='vm';argv<<output+'/contracts.kujo'
 out,err,status=Open3.capture3(*argv);File.write(output+"/#{mode}.out",out);File.write(output+"/#{mode}.err",err)
 raise "#{mode}: #{err}" unless status.success? && out.strip=='tree-full-state: OK'
 {mode:mode,argv:argv,exit:status.exitstatus,stdout:out,stderr:err}
end
sources.each{|p|raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==hashes[p]}
version,status=Open3.capture2('rustc','--version');raise 'toolchain' unless status.success?
File.write(output+'/report.json',JSON.pretty_generate({scope:'16 bounded full-state Rust/all-three-Kujo cases; Goose source review only; no timing/production/native-coverage claim',rustc:version.strip,sources:hashes,cases:cases.zip(expected).map{|(depth,item),e|e.merge(depth:depth,item:item)},rows:rows,build_argv:build_argv}))
puts '16 complete object/arena states and checksums agree across Rust and three Kujo runtimes.'
