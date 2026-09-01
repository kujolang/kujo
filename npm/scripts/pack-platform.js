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
  writeProvenanceMetadata,
  writeJson
} = require('./package-utils');

function option(name) {
  const index = process.argv.indexOf(name);
  if (index < 0 || !process.argv[index + 1]) {
    throw new Error(`missing required option ${name}`);
  }
  return process.argv[index + 1];
}

const npmRoot = path.resolve(__dirname, '..');
const target = option('--target');
const binarySource = path.resolve(option('--binary'));
const version = option('--version');
const gitCommit = option('--git-commit');
const output = path.resolve(option('--output'));
const allowedTargets = new Set(['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64', 'win32-x64']);

if (!allowedTargets.has(target)) throw new Error(`unsupported npm target ${target}`);
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/.test(version)) {
  throw new Error(`invalid npm version ${version}`);
}
if (!fs.statSync(binarySource).isFile()) throw new Error(`binary not found: ${binarySource}`);

const stagingRoot = createTemporaryDirectory(`kujo-npm-${target}-`);
try {
  const packageRoot = path.join(stagingRoot, target);
  copyDirectory(path.join(npmRoot, 'platforms', target), packageRoot);
  const manifestPath = path.join(packageRoot, 'package.json');
  const manifest = readJson(manifestPath);
  assertNoLifecycleScripts(manifest, manifestPath);
  manifest.version = version;
  writeJson(manifestPath, manifest);

  const binaryName = target === 'win32-x64' ? 'kujo.exe' : 'kujo';
  const binaryDestination = path.join(packageRoot, 'bin', binaryName);
  fs.mkdirSync(path.dirname(binaryDestination), { recursive: true });
  fs.copyFileSync(binarySource, binaryDestination);
  fs.chmodSync(binaryDestination, 0o755);
  writeProvenanceMetadata(path.join(packageRoot, 'metadata.json'), {
    runtimeVersion: version,
    gitCommit,
    target,
    binaryPath: binaryDestination
  });
  fs.mkdirSync(output, { recursive: true });

  const report = pack(packageRoot, output, false);
  if (!report.files.some((file) => file.path === `bin/${binaryName}`)) {
    throw new Error(`${manifest.name} package omitted bin/${binaryName}`);
  }
  if (!report.files.some((file) => file.path === 'metadata.json')) {
    throw new Error(`${manifest.name} package omitted metadata.json`);
  }
  process.stdout.write(`${path.join(output, report.filename)}\n`);
} finally {
  fs.rmSync(stagingRoot, { recursive: true, force: true });
}
