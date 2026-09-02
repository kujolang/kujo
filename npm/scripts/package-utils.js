'use strict';

const fs = require('node:fs');
const crypto = require('node:crypto');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

function npmInvocation(args) {
  if (process.platform !== 'win32') return { command: 'npm', args };

  // Node does not execute .cmd shims with shell:false on Windows. Invoke npm's
  // JavaScript entry point directly so packaging remains shell-free and paths
  // cannot be reinterpreted as command text.
  const npmCli = path.join(path.dirname(process.execPath), 'node_modules', 'npm', 'bin', 'npm-cli.js');
  if (!fs.existsSync(npmCli)) {
    throw new Error(`npm CLI entry point not found beside Node.js: ${npmCli}`);
  }
  return { command: process.execPath, args: [npmCli, ...args] };
}

function copyDirectory(source, destination) {
  fs.cpSync(source, destination, { recursive: true });
}

function createTemporaryDirectory(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}

function writeJson(file, value) {
  fs.writeFileSync(file, `${JSON.stringify(value, null, 2)}\n`);
}

function assertNoLifecycleScripts(manifest, manifestPath) {
  const forbidden = ['preinstall', 'install', 'postinstall'];
  for (const name of forbidden) {
    if (manifest.scripts && Object.hasOwn(manifest.scripts, name)) {
      throw new Error(`${manifestPath} contains forbidden lifecycle script ${name}`);
    }
  }
}

function writeProvenanceMetadata(file, { runtimeVersion, gitCommit, target, binaryPath }) {
  if (!/^[0-9a-f]{40}$/i.test(gitCommit)) {
    throw new Error(`git commit must be a 40-character hexadecimal SHA: ${gitCommit}`);
  }
  const metadata = {
    runtimeVersion,
    gitCommit: gitCommit.toLowerCase(),
    target,
    sha256: crypto.createHash('sha256').update(fs.readFileSync(binaryPath)).digest('hex')
  };
  writeJson(file, metadata);
  return metadata;
}

function pack(directory, destination, dryRun = false) {
  const args = ['pack', '--json', '--ignore-scripts', '--pack-destination', destination];
  if (dryRun) args.push('--dry-run');
  const invocation = npmInvocation(args);
  const result = spawnSync(invocation.command, invocation.args, {
    cwd: directory,
    encoding: 'utf8',
    shell: false
  });
  if (result.status !== 0) {
    const detail = result.error?.message || result.stderr || result.stdout || 'unknown process error';
    throw new Error(`npm pack failed in ${directory}: ${detail}`);
  }
  const report = JSON.parse(result.stdout);
  if (!Array.isArray(report) || report.length !== 1) {
    throw new Error(`unexpected npm pack report for ${directory}`);
  }
  return report[0];
}

module.exports = {
  assertNoLifecycleScripts,
  copyDirectory,
  createTemporaryDirectory,
  pack,
  readJson,
  writeProvenanceMetadata,
  writeJson
};
