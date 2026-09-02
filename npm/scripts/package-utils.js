'use strict';

const fs = require('node:fs');
const crypto = require('node:crypto');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';

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
  const result = spawnSync(npmCommand, args, {
    cwd: directory,
    encoding: 'utf8',
    // Windows cannot execute npm.cmd directly through spawnSync without a
    // command shell. All arguments here are fixed packaging inputs.
    shell: process.platform === 'win32'
  });
  if (result.status !== 0) {
    const detail = result.error?.message || result.stderr || result.stdout || 'unknown error';
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
