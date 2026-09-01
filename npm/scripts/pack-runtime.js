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
  writeJson
} = require('./package-utils');

function option(name) {
  const index = process.argv.indexOf(name);
  if (index < 0 || !process.argv[index + 1]) throw new Error(`missing required option ${name}`);
  return process.argv[index + 1];
}

const version = option('--version');
const output = path.resolve(option('--output'));
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`invalid npm version ${version}`);
}

const stagingRoot = createTemporaryDirectory('kujo-npm-runtime-');
try {
  const packageRoot = path.join(stagingRoot, 'runtime');
  copyDirectory(path.resolve(__dirname, '..', 'runtime'), packageRoot);
  const manifestPath = path.join(packageRoot, 'package.json');
  const manifest = readJson(manifestPath);
  assertNoLifecycleScripts(manifest, manifestPath);
  manifest.version = version;
  for (const packageName of Object.keys(manifest.optionalDependencies)) {
    manifest.optionalDependencies[packageName] = version;
  }
  writeJson(manifestPath, manifest);
  fs.mkdirSync(output, { recursive: true });
  const report = pack(packageRoot, output, false);
  process.stdout.write(`${path.join(output, report.filename)}\n`);
} finally {
  fs.rmSync(stagingRoot, { recursive: true, force: true });
}
