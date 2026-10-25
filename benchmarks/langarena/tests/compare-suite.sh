#!/usr/bin/env bash
# Host-process fixtures for the Kujo comparison driver; no real benchmark work.
set -euo pipefail
kujo=${1:?Usage: compare-suite.sh /absolute/kujo}
root=$(mktemp -d)
cleanup() {
 status=$?
 if [[ $status == 0 ]]; then rm -rf "$root"
 else printf 'Comparison fixture failed; logs retained at %s\n' "$root" >&2
 fi
}
trap cleanup EXIT
cat > "$root/report.rb" <<'REPORT'
require 'json'
def read_report(path)
 report = JSON.parse(File.read(path))
 raise 'wrong schema' unless report.fetch('schema_version') == 2
 report['results'] = report.fetch('result_files').map do |name|
  row = JSON.parse(File.read(File.join(File.dirname(path), name)))
  %w[stdout_path stderr_path].each { |key| raise 'missing raw output' unless File.file?(File.join(File.dirname(path),row.fetch(key))) }
  row
 end
 report
end
REPORT
cat > "$root/fake" <<'FAKE'
#!/bin/sh
for argument do name=$argument; done
if [ "$1" = run ]; then
 printf '@comparison_ns 1000000\n%s: OK 123\nSummary: 1, 1, 0\n' "$name"
else
 printf '%s: OK in 0.001s\nSummary, 1, 1, 0\n' "$name"
fi
FAKE
chmod +x "$root/fake"
ruby -rjson -r "$root/report.rb" -e 'puts JSON.generate((1..50).map{|n|{name:"Fixture::#{n}",checksum:123}})' > "$root/config.json"
for runtime in vm interpreter jit; do
 mkdir "$root/$runtime"
 flags=(--scheduler-no-timeout)
 if [[ $runtime != vm ]]; then flags+=("--$runtime"); fi
 "$kujo" run "${flags[@]}" benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/$runtime" "$root/config.json" 1 1000 > "$root/$runtime.log" 2>&1
 ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0])["results"];abort unless r.size==300 && r.all?{|x|x["passed"]} && r.map{|x|[x["benchmark"],x["language"]]}.uniq.size==300' "$root/$runtime/samples.json"
done
# Parse both platform formats independently of the current host.
ruby -e 'print File.read(ARGV[0]).split("let options := args()").first' benchmarks/langarena/compare_suite.kujo > "$root/resource-parser.kujo"
cat >> "$root/resource-parser.kujo" <<'PARSE'
assert_equal(peak_rss_kib("Maximum resident set size (kbytes): 4096", "Linux"),4096.0)
assert_equal(peak_rss_kib(" 4194304 maximum resident set size", "Darwin"),4096.0)
assert_equal(peak_rss_kib("", "Linux"),null)
assert_equal(peak_rss_kib("Maximum resident set size (kbytes): bad", "Linux"),null)
PARSE
# Optional native time wrapper preserves checksums and records positive peak RSS.
for runtime in vm interpreter jit; do
 mkdir "$root/resources-$runtime"
 flags=(--scheduler-no-timeout)
 if [[ $runtime != vm ]]; then flags+=("--$runtime"); fi
 "$kujo" run "${flags[@]}" "$root/resource-parser.kujo" > "$root/parser-$runtime.log" 2>&1
 "$kujo" run "${flags[@]}" benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/resources-$runtime" "$root/config.json" 1 2000 Fixture::7 resources > "$root/resources-$runtime.log" 2>&1
 ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0]);abort unless r["resource_measurement"] && r["results"].size==6 && r["results"].all?{|x|x["passed"] && x["peak_rss_kib"]>0 && x["host_after"]["captured_ns"]>=x["host_before"]["captured_ns"]};if r["resource_platform"]=="Linux";abort unless r["results"].all?{|x|x["host_before"]["cpu_jiffies"].start_with?("cpu ") && x["host_after"]["swap_pages"].size==2};end' "$root/resources-$runtime/samples.json"
done
# A wrong checksum must fail while retaining every variant, not report success.
printf '[{"name":"Fixture::1","checksum":124}]\n' > "$root/wrong.json"
mkdir "$root/wrong"
if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/wrong" "$root/wrong.json" 1 1000 > "$root/wrong.log" 2>&1; then
 echo 'Wrong checksum accepted' >&2; exit 1
fi
ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0])["results"];abort unless r.size==6 && r.count{|x|!x["passed"]}==4' "$root/wrong/samples.json"
# Existing evidence must remain byte-for-byte intact.
before=$(shasum -a 256 "$root/vm/samples.json")
if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/vm" "$root/config.json" 1 1000 > "$root/reuse.log" 2>&1; then
 echo 'Existing output accepted' >&2; exit 1
fi
[[ $(shasum -a 256 "$root/vm/samples.json") == "$before" ]]
# Exact selection and rotating order across rounds.
mkdir "$root/selected"
"$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/selected" "$root/config.json" 2 1000 Fixture::7 > "$root/selected.log" 2>&1
ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0])["results"];abort unless r.size==12 && r.all?{|x|x["benchmark"]=="Fixture::7"} && r[0]["language"]=="Baseline VM" && r[6]["language"]=="Baseline JIT"' "$root/selected/samples.json"
# Malformed times and process deadlines remain failures with raw evidence.
sed -e 's/1000000/not-a-time/' -e 's/0.001/not-a-time/' "$root/fake" > "$root/malformed"
chmod +x "$root/malformed"
printf '#!/bin/sh\nexec sleep 2\n' > "$root/slow"
chmod +x "$root/slow"
for fixture in malformed slow; do
 mkdir "$root/$fixture-output"
 if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/$fixture" "$root/$fixture" "$root/$fixture" "$root/$fixture" "$root/$fixture-output" "$root/wrong.json" 1 50 > "$root/$fixture.log" 2>&1; then
  echo "$fixture accepted" >&2; exit 1
 fi
 ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0])["results"];abort unless r.size==6 && r.none?{|x|x["passed"]};abort if ARGV[1]=="slow" && !r.all?{|x|x["timed_out"]}' "$root/$fixture-output/samples.json" "$fixture"
done
# Truncated successful-looking output is not usable benchmark evidence.
cat "$root/fake" > "$root/truncated"
printf '/usr/bin/head -c 1100000 /dev/zero >&2\n' >> "$root/truncated"
chmod +x "$root/truncated"
mkdir "$root/truncated-output"
if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/truncated" "$root/truncated" "$root/truncated" "$root/truncated" "$root/truncated-output" "$root/config.json" 1 2000 Fixture::7 > "$root/truncated.log" 2>&1; then
 echo 'Truncated output accepted' >&2; exit 1
fi
ruby -rjson -r "$root/report.rb" -e 'r=read_report(ARGV[0])["results"];abort unless r.size==6 && r.all?{|x|!x["passed"] && x["stderr_truncated"]}' "$root/truncated-output/samples.json"
echo 'Comparison driver: 900 registry fixtures, 18 resource samples, both RSS formats, truncated output, wrong checksums, deadlines, output preservation and rotating selection passed across tested runtimes.'
