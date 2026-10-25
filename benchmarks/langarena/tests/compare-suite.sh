#!/usr/bin/env bash
# Host-process fixtures for the Kujo comparison driver; no real benchmark work.
set -euo pipefail
kujo=${1:?Usage: compare-suite.sh /absolute/kujo}
root=$(mktemp -d)
trap 'rm -rf "$root"' EXIT
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
ruby -rjson -e 'puts JSON.generate((1..50).map{|n|{name:"Fixture::#{n}",checksum:123}})' > "$root/config.json"
for runtime in vm interpreter jit; do
 mkdir "$root/$runtime"
 flags=(--scheduler-no-timeout)
 if [[ $runtime != vm ]]; then flags+=("--$runtime"); fi
 "$kujo" run "${flags[@]}" benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/$runtime" "$root/config.json" 1 1000 > "$root/$runtime.log" 2>&1
 ruby -rjson -e 'r=JSON.parse(File.read(ARGV[0]))["results"];abort unless r.size==300 && r.all?{|x|x["passed"]} && r.map{|x|[x["benchmark"],x["language"]]}.uniq.size==300' "$root/$runtime/samples.json"
done
# A wrong checksum must fail while retaining every variant, not report success.
printf '[{"name":"Fixture::1","checksum":124}]\n' > "$root/wrong.json"
mkdir "$root/wrong"
if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/wrong" "$root/wrong.json" 1 1000 > "$root/wrong.log" 2>&1; then
 echo 'Wrong checksum accepted' >&2; exit 1
fi
ruby -rjson -e 'r=JSON.parse(File.read(ARGV[0]))["results"];abort unless r.size==6 && r.count{|x|!x["passed"]}==4' "$root/wrong/samples.json"
# Existing evidence must remain byte-for-byte intact.
before=$(shasum -a 256 "$root/vm/samples.json")
if "$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/vm" "$root/config.json" 1 1000 > "$root/reuse.log" 2>&1; then
 echo 'Existing output accepted' >&2; exit 1
fi
[[ $(shasum -a 256 "$root/vm/samples.json") == "$before" ]]
# Exact selection and rotating order across rounds.
mkdir "$root/selected"
"$kujo" run benchmarks/langarena/compare_suite.kujo "$root/fake" "$root/fake" "$root/fake" "$root/fake" "$root/selected" "$root/config.json" 2 1000 Fixture::7 > "$root/selected.log" 2>&1
ruby -rjson -e 'r=JSON.parse(File.read(ARGV[0]))["results"];abort unless r.size==12 && r.all?{|x|x["benchmark"]=="Fixture::7"} && r[0]["language"]=="Baseline VM" && r[6]["language"]=="Baseline JIT"' "$root/selected/samples.json"
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
 ruby -rjson -e 'r=JSON.parse(File.read(ARGV[0]))["results"];abort unless r.size==6 && r.none?{|x|x["passed"]};abort if ARGV[1]=="slow" && !r.all?{|x|x["timed_out"]}' "$root/$fixture-output/samples.json" "$fixture"
done
echo 'Comparison driver: 900 fixture results across VM/interpreter/JIT, wrong checksum, output preservation and rotating selection passed.'
