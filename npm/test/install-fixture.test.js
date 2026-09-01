'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');
const { targetFor } = require('../runtime');
const { copyDirectory, pack, writeProvenanceMetadata } = require('../scripts/package-utils');

const npmCommand = process.platform === 'win32' ? 'npm.cmd' : 'npm';

test('packed runtime installs from local fixtures with lifecycle scripts disabled', () => {
  const target = targetFor();
  if (!target) return;

  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'kujo-npm-install-'));
  try {
    const npmRoot = path.resolve(__dirname, '..');
    const packageRoot = path.join(root, 'packages');
    const packedRoot = path.join(root, 'packed');
    const projectRoot = path.join(root, 'project');
    const runtimeRoot = path.join(packageRoot, 'runtime');
    const targetDirectory = target.packageName.replace('@kujolang/kujo-', '');
    const nativeRoot = path.join(packageRoot, targetDirectory);
    fs.mkdirSync(packedRoot, { recursive: true });
    fs.mkdirSync(projectRoot, { recursive: true });
    copyDirectory(path.join(npmRoot, 'runtime'), runtimeRoot);
    copyDirectory(path.join(npmRoot, 'platforms', targetDirectory), nativeRoot);
    const binary = path.join(nativeRoot, 'bin', target.binaryName);
    fs.mkdirSync(path.dirname(binary), { recursive: true });
    fs.writeFileSync(binary, 'local install fixture\n', { mode: 0o755 });
    writeProvenanceMetadata(path.join(nativeRoot, 'metadata.json'), {
      runtimeVersion: '1.2.0',
      gitCommit: '0000000000000000000000000000000000000000',
      target: targetDirectory,
      binaryPath: binary
    });

    const nativePack = pack(nativeRoot, packedRoot);
    const runtimePack = pack(runtimeRoot, packedRoot);
    fs.writeFileSync(path.join(projectRoot, 'package.json'), '{"private":true}\n');
    const install = spawnSync(npmCommand, [
      'install',
      '--ignore-scripts',
      '--offline',
      '--no-audit',
      '--no-fund',
      '--package-lock=false',
      path.join(packedRoot, nativePack.filename),
      path.join(packedRoot, runtimePack.filename)
    ], { cwd: projectRoot, encoding: 'utf8', shell: false });
    assert.equal(install.status, 0, install.stderr || install.stdout);

    const installedRuntime = JSON.parse(fs.readFileSync(
      path.join(projectRoot, 'node_modules', '@kujolang', 'kujo-runtime', 'package.json')
    ));
    assert.equal(installedRuntime.scripts, undefined);
    assert.equal(fs.existsSync(path.join(
      projectRoot,
      'node_modules',
      ...target.packageName.split('/'),
      'bin',
      target.binaryName
    )), true);
    const installedMetadata = JSON.parse(fs.readFileSync(path.join(
      projectRoot,
      'node_modules',
      ...target.packageName.split('/'),
      'metadata.json'
    )));
    assert.equal(installedMetadata.runtimeVersion, '1.2.0');
    assert.equal(installedMetadata.target, targetDirectory);
    assert.match(installedMetadata.gitCommit, /^[0-9a-f]{40}$/);
    assert.match(installedMetadata.sha256, /^[0-9a-f]{64}$/);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
