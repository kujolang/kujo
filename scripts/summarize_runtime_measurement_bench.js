// node scripts/summarize_runtime_measurement_bench.js raw.json > summary.json
const fs = require('node:fs');
const input = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
function stats(xs) {
    const sorted = [...xs].sort((a,b)=>a-b);
    return {min:sorted[0], median:sorted[Math.floor(sorted.length/2)], p90:sorted[Math.ceil(sorted.length*.9)-1], max:sorted.at(-1)};
}
const workloads = input.workloads.map(w => {
    const base=w.baseline_ns, off=w.disabled_ns, on=w.enabled_ns;
    if (![base,off,on].every(xs=>xs.length===input.samples_per_mode && xs.every(x=>Number.isSafeInteger(x)&&x>0))) throw Error('invalid samples');
    return {name:w.name, baseline_ms:stats(base.map(x=>x/1e6)), disabled_ms:stats(off.map(x=>x/1e6)), enabled_ms:stats(on.map(x=>x/1e6)),
        paired_disabled_minus_baseline_ms:stats(off.map((x,i)=>(x-base[i])/1e6)),
        paired_enabled_minus_disabled_ms:stats(on.map((x,i)=>(x-off[i])/1e6)),
        paired_disabled_over_baseline_pct:stats(off.map((x,i)=>(x/base[i]-1)*100)),
        paired_enabled_over_disabled_pct:stats(on.map((x,i)=>(x/off[i]-1)*100))};
});
console.log(JSON.stringify({method:'median and nearest-rank p90; same-round paired deltas; warmups excluded',workloads},null,2));
