# Repository-only profiling fixture generation; all runtime workloads stay Kujo.
require 'json'
require 'fileutils'
root=File.expand_path(ARGV.fetch(0)); FileUtils.mkdir_p(root+'/sources')
port=File.read('benchmarks/langarena/port.kujo')
cases=%w[Sort::Quick Sort::Merge Etc::NeuralNet Etc::CacheSimulation Etc::GameOfLife Brainfuck::Array Brainfuck::Recursion Etc::TextRaytracer Base64::Encode]
manifest=[]
cases.each do |name|
 s=port.sub('let arguments := args()', 'let arguments := ["benchmarks/langarena/test.json", "'+name+'", "--profile"]')
 # Private marker writes classify subsequent allocations. These instrumented
 # runs are counts only, never comparative timings. Runtime source is unchanged.
 {'mut state := create(cfg)'=>'1','state = prepare(state)'=>'2','while i < warmups {'=>'3','while i < cfg["iterations"] {'=>'4','let actual := u32(checksum(state))'=>'5'}.each do |needle,phase|
  abort 'ambiguous marker' unless s.scan(needle).size==1
  s=s.sub(needle, 'env_set("KUJO_ALLOCATION_PHASE", "'+phase+'") '+needle)
 end
 path=root+'/sources/phase-'+name.gsub('::','-')+'.kujo';File.write(path,s)
 manifest << {kind:'phase',name:name,path:path}
end
[[128,64],[512,64],[2048,64],[2048,256],[2048,1024]].each do |n,operations|
 %w[array nested dictionary helper struct_rows].each do |kind|
  %w[unique once each].each do |owner|
   init=case kind
   when 'nested' then 'mut data := {"row": values}'
   when 'struct_rows' then 'mut data := {"rows": values}'
   when 'dictionary' then 'mut data := {} mut dict_key := 0 while dict_key < size { data[str(dict_key)] = 0 dict_key += 1 }'
   else 'mut data := values'
   end
   read=case kind
   when 'nested' then 'data["row"][i]'
   when 'struct_rows' then 'data["rows"][i].value'
   when 'dictionary' then 'data[str(i)]'
   else 'data[i]'
   end
   mutation=kind=='helper' ? 'data = update(data, i)' : read+' = i + 1'
   oldread=read.sub('data','saved')
   source=<<~KUJO
    func update(input, index) { mut result := input result[index] = index + 1 return result }
    #{kind=='struct_rows' ? 'struct Cell { value: int }' : ''}
    let size := #{n}
    mut values := [] mut k := 0 while k < size { values = push(values, #{kind=='struct_rows' ? 'Cell { value: 0 }' : '0'}) k += 1 }
    #{init}
    values = []
    mut saved := []
    #{owner=='once' ? 'saved = data' : ''}
    env_set("KUJO_ALLOCATION_PHASE", "4")
    mut i := 0
    while i < #{operations} {
      #{owner=='each' ? 'saved = data' : ''}
      #{mutation}
      assert_equal(#{read}, i + 1)
      #{owner!='unique' ? 'assert_equal('+oldread+', 0)' : ''}
      i += 1
    }
    env_set("KUJO_ALLOCATION_PHASE", "5")
    print("ISOLATION PASS #{kind} #{owner} #{n}")
   KUJO
   path=root+"/sources/ownership-#{kind}-#{owner}-#{n}-#{operations}.kujo";File.write(path,source)
   manifest << {kind:'ownership',shape:kind,owner:owner,size:n,operations:operations,path:path}
  end
 end
end
File.write(root+'/fixtures.json',JSON.pretty_generate(manifest)+"\n")
[32,512,8192,131072].each do |iterations|
 text="encode aaaa... to YWFh...: #{40*(iterations+1)}"
 checksum=text.bytes.reduce(5381){|h,b|(h*33+b)&0xffffffff}
 cfg=[{name:'Base64::Encode',size:30,iterations:iterations,warmup_iterations:1,checksum:checksum}]
 File.write(root+"/base64-#{iterations}.json",JSON.pretty_generate(cfg)+"\n")
end
