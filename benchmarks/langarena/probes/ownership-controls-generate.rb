# Refine the first fixture set: separate scope and success-path assertion effects.
require 'json'
root=File.expand_path(ARGV.fetch(0))
fixtures=JSON.parse(File.read(root+'/fixtures.json')).select{|f|f['kind']=='ownership' && [[128,64],[2048,64],[2048,1024]].include?([f['size'],f['operations']])}
controls=[]
fixtures.each do |f|
 source=File.read(f['path'])
 # Keep exactly the same failed-condition assertions, but no native assertion
 # call on successful iterations. Snapshot reads still occur every iteration.
 guarded=source.gsub(/assert_equal\((.+), (i \+ 1|0)\)/){"if (#{$1}) != (#{$2}) { #{$&} }"}
 %w[global_guarded local_guarded].each do |scope|
  s=guarded
  if scope=='local_guarded'
   offset=s.index('let size :=');abort 'Missing fixture body' unless offset
   s=s[0...offset]+"func exercise() {\n"+s[offset..]+"}\nexercise()\n"
  end
  path=root+"/sources/control-#{scope}-#{f['shape']}-#{f['owner']}-#{f['size']}-#{f['operations']}.kujo"
  File.write(path,s)
  controls << f.merge('kind'=>'ownership_controls','scope'=>scope,'path'=>path)
 end
end
File.write(root+'/ownership-controls-fixtures.json',JSON.pretty_generate(controls)+"\n")
