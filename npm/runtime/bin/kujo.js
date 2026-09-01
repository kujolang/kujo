#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');
const { resolveKujoBinary } = require('..');

function fail(message) {
  process.stderr.write(`kujo: ${message}\n`);
  process.exitCode = 1;
}

let binaryPath;
try {
  binaryPath = resolveKujoBinary();
} catch (error) {
  fail(error.message);
  return;
}

const result = spawnSync(binaryPath, process.argv.slice(2), {
  stdio: 'inherit',
  shell: false,
  windowsHide: false
});

if (result.error) {
  fail(`failed to start the bundled runtime: ${result.error.message}`);
} else if (result.signal) {
  process.kill(process.pid, result.signal);
} else {
  process.exitCode = result.status === null ? 1 : result.status;
}
