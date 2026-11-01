require 'json';require 'digest'
d=ARGV.fetch(0);lines=File.readlines(d+'/observations.txt',chomp:true);raise 'controls' unless lines.shift=='controls=50'
rows=lines.map do |l|
 m=l.match(/\Apair=(\d+) mode=(\w+) reuse=(true|false) iterations=(\d+) ns=(\d+) sum=(\d+)\z/);raise 'row' unless m
 {pair:m[1].to_i,mode:m[2],reuse:m[3]=='true',iterations:m[4].to_i,ns:m[5].to_i,sum:m[6].to_i}
end
modes=%w[absent empty enabled unicode invalid];expected=(0..6).flat_map{|p|modes.flat_map{|m|(p.even? ? [false,true] : [true,false]).map{|reuse|[p,m,reuse]}}}
raise 'order' unless rows.map{|r|r.values_at(:pair,:mode,:reuse)}==expected
rows.each{|r|raise 'result' unless r[:iterations]==200000&&r[:ns]>0&&r[:sum]==(%w[absent invalid].include?(r[:mode]) ? 0 : 1400000)}
summary=modes.map do |mode|
 ratios=(0..6).map{|p|a=rows.find{|r|r[:pair]==p&&r[:mode]==mode&&!r[:reuse]};b=rows.find{|r|r[:pair]==p&&r[:mode]==mode&&r[:reuse]};a[:ns].fdiv(b[:ns])}
 {mode:mode,ratios:ratios,median:ratios.sort[3]}
end
sha=->(p){Digest::SHA256.file(p).hexdigest}
report={status:'verified',observations:70,transition_controls:50,summary:summary,source_sha256:sha.call('benchmarks/langarena/probes/debug_call_environment.rs'),binary_sha256:sha.call(d+'/probe'),raw_sha256:sha.call(d+'/observations.txt'),limits:'Standalone single-threaded std::env lookup isolation. No full bridge, diagnostic formatting, concurrent writers or whole-runtime speed acceptance.'}
File.write(d+'/verified.json',JSON.pretty_generate(report)+"\n");summary.each{|r|puts "#{r[:mode]} #{r[:median].round(3)}x"}
