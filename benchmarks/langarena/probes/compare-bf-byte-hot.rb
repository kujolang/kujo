# Separately report warmed JIT kernel and process costs; no official score.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO COLD_TIMING_DIRECTORY NEW_OUTPUT' unless ARGV.size==3
binary,cold,out=ARGV.map { |p| File.expand_path(p) };raise 'output exists' if File.exist?(out)
raise 'diagnostic override' unless ENV.keys.grep(/\A(?:DEBUG_|DISABLE_.*JIT|KUJO_HASHMAP_PROFILE\z)/).empty?
prior=JSON.parse(File.read(cold+'/report.json'));raise 'cold run incomplete' unless prior['status']=='collected-review-pending'
prior['files'].each { |p,h| raise 'cold identity drift' unless Digest::SHA256.file(p).hexdigest==h }
FileUtils.mkdir_p(out)
%w[string byte].each do |variant|
  text=File.read(cold+'/'+variant+'.kujo');marker="\nmut result := 0\nmut iteration := 0\n"
  raise 'fixture structure' unless text.scan(marker).size==1
  base,body=text.split(marker,2)
  expression=body.match(/result = u32\(result \+ (bf_(?:byte_)?array\([^\n]+\))\) iteration \+= 1/)[1]
  source=base+"\nmut result := 0\nmut iteration := 0\nwhile iteration < 120 { result = u32(result + #{expression}) iteration += 1 }\nassert_equal(result, #{(prior['expected']*6)&0xffffffff})\nresult = 0\niteration = 0\nlet started := time_ns()\nwhile iteration < 20 { result = u32(result + #{expression}) iteration += 1 }\nlet elapsed := time_ns() - started\nprint(to_json({\"result\":result,\"elapsed_ns\":elapsed}))\n"
  File.write(out+'/'+variant+'.kujo',source)
end
files=[binary,File.expand_path(__FILE__),out+'/string.kujo',out+'/byte.kujo'].to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
report={status:'running',files:files,expected:prior['expected'],warmup_calls:120,measured_calls:20,rows:[],scope:'JIT only, same official warmup program:120 admission calls followed by20 measured calls. Seven alternating pairs; kernel monotonic time_ns and full process time reported separately. Loaded laptop diagnostic, not official production or overall speed.'}
save=-> { File.write(out+'/report.json',JSON.pretty_generate(report)) }
begin
  7.times do |round|
    (round.even? ? %w[string byte] : %w[byte string]).each do |variant|
      files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
      argv=[binary,'run','--jit','--scheduler-timeout-ms','120000',out+'/'+variant+'.kujo']
      start=Process.clock_gettime(Process::CLOCK_MONOTONIC);stdout,stderr,status=Open3.capture3(*argv);elapsed=Process.clock_gettime(Process::CLOCK_MONOTONIC)-start
      label="#{round}-#{variant}";File.write(out+'/'+label+'.out',stdout);File.write(out+'/'+label+'.err',stderr)
      data=JSON.parse(stdout) if status.success?
      passed=status.success? && data['result']==report[:expected] && data['elapsed_ns'].is_a?(Numeric) && data['elapsed_ns']>0
      report[:rows]<<{round:round,variant:variant,argv:argv,exit:status.exitstatus,signal:status.termsig,passed:passed,process_seconds:elapsed,kernel_seconds:data && data['elapsed_ns']/1e9,stdout:stdout,stderr:stderr};save.call
      raise "failed #{label}: #{stderr}" unless passed
    end
  end
  files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
  report[:comparisons]=%w[kernel_seconds process_seconds].map do |field|
    ratios=7.times.map do |round|
      pair=report[:rows].select { |r| r[:round]==round }.to_h { |r| [r[:variant],r[field.to_sym]] }
      pair['string']/pair['byte']
    end
    {metric:field,paired_speedups:ratios,median:ratios.sort[3],range:ratios.minmax}
  end
  report[:status]='collected-review-pending'
rescue => error
  report[:status]='failed';report[:error]=error.message;raise
ensure
  save.call
end
puts JSON.pretty_generate(report[:comparisons])
