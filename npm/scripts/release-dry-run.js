#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {
  assertNoLifecycleScripts,
  copyDirectory,
  createTemporaryDirectory,
  pack,
  readJson,
  writeProvenanceMetadata
} = require('./package-utils');

const npmRoot = path.resolve(__dirname, '..');
const stagingRoot = createTemporaryDirectory('kujo-npm-dry-run-');
const outputRoot = path.join(stagingRoot, 'packed');
fs.mkdirSync(outputRoot);

const packages = [
  { source: path.join(npmRoot, 'runtime'), binary: null },
  { source: path.join(npmRoot, 'platforms', 'darwin-arm64'), binary: 'bin/kujo' },
  { source: path.join(npmRoot, 'platforms', 'darwin-x64'), binary: 'bin/kujo' },
  { source: path.join(npmRoot, 'platforms', 'linux-x64'), binary: 'bin/kujo' },
  { source: path.join(npmRoot, 'platforms', 'linux-arm64'), binary: 'bin/kujo' },
  { source: path.join(npmRoot, 'platforms', 'win32-x64'), binary: 'bin/kujo.exe' }
];

try {
  for (const entry of packages) {
    const destination = path.join(stagingRoot, path.basename(entry.source));
    copyDirectory(entry.source, destination);
    const manifestPath = path.join(destination, 'package.json');
    const manifest = readJson(manifestPath);
    assertNoLifecycleScripts(manifest, manifestPath);

    if (entry.binary) {
      const binaryPath = path.join(destination, entry.binary);
      fs.mkdirSync(path.dirname(binaryPath), { recursive: true });
      fs.writeFileSync(binaryPath, 'Kujo npm release dry-run fixture\n', { mode: 0o755 });
      writeProvenanceMetadata(path.join(destination, 'metadata.json'), {
        runtimeVersion: manifest.version,
        gitCommit: '0000000000000000000000000000000000000000',
        target: path.basename(entry.source),
        binaryPath
      });
    }

    const report = pack(destination, outputRoot, true);
    const packedPaths = new Set(report.files.map((file) => file.path));
    for (const required of manifest.files || []) {
      if (!packedPaths.has(required)) {
        throw new Error(`${manifest.name} dry run omitted required file ${required}`);
      }
    }
    process.stdout.write(`${manifest.name}@${manifest.version}: ${report.files.length} files\n`);
  }
} finally {
  fs.rmSync(stagingRoot, { recursive: true, force: true });
}
