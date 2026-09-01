'use strict';

const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const { TARGETS } = require('../runtime');
const { writeProvenanceMetadata } = require('../scripts/package-utils');

const npmRoot = path.resolve(__dirname, '..');
const main = JSON.parse(fs.readFileSync(path.join(npmRoot, 'runtime', 'package.json')));
const cargoVersion = fs.readFileSync(path.resolve(npmRoot, '..', 'Cargo.toml'), 'utf8')
  .match(/^version = "([^"]+)"/m)[1];

test('all packages are lifecycle-script-free and version-aligned', () => {
  assert.equal(main.version, cargoVersion);
  const manifests = [main];
  for (const directory of fs.readdirSync(path.join(npmRoot, 'platforms'))) {
    const manifestPath = path.join(npmRoot, 'platforms', directory, 'package.json');
    if (fs.existsSync(manifestPath)) manifests.push(JSON.parse(fs.readFileSync(manifestPath)));
  }
  for (const manifest of manifests) {
    assert.equal(manifest.version, main.version);
    for (const lifecycle of ['preinstall', 'install', 'postinstall']) {
      assert.equal(manifest.scripts?.[lifecycle], undefined, `${manifest.name} has ${lifecycle}`);
    }
  }
});

test('optional dependencies exactly match resolver packages and versions', () => {
  const expected = Object.fromEntries(
    Object.values(TARGETS).map(({ packageName }) => [packageName, main.version])
  );
  assert.deepEqual(main.optionalDependencies, expected);
});

test('platform constraints agree with their resolver keys', () => {
  for (const [key, target] of Object.entries(TARGETS)) {
    const directory = target.packageName.replace('@kujolang/kujo-', '');
    const manifest = JSON.parse(fs.readFileSync(path.join(npmRoot, 'platforms', directory, 'package.json')));
    const [platform, arch] = key.split('-');
    assert.deepEqual(manifest.os, [platform]);
    assert.deepEqual(manifest.cpu, [arch]);
    assert.ok(manifest.files.includes('metadata.json'));
  }
});

test('writes deterministic provenance with a binary SHA-256', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'kujo-provenance-test-'));
  try {
    const binaryPath = path.join(root, 'kujo');
    const metadataPath = path.join(root, 'metadata.json');
    const contents = Buffer.from('deterministic binary fixture\n');
    fs.writeFileSync(binaryPath, contents);
    const metadata = writeProvenanceMetadata(metadataPath, {
      runtimeVersion: '1.2.0',
      gitCommit: 'ABCDEF0123456789ABCDEF0123456789ABCDEF01',
      target: 'linux-x64',
      binaryPath
    });
    assert.deepEqual(metadata, {
      runtimeVersion: '1.2.0',
      gitCommit: 'abcdef0123456789abcdef0123456789abcdef01',
      target: 'linux-x64',
      sha256: crypto.createHash('sha256').update(contents).digest('hex')
    });
    assert.equal(fs.readFileSync(metadataPath, 'utf8'), `${JSON.stringify(metadata, null, 2)}\n`);
    assert.throws(
      () => writeProvenanceMetadata(metadataPath, {
        runtimeVersion: '1.2.0',
        gitCommit: 'not-a-full-commit',
        target: 'linux-x64',
        binaryPath
      }),
      /40-character hexadecimal SHA/
    );
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
