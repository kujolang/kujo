require 'json'
require 'digest'
require 'fileutils'
require 'open3'
binary=File.expand_path(ARGV.fetch(0)); out=File.expand_path(ARGV.fetch(1))
raise 'Output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
report={binary:binary,binary_sha256:Digest::SHA256.file(binary).hexdigest,rows:[],status:'running'}
save=->{File.write(out+'/report.json',JSON.pretty_generate(report)+"\n")};save.call
begin
 %w[root nested].each do |shape|
  dir=out+'/'+shape;FileUtils.mkdir_p(dir)
  declaration=shape=='root' ? 'mut state := {"0": Item { value: 0 }, "1": Item { value: 1 }}' : 'mut state := {"branch": {"0": Item { value: 0 }, "1": Item { value: 1 }}}'
  row=shape=='root' ? 'state' : 'state["branch"]'
  owner=<<~KUJO
   struct Item { value: int }
   struct Envelope { before: array, value: int, after: array }
   #{declaration}
   export func snapshot() { return #{row} }
   export func edit(n) { #{row}[0] = Item { value: n } }
   func change() { assert_equal(#{row}[0].value, 20) #{row}[1] = Item { value: 21 } return 22 }
   export func ordered() {
     #{row}[0] = Item { value: 20 }
     let result := Envelope { before: [#{row}], value: change(), after: [#{row}] }
     #{row}[0] = Item { value: 23 }
     return result
   }
   export func failure() {
     #{row}[0] = Item { value: 30 }
     #{row}[0] = Item { value: [1][99] }
   }
   export func expose() {
     #{row}[0] = Item { value: 40 }
     let result := Envelope { before: [#{row}], value: 41, after: [#{row}] }
     #{row}[0] = Item { value: 42 }
     return result
   }
  KUJO
  main=<<~KUJO
   from owner import snapshot, edit, ordered, failure, expose
   let original := snapshot()
   edit(2) edit(3)
   assert_equal(original[0].value, 0)
   assert_equal(original[1].value, 1)
   let ordered_result := ordered()
   assert_equal(ordered_result.before[0][0].value, 20)
   assert_equal(ordered_result.before[0][1].value, 1)
   assert_equal(ordered_result.value, 22)
   assert_equal(ordered_result.after[0][0].value, 20)
   assert_equal(ordered_result.after[0][1].value, 21)
   assert_equal(snapshot()[0].value, 23)
   mut caught := false
   try { failure() } except e { caught = true }
   assert(caught)
   assert_equal(snapshot()[0].value, 30)
   let exposed := expose()
   assert_equal(exposed.before[0][0].value, 40)
   assert_equal(exposed.after[0][0].value, 40)
   assert_equal(snapshot()[0].value, 42)
   assert_equal(original[0].value, 0)
   print("STRUCT BOUNDARIES PASS")
  KUJO
  File.write(dir+'/owner.kujo',owner);File.write(dir+'/main.kujo',main)
  %w[vm interpreter jit].each do |mode|
   argv=[binary,'run',dir+'/main.kujo','--untrusted'];argv<<'--'+mode unless mode=='vm'
   stdout,stderr,status=Open3.capture3(*argv)
   report[:rows]<<{shape:shape,mode:mode,argv:argv,exit:status.exitstatus,stdout:stdout,stderr:stderr,owner_sha256:Digest::SHA256.hexdigest(owner),main_sha256:Digest::SHA256.hexdigest(main)};save.call
   raise "Failed #{shape}/#{mode}: #{stdout} #{stderr}" unless status.success?&&stdout=="STRUCT BOUNDARIES PASS\n"
  end
 end
 raise 'Binary changed' unless Digest::SHA256.file(binary).hexdigest==report[:binary_sha256]
 report[:status]='passed';save.call
rescue=>e
 report[:status]='failed';report[:error]=e.message;save.call;raise
end
