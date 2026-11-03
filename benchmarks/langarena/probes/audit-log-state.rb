# ASCII full-state oracle; comparator sources are pinned, not executed.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT' unless ARGV.size == 3
binary, upstream, out = ARGV.map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
pin = '3c9f84ddaeec834c0631747cb789e8cfd602f993'
sources = %w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/48_logs.kujo]
files = ([binary,File.expand_path(__FILE__)] + sources).to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
comparators = %w[rust/src/benchmarks/log_parser.rs goose/src/log_parser.goose].map do |path|
  bytes, err, status = Open3.capture3('git','-C',upstream,'show',"#{pin}:#{path}")
  raise err unless status.success? && bytes == File.binread(upstream+'/'+path)
  files[upstream+'/'+path] = Digest::SHA256.hexdigest(bytes)
  File.write(out+'/'+File.basename(path),bytes)
  bytes
end
patterns = JSON.parse(File.read(sources.last).match(/let patterns := (\[[^\n]+\])/)[1])
rust_patterns = comparators[0].scan(/Regex::new\(("(?:\\.|[^"\\])*")\)/m).flatten.map { |s| JSON.parse(s) }
goose_patterns = comparators[1].scan(/regex::compile\("""(.*?)"""\)/m).flatten
raise 'comparator pattern drift' unless patterns.size == 13 && patterns == rust_patterns && patterns == goose_patterns
names = JSON.parse(File.read(sources.last).match(/let names := (\[[^\n]+\])/)[1])
rust_names = comparators[0].scan(/\(\s*"([a-z_]+)",\s*Regex::new/m).flatten
goose_names = comparators[1].scan(/Pattern \{ "([a-z_]+)", regex::compile/).flatten
raise 'comparator name drift' unless names.size == 13 && names == rust_names && names == goose_names
regexes = patterns.map { |p| Regexp.new(p) }
methods = %w[GET POST PUT DELETE]
paths = %w[/index.html /api/users /admin /images/logo.png /etc/passwd /wp-admin/setup.php]
statuses = [200,201,301,302,400,401,403,404,500,502,503]
agents = %w[Mozilla/5.0 Googlebot/2.1 curl/7.68.0 scanner/2.0]
users = %w[john jane alex sarah mike anna david elena]
domains = %w[example.com gmail.com yahoo.com hotmail.com company.org mail.ru]
cases = [0,1,2,3,7,16,60,256].map do |count|
  text = (0...count).map do |i|
    path = if i%3 == 0
      "/login?email=#{users[i%8]}#{i%100}@#{domains[i%6]}&password=secret#{i%10000}"
    elsif i%5 == 0
      '/api/data?token=' + 'abcdef123456'*(i%3+1)
    elsif i%7 == 0
      '/user/profile?session_id=sess_' + (i*12345).to_s(16)
    else
      paths[i%6]
    end
    "192.168.1.#{i%255+1} - - [#{i%31}/Oct/2023:#{i%60}:55:36 +0000] \"#{methods[i%4]} #{path} HTTP/1.1\" #{statuses[i%11]} 2326 \"http://#{domains[i%6]}\" \"#{agents[i%4]}\"\n"
  end.join
  {prepare:true,state:{'cfg'=>{'name'=>'Etc::LogParser','lines_count'=>count},'result'=>0,'text'=>text}}
end
['', 'not a log', ' 404  500  robot ROBOT bot spider ', 'POST /api/foo HTTP /LOGIN /signin ../etc/passwd wp-admin',
 'a+b@example.com password=abc&token=xy api_key=zz session-id=ss',
 "password=\" token=& session_id=\n", '[1/Oct/2023:13:55:36 +0000] [1/Oct/2023:18:55:36 +0000]',
 '192.168.1.35 999.999.999.350 GETTING compost PUT'].each do |text|
  cases << {prepare:false,state:{'cfg'=>{'name'=>'Etc::LogParser','lines_count'=>0},'result'=>4294967290,'text'=>text}}
end
expected = cases.each_with_index.flat_map do |entry,index|
  state = Marshal.load(Marshal.dump(entry[:state]))
  matches = regexes.map { |regex| state['text'].scan(regex) }
  3.times.map do |step|
    state['result'] = (state['result'] + matches.sum(&:size)) & 0xffffffff
    {'case'=>index,'step'=>step,'state'=>Marshal.load(Marshal.dump(state)),'matches'=>matches,'counts'=>names.zip(matches.map(&:size)).to_h,'checksum'=>state['result']}
  end
end
fixture = sources.map { |p| File.read(p) }.join("\n") + "\nlet cases := parse_json(read_file(#{JSON.generate(out+'/cases.json')}))\nlet audit_patterns := #{JSON.generate(patterns)}\n" + <<'KUJO'
mut index := 0
for entry in cases {
    mut state := entry["state"]
    if entry["prepare"] { state = logs_prepare(basic_state(state["cfg"])) }
    mut matches := []
    for pattern in audit_patterns { matches = push(matches, regex_find_all(state["text"], pattern)) }
    mut step := 0
    while step < 3 {
        let snapshot := state
        let counts := logs_match_counts(state["text"])
        mut altered := counts
        altered["errors"] = -1
        assert(counts["errors"] >= 0)
        state = logs_run(state, step)
        assert_equal(snapshot["text"], state["text"])
        assert_equal(snapshot["cfg"], state["cfg"])

        print(to_json({"case":index,"step":step,"state":state,"matches":matches,"counts":counts,"checksum":result_checksum(state)}))
        step += 1
    }
    index += 1
}
KUJO
{'fixture.kujo'=>fixture,'cases.json'=>JSON.generate(cases),'expected.json'=>JSON.generate(expected)}.each do |name,bytes|
  File.write(out+'/'+name,bytes); files[out+'/'+name] = Digest::SHA256.hexdigest(bytes)
end
manifest = {pin:pin,files:files,cases:cases.size,states_per_runtime:expected.size,commands:[],scope:'ASCII full text, thirteen pinned named counts, per-pattern matched strings, snapshots and accumulated checksum; no competitor execution or speed claim'}
%w[vm interpreter jit].each do |mode|
  files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
  argv = [binary,'run','--scheduler-timeout-ms','120000']; argv << "--#{mode}" unless mode == 'vm'; argv << out+'/fixture.kujo'
  stdout, stderr, status = Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout); File.write(out+'/'+mode+'.err',stderr)
  manifest[:commands] << {mode:mode,argv:argv,exit:status.exitstatus,signal:status.termsig}
  File.write(out+'/manifest.json',JSON.pretty_generate(manifest))
  raise "#{mode} failed: #{stderr}" unless status.success?
  raise "#{mode} full state mismatch" unless stdout.lines.map { |l| JSON.parse(l) } == expected
end
files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest == h }
puts "#{expected.size*3} complete log states and per-pattern match lists verified"
