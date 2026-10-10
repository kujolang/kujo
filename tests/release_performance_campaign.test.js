'use strict';
const { test } = require('node:test');
const assert = require('node:assert/strict');
const { stats, samples, summarize, main } = require('../scripts/release_performance_campaign');

function fixture() {
    const timings = Array(11).fill(1000000);
    return [{ schema: 'kujo.measurement-benchmark/v1', samples_per_mode: 11,
        workloads: [{ name: 'probe', iterations: 100, baseline_ns: timings, disabled_ns: timings,
            enabled_ns: timings, enabled_reports: Array.from({ length: 11 }, () => ({
                schema: 'kujo.runtime-measurements/v1', runtime: 'vm', outcome: 'success',
                cpu_seconds: null, process_peak_rss_bytes: null, counters: { tasks_admitted: 0 }
            })) }] },
    { schema: 'kujo.static-analysis-benchmark/v1', samples_per_binary: 21,
        workloads: ['startup', 'project_check', 'lsp_diagnostics'].map(name => ({ name,
            baseline_ns: Array(21).fill(1000000), candidate_ns: Array(21).fill(1000000) })) }];
}
test('statistics retain outliers and use nearest-rank p90', () => {
    assert.deepEqual(stats([1, 2, 3]), { min: 1, median: 2, p90: 3, max: 3, mean: 2, standard_deviation: Math.sqrt(2 / 3) });
    assert.equal(stats([1, 1, 1, 1, 100]).max, 100);
    assert.throws(() => stats([]));
    assert.throws(() => samples([0], 1));
    assert.throws(() => samples([1], 2));
    assert.throws(() => samples([Number.MAX_SAFE_INTEGER + 1], 1));
});
test('A/A control reports zero paired delta and never fabricates unsupported resources', () => {
    const result = summarize(...fixture()).workloads[0];
    assert.equal(result.paired_candidate_delta_pct.median, 0);
    assert.equal(result.process_peak_rss_bytes, null);
    assert.equal(result.cpu_seconds, null);
    assert.equal(result.counters.tasks_admitted.median, 0);
});
test('one unavailable resource sample makes that aggregate unavailable', () => {
    const f = fixture();
    f[0].workloads[0].enabled_reports[0].cpu_seconds = 0.1;
    assert.equal(summarize(...f).workloads[0].cpu_seconds, null);
});
test('failed outcomes, corrupted counters and incomplete series cannot produce a receipt', () => {
    for (const mutate of [
        f => { f[0].workloads[0].enabled_reports[0].outcome = 'runtime_error'; },
        f => { f[0].workloads[0].enabled_reports[0].counters.tasks_admitted = -1; },
        f => { f[0].workloads[0].enabled_reports.pop(); },
        f => { f[0].workloads[0].baseline_ns.pop(); },
        f => { f[1].workloads.pop(); },
        f => { f[0].schema = 'unknown'; },
        f => { f[0].workloads.push(f[0].workloads[0]); },
    ]) {
        const f = fixture(); mutate(f); assert.throws(() => summarize(...f));
    }
});
test('CLI rejects incomplete provenance before creating output', () => {
    assert.throws(() => main([]), /usage/);
    assert.throws(() => main(['base', 'candidate', 'main', 'main', 'output']), /usage/);
});
