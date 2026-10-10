// Repository-maintenance orchestration: Node's standard library keeps the same
// evidence format on Linux, macOS and Windows; workloads and runners remain Rust/Kujo.
'use strict';
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const crypto = require('node:crypto');
const { spawn, spawnSync } = require('node:child_process');

function stats(values) {
    if (!values.length || values.some(v => !Number.isFinite(v))) throw Error('invalid samples');
    const sorted = [...values].sort((a, b) => a - b);
    const mean = values.reduce((a, b) => a + b, 0) / values.length;
    return { min: sorted[0], median: sorted[Math.floor(sorted.length / 2)],
        p90: sorted[Math.ceil(sorted.length * 0.9) - 1], max: sorted.at(-1),
        mean, standard_deviation: Math.sqrt(values.reduce((n, v) => n + (v - mean) ** 2, 0) / values.length) };
}
function samples(values, count) {
    if (!Array.isArray(values) || values.length !== count || values.some(v => !Number.isSafeInteger(v) || v <= 0)) throw Error('invalid timing samples');
    return values;
}
function summarize(runtime, analysis) {
    if (runtime.schema !== 'kujo.measurement-benchmark/v1' || analysis.schema !== 'kujo.static-analysis-benchmark/v1') throw Error('unexpected benchmark schema');
    if (runtime.samples_per_mode !== 11 || analysis.samples_per_binary !== 21) throw Error('unexpected sample count');
    const seen = new Set();
    const workloads = runtime.workloads.map(w => {
        if (seen.has(w.name)) throw Error('duplicate workload');
        seen.add(w.name);
        const base = samples(w.baseline_ns, 11), candidate = samples(w.disabled_ns, 11);
        const enabled = samples(w.enabled_ns, 11);
        if (w.enabled_reports.length !== 11) throw Error('missing measurement reports');
        for (const r of w.enabled_reports) {
            if (r.schema !== 'kujo.runtime-measurements/v1' || r.outcome !== 'success' || r.runtime !== 'vm') throw Error('invalid runtime report');
            for (const k of ['cpu_seconds', 'process_peak_rss_bytes']) {
                if (r[k] !== null && (!Number.isFinite(r[k]) || r[k] < 0)) throw Error('invalid resource counter');
            }
            if (!r.counters || Object.values(r.counters).some(v => !Number.isSafeInteger(v) || v < 0)) throw Error('invalid runtime counters');
        }
        const resource = key => {
            const xs = w.enabled_reports.map(r => r[key]);
            return xs.every(v => v !== null) ? stats(xs) : null;
        };
        const counters = Object.fromEntries(Object.keys(w.enabled_reports[0].counters).map(k =>
            [k, stats(w.enabled_reports.map(r => r.counters[k]))]));
        return { name: w.name, iterations: w.iterations, baseline_ms: stats(base.map(v => v / 1e6)),
            candidate_ms: stats(candidate.map(v => v / 1e6)), instrumented_ms: stats(enabled.map(v => v / 1e6)),
            paired_candidate_delta_pct: stats(candidate.map((v, i) => (v / base[i] - 1) * 100)),
            cpu_seconds: resource('cpu_seconds'), process_peak_rss_bytes: resource('process_peak_rss_bytes'),
            counters };
    });
    if (!workloads.length) throw Error('empty runtime corpus');
    const staticWorkloads = analysis.workloads.map(w => ({ name: w.name,
        baseline_ms: stats(samples(w.baseline_ns, 21).map(v => v / 1e6)),
        candidate_ms: stats(samples(w.candidate_ns, 21).map(v => v / 1e6)) }));
    if (staticWorkloads.length !== 3) throw Error('incomplete static corpus');
    return { schema: 'kujo.release-performance-summary/v1', workloads, static_workloads: staticWorkloads,
        interpretation: 'Warm process-inclusive samples; VM counters and CPU/RSS are separate instrumented runs. RSS is process-lifetime peak, not retained heap. Null is unsupported/unavailable, never zero. A/A differences are noise, not improvements.' };
}
const sha256 = file => crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
function run(command, args, options = {}) {
    const r = spawnSync(command, args, { encoding: 'utf8', timeout: 300000, maxBuffer: 16 * 1024 * 1024, ...options });
    if (r.error || r.status !== 0) throw Error(`${path.basename(command)} failed: ${r.error?.message || `exit ${r.status}`}\n${(r.stderr || '').slice(-4000)}`);
    return r.stdout;
}
function runBenchmark(command, args, stdio, timeoutMs = 1200000) {
    return new Promise((resolve, reject) => {
        const child = spawn(command, args, { stdio, detached: process.platform !== 'win32' });
        let expired = false;
        const timer = setTimeout(() => {
            expired = true;
            // Kill the owned tree while its parent is still present on Windows;
            // on Unix the detached runner owns the entire benchmark process group.
            if (process.platform === 'win32') {
                const cleanup = spawnSync('taskkill', ['/pid', String(child.pid), '/t', '/f'], { encoding: 'utf8', timeout: 10000 });
                if (cleanup.error || cleanup.status !== 0) reject(Error('benchmark timeout; process-tree cleanup failed'));
            } else {
                try { process.kill(-child.pid, 'SIGKILL'); }
                catch (error) { if (error.code !== 'ESRCH') reject(error); }
            }
        }, timeoutMs);
        child.once('error', error => { clearTimeout(timer); reject(error); });
        child.once('close', code => {
            clearTimeout(timer);
            if (expired || code !== 0) reject(Error(`benchmark ${expired ? 'timed out' : `failed with exit ${code}`}; inspect preserved stderr log`));
            else resolve();
        });
    });
}
async function main(args) {
    if (args.length !== 5 || args.slice(2, 4).some(s => !/^[a-f0-9]{40}$/.test(s))) throw Error('usage: node scripts/release_performance_campaign.js BASELINE CANDIDATE BASELINE_SHA CANDIDATE_SHA NEW_OUTPUT_DIR');
    const [baseArg, candidateArg, baselineSha, candidateSha, outArg] = args;
    const baseline = path.resolve(baseArg), candidate = path.resolve(candidateArg), out = path.resolve(outArg);
    for (const binary of [baseline, candidate]) if (!fs.statSync(binary).isFile()) throw Error('binary must be a file');
    fs.mkdirSync(out); // Exclusive creation: never overwrite evidence.
    const environment = { schema: 'kujo.release-performance-environment/v1', recorded_at: new Date().toISOString(),
        baseline: { source_sha: baselineSha, binary_sha256: sha256(baseline), bytes: fs.statSync(baseline).size },
        candidate: { source_sha: candidateSha, binary_sha256: sha256(candidate), bytes: fs.statSync(candidate).size },
        corpus_sha: run('git', ['rev-parse', 'HEAD']).trim(), corpus_dirty: run('git', ['status', '--porcelain']).trim() !== '',
        driver_sha256: sha256('scripts/release_performance_campaign.js'),
        runtime_runner_sha256: sha256('scripts/runtime_measurement_release_bench.rs'),
        static_runner_sha256: sha256('scripts/static_analysis_release_bench.rs'),
        rustc: run('rustc', ['-Vv']).trim(), node: process.version,
        platform: process.platform, arch: process.arch, os_release: os.release(),
        cpu_models: [...new Set(os.cpus().map(c => c.model))], logical_cpus: os.cpus().length, total_memory_bytes: os.totalmem(),
        profile: 'release (caller-built; build command and cache state must accompany receipt)',
        power_storage: 'not controlled; internal regression evidence only',
        cache: 'two discarded process warmups per mode; not OS-cache-cold measurements',
        unsupported: ['total_heap_allocations', 'retained_heap_bytes', 'filesystem_syscall_counts', 'network_request_counts', 'child_process_counts'],
        github: process.env.GITHUB_RUN_ID ? { run_id: process.env.GITHUB_RUN_ID, attempt: process.env.GITHUB_RUN_ATTEMPT } : null };
    fs.writeFileSync(path.join(out, 'environment.json'), JSON.stringify(environment, null, 2) + '\n');
    const extension = process.platform === 'win32' ? '.exe' : '';
    for (const [label, source] of [['runtime', 'runtime_measurement_release_bench'], ['static', 'static_analysis_release_bench']]) {
        const runner = path.join(out, label + '-runner' + extension);
        const compilerLog = fs.openSync(path.join(out, label + '.compiler.log'), 'wx');
        try { run('rustc', ['-O', `scripts/${source}.rs`, '-o', runner], { stdio: ['ignore', compilerLog, compilerLog] }); }
        finally { fs.closeSync(compilerLog); }
        fs.copyFileSync(`scripts/${source}.rs`, path.join(out, source + '.rs'));
        const stdout = fs.openSync(path.join(out, label + '.json'), 'wx');
        const stderr = fs.openSync(path.join(out, label + '.stderr.log'), 'wx');
        try { await runBenchmark(runner, [baseline, candidate], ['ignore', stdout, stderr]); }
        finally { fs.closeSync(stdout); fs.closeSync(stderr); }
        fs.unlinkSync(runner);
    }
    const raw = file => JSON.parse(fs.readFileSync(path.join(out, file), 'utf8'));
    const summary = summarize(raw('runtime.json'), raw('static.json'));
    if (sha256(baseline) !== environment.baseline.binary_sha256 || sha256(candidate) !== environment.candidate.binary_sha256) throw Error('binary changed during campaign');
    fs.writeFileSync(path.join(out, 'summary.json'), JSON.stringify(summary, null, 2) + '\n');
    const hashes = Object.fromEntries(fs.readdirSync(out).map(file => [file, sha256(path.join(out, file))]));
    fs.writeFileSync(path.join(out, 'sha256.json'), JSON.stringify(hashes, null, 2) + '\n');
    console.log(JSON.stringify({ status: 'passed', runtime_workloads: summary.workloads.length, static_workloads: summary.static_workloads.length, evidence: out }));
}
module.exports = { stats, samples, summarize, main, runBenchmark };
if (require.main === module) {
    main(process.argv.slice(2)).catch(error => { console.error(error.message); process.exitCode = 1; });
}
