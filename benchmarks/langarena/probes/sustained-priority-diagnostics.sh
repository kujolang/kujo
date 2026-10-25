#!/bin/bash
set -u
repo="$PWD"
root=${1:?Expected absolute directory containing priority-probe}
test "${root:0:1}" = / || exit 2
test -x "$root/priority-probe" || exit 2
for mode in vm jit; do
 for name in Etc::CacheSimulation Etc::GameOfLife Brainfuck::Array Brainfuck::Recursion Base64::Encode; do
  slug=${name//::/-}
  stem="$root/$mode-$slug-sustained"
  reps=100
  if [ "$name" = 'Etc::CacheSimulation' ]; then reps=10; fi
  work="$stem-work"
  test ! -e "$work" || exit 2
  mkdir -p "$work/benchmarks/langarena"
  cp benchmarks/langarena/test.json "$work/benchmarks/langarena/test.json"
  ruby -e 's=File.read(ARGV[0]); abort "entry mismatch" unless s.end_with?("main()\n\n"); s=s.sub(/main\(\)\n\n\z/,"mut diagnostic_round := 0\nwhile diagnostic_round < #{ARGV[2]} { main() diagnostic_round += 1 }\n"); File.write(ARGV[1],s)' benchmarks/langarena/port.kujo "$work/benchmarks/langarena/port.kujo" "$reps"
  echo "START sustained $mode $name repetitions=$reps"
  (cd "$work"; exec "$root/priority-probe" "$mode" "$name" sample) > "$stem.stdout" 2> "$stem.stderr" &
  child=$!
  /usr/bin/sample "$child" 5 2 -mayDie -file "$stem.cpu.txt" > "$stem.sampler.log" 2>&1
  wait "$child"
  echo "$?" > "$stem.exit"
  echo "$reps" > "$stem.repetitions"
  echo "DONE sustained $mode $name"
 done
done
