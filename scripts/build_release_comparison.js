// Run under the runner's native shell: Git Bash shadows native Windows Perl,
// which the locked vendored OpenSSL build requires. No shell command strings.
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
function logged(command, args, file) {
    const log = fs.openSync(file, 'wx');
    try {
        fs.writeSync(log, JSON.stringify({ command, args }) + '\n');
        const result = spawnSync(command, args, { stdio: ['ignore', log, log] });
        if (result.error || result.status !== 0) throw Error(`${command} failed; inspect ${file}`);
    } finally { fs.closeSync(log); }
}
function main() {
    const baseline = process.env.SOURCE_SHA;
    const candidate = process.env.CANDIDATE_SHA || baseline;
    if (![baseline, candidate].every(sha => /^[a-f0-9]{40}$/.test(sha || '')) || !process.env.RUNNER_TEMP) throw Error('exact source SHAs and RUNNER_TEMP are required');
    const root = path.join(process.env.RUNNER_TEMP, 'perf-runtime');
    const binary = process.platform === 'win32' ? 'kujo.exe' : 'kujo';
    if (process.platform === 'win32') logged('perl', ['-MIPC::Cmd', '-e', 'print "$^X\\n"'], 'windows-perl.log');
    const args = ['build', '--manifest-path', path.join(root, 'Cargo.toml'), '--release', '--locked', '--bin', 'kujo'];
    logged('cargo', args, 'baseline-build.log');
    fs.copyFileSync(path.join(root, 'target', 'release', binary), path.join(process.env.RUNNER_TEMP, 'baseline-' + binary), fs.constants.COPYFILE_EXCL);
    if (candidate !== baseline) {
        logged('git', ['-C', root, 'checkout', '--detach', candidate], 'candidate-checkout.log');
        logged('cargo', args, 'candidate-build.log');
    }
    console.log('Locked release binaries built; build logs preserved.');
}
try { main(); } catch (error) { console.error(error.message); process.exitCode = 1; }
