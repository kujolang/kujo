#!/bin/bash
set -u
# Run from the repository root after building priority-probe in OUTPUT_DIRECTORY.
# Diagnostic-only CPU sampling; never use sampled runs as timing comparisons.
root=${1:?Expected absolute output directory containing priority-probe}
test "${root:0:1}" = / || exit 2
test -x "$root/priority-probe" || exit 2
probe="$root/priority-probe"
observe_host() {
  while true; do
    date -u +%FT%TZ
    uptime
    vm_stat
    pmset -g therm
    sleep 10
  done
}
observe_host >> "$root/host-during.log" 2>&1 &
observer=$!
trap 'kill "$observer" 2>/dev/null' EXIT
for mode in vm jit; do
  for case_name in Sort::Quick Sort::Merge Etc::NeuralNet Etc::CacheSimulation Etc::GameOfLife Brainfuck::Array Brainfuck::Recursion Etc::TextRaytracer Base64::Encode; do
    slug=${case_name//::/-}
    for kind in count sample; do
      stem="$root/$mode-$slug-$kind"
      if [ -e "$stem.exit" ]; then
        test -f "$stem.stdout" && test -f "$stem.stderr" || exit 2
        echo "PRESERVED $mode $case_name $kind"
        continue
      fi
      test ! -e "$stem.stdout" || exit 2
      echo "START $mode $case_name $kind"
      "$probe" "$mode" "$case_name" "$kind" > "$stem.stdout" 2> "$stem.stderr" &
      child=$!
      (sleep 180; kill -TERM "$child" 2>/dev/null) &
      watchdog=$!
      if [ "$kind" = sample ]; then
        /usr/bin/sample "$child" 3 2 -mayDie -file "$stem.cpu.txt" > "$stem.sampler.log" 2>&1
      fi
      wait "$child"
      result=$?
      kill "$watchdog" 2>/dev/null
      wait "$watchdog" 2>/dev/null
      echo "$result" > "$stem.exit"
      echo "DONE $mode $case_name $kind exit=$result"
    done
    stem="$root/$mode-$slug-counters"
    if [ -e "$stem.exit" ]; then
      test -s "$stem.json" && test -f "$stem.stdout" && test -f "$stem.stderr" || exit 2
      echo "PRESERVED $mode $case_name counters"
      continue
    fi
    test ! -e "$stem.json" && test ! -e "$stem.stdout" || exit 2
    opts=(--scheduler-no-timeout)
    if [ "$mode" = jit ]; then opts+=(--jit); fi
    /usr/bin/time -l target/release/kujo run "${opts[@]}" --measurements "$stem.json" benchmarks/langarena/port.kujo benchmarks/langarena/test.json "$case_name" --profile > "$stem.stdout" 2> "$stem.stderr"
    echo "$?" > "$stem.exit"
  done
done
