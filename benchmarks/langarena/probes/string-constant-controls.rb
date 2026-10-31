# Repository-maintenance allocation diagnostic. Runtime/workload semantics unchanged.
require 'json'
require 'digest'
require 'open3'
require 'fileutils'
probe, cli, directory = ARGV
raise 'usage: PROBE ACCEPTED_CLI NEW_OUTPUT_DIRECTORY' unless directory
probe = File.realpath(probe)
cli = File.realpath(cli)
raise 'new output directory required' if File.exist?(directory)
FileUtils.mkdir_p(directory)
directory = File.expand_path(directory)
sha = ->(path) { Digest::SHA256.file(path).hexdigest }
raise 'wrong accepted CLI' unless sha.call(cli) == '45b21be67fe946d54cabaa5be20f67b4ab26414135afb2a3c5a55af1b672393f'
identity = [probe, cli, __FILE__, 'src/vm.rs', 'src/compiler.rs'].to_h { |path| [path, sha.call(path)] }
environment = {'KUJO_ALLOCATION_PHASE'=>'0', 'DEBUG_JIT_CALL'=>nil, 'DEBUG_VM'=>nil, 'DEBUG_JIT'=>nil, 'DISABLE_FUNCTION_JIT'=>nil}
run = ->(argv) { out, err, status = Open3.capture3(environment, *argv); raise "failed #{argv.inspect}: #{err}" unless status.success?; {argv:argv, stdout:out, stderr:err, exit:status.exitstatus} }
control = run.call([probe, 'micro'])
raise 'positive allocation control' unless control[:stdout].match?(/^MICRO mode=named_opcode size=128 operations=128 requests=128 bytes=[1-9]\d* /)
raise 'zero allocation control' unless control[:stdout].match?(/^MICRO mode=scalar_opcode size=128 operations=128 requests=0 bytes=0 /)
File.write(directory+'/allocator-controls.json', JSON.pretty_generate(control))
fixtures = []
[8, 256].each do |width|
  [200, 2000].each do |reads|
    %w[literal bound].each do |form|
      key = 'a' * width
      index = form == 'literal' ? key.to_json : 'key'
      source = <<~KUJO
        func scan(data, key, reads) {
            mut sum := 0
            mut i := 0
            while i < reads { sum += data[#{index}] i += 1 }
            return sum
        }
        let key := #{key.to_json}
        let data := {#{key.to_json}: 7}
        assert_equal(scan(data, key, 2), 14)
        env_set("KUJO_ALLOCATION_PHASE", "1")
        let result := scan(data, key, #{reads})
        env_set("KUJO_ALLOCATION_PHASE", "0")
        assert_equal(result, #{reads * 7})
        assert_equal(data[key], 7)
        print("PASS")
      KUJO
      path = directory+"/#{form}-#{width}-#{reads}.kujo"
      File.write(path, source)
      fixtures << {form:form, width:width, reads:reads, path:path, sha256:sha.call(path)}
    end
  end
end
report = {status:'running', identities:identity, fixtures:fixtures, allocations:[], semantics:[], limits:'Phase1 includes the measured call and fixed marker costs; warmup/compiler/setup are phase0. Compare matched slopes, not elapsed time, retained memory or whole-language speed.'}
save = -> { File.write(directory+'/results.json', JSON.pretty_generate(report)+"\n") }
save.call
begin
  fixtures.each do |fixture|
    %w[vm interpreter jit].each do |mode|
      argv = [cli, 'run']
      argv << '--interpreter' if mode == 'interpreter'
      argv << '--jit' if mode == 'jit'
      metrics = directory+"/#{fixture[:form]}-#{fixture[:width]}-#{fixture[:reads]}-#{mode}.metrics.json"
      argv += ['--measurements', metrics] unless mode == 'interpreter'
      result = run.call(argv+[fixture[:path]])
      raise 'semantic stdout' unless result[:stdout] == "PASS\n"
      unless mode == 'interpreter'
        measurement = JSON.parse(File.read(metrics))
        entries = measurement.fetch('counters').fetch('jit_native_function_entries')
        raise 'native coverage' unless mode == 'jit' ? entries >= 2 : entries == 0
        result[:measurements] = measurement
      end
      report[:semantics] << fixture.merge(mode:mode, result:result)
      save.call
    end
  end
  3.times do |round|
    (round.even? ? fixtures : fixtures.reverse).each do |fixture|
      %w[vm jit].each do |mode|
        result = run.call([probe, mode, fixture[:path], 'count'])
        raise 'allocation stdout' unless result[:stdout] == "PASS\n"
        phase = result[:stderr].lines.grep(/^PHASE 1 /)
        raise 'phase inventory' unless phase.size == 1
        values = phase.first.match(/^PHASE 1 requests=(\d+) bytes=(\d+)\s*$/)
        raise 'phase format' unless values
        counters = result[:stderr].lines.grep(/^COUNTERS /)
        raise 'counter inventory' unless counters.size == 1
        entries = JSON.parse(counters.first.delete_prefix('COUNTERS ')).fetch('counters').fetch('jit_native_function_entries')
        raise 'allocation native coverage' unless mode == 'jit' ? entries >= 2 : entries == 0
        report[:allocations] << fixture.merge(round:round, mode:mode, requests:values[1].to_i, bytes:values[2].to_i, native_entries:entries, result:result)
        save.call
      end
    end
  end
  identity.each { |path, hash| raise 'identity drift' unless sha.call(path) == hash }
  fixtures.each { |f| raise 'fixture drift' unless sha.call(f[:path]) == f[:sha256] }
  report[:status] = 'measured-review-pending'
  save.call
rescue => error
  report[:status] = 'failed'
  report[:error] = error.message
  save.call
  raise
end
