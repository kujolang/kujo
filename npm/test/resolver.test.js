'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { spawnSync } = require('node:child_process');
const test = require('node:test');
const {
  BinaryNotFoundError,
  MissingPlatformPackageError,
  TARGETS,
  UnsupportedPlatformError,
  getKujoRuntimeInfo,
  resolveKujoBinary,
  targetFor
} = require('../runtime');

test('maps every supported Node target to an allow-listed package', () => {
  assert.deepEqual(Object.keys(TARGETS).sort(), [
    'darwin-arm64',
    'darwin-x64',
    'linux-arm64',
    'linux-x64',
    'win32-x64'
  ]);
  assert.equal(targetFor('linux', 'x64').packageName, '@kujolang/kujo-linux-x64');
});

test('rejects unsupported targets before package resolution', () => {
  assert.throws(
    () => resolveKujoBinary({ platform: 'freebsd', arch: 'arm64', resolvePackage: assert.fail }),
    (error) => error instanceof UnsupportedPlatformError &&
      error.code === 'KUJO_UNSUPPORTED_PLATFORM' &&
      error.platform === 'freebsd' &&
      error.arch === 'arm64'
  );
});

test('returns the complete bundled runtime contract', () => {
  assert.deepEqual(getKujoRuntimeInfo({
    platform: 'linux',
    arch: 'arm64',
    resolvePackage: () => path.join('/fixture', 'node_modules', '@kujolang', 'kujo-linux-arm64', 'package.json'),
    fileExists: () => true,
    readManifest: () => ({ version: '1.2.0' })
  }), {
    platform: 'linux',
    arch: 'arm64',
    packageName: '@kujolang/kujo-linux-arm64',
    packageVersion: require('../runtime/package.json').version,
    runtimeVersion: '1.2.0',
    binaryName: 'kujo',
    binaryPath: path.join('/fixture', 'node_modules', '@kujolang', 'kujo-linux-arm64', 'bin', 'kujo'),
    source: 'bundled'
  });
});

test('resolves only the selected package manifest and fixed binary path', () => {
  const requested = [];
  const binary = resolveKujoBinary({
    platform: 'darwin',
    arch: 'arm64',
    resolvePackage(specifier) {
      requested.push(specifier);
      return path.join('/fixture', 'node_modules', '@kujolang', 'kujo-darwin-arm64', 'package.json');
    },
    fileExists: () => true,
    readManifest: () => ({ version: '1.2.0' })
  });
  assert.deepEqual(requested, ['@kujolang/kujo-darwin-arm64/package.json']);
  assert.equal(binary, path.join('/fixture', 'node_modules', '@kujolang', 'kujo-darwin-arm64', 'bin', 'kujo'));
});

test('reports a corrupt platform package with a typed error', () => {
  assert.throws(
    () => resolveKujoBinary({
      platform: 'darwin',
      arch: 'x64',
      resolvePackage: () => path.join('/fixture', 'package.json'),
      fileExists: () => false
    }),
    (error) => error instanceof BinaryNotFoundError &&
      error.code === 'KUJO_BINARY_MISSING' &&
      error.packageName === '@kujolang/kujo-darwin-x64'
  );
});

test('reports omitted optional dependencies with a remediation', () => {
  assert.throws(
    () => resolveKujoBinary({
      platform: 'linux',
      arch: 'x64',
      resolvePackage() {
        throw Object.assign(new Error('not found'), { code: 'MODULE_NOT_FOUND' });
      }
    }),
    (error) => error instanceof MissingPlatformPackageError &&
      error.code === 'KUJO_PLATFORM_PACKAGE_MISSING' &&
      error.packageName === '@kujolang/kujo-linux-x64' &&
      /without --omit=optional/.test(error.message)
  );
});

test('launcher forwards literal arguments without a shell and preserves exit status', {
  skip: process.platform === 'win32'
}, () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'kujo-launcher-test-'));
  try {
    const mainRoot = path.join(root, 'node_modules', '@kujolang', 'kujo-runtime');
    fs.cpSync(path.resolve(__dirname, '..', 'runtime'), mainRoot, { recursive: true });
    const target = targetFor();
    assert.ok(target, 'test host must be an npm-supported target');
    const nativeRoot = path.join(root, 'node_modules', ...target.packageName.split('/'));
    fs.mkdirSync(path.join(nativeRoot, 'bin'), { recursive: true });
    fs.writeFileSync(
      path.join(nativeRoot, 'package.json'),
      JSON.stringify({ name: target.packageName, version: '1.2.0' })
    );
    const binary = path.join(nativeRoot, 'bin', target.binaryName);
    fs.writeFileSync(
      binary,
      '#!/usr/bin/env node\nprocess.stdout.write(JSON.stringify(process.argv.slice(2))); process.exitCode = 23;\n',
      { mode: 0o755 }
    );

    const marker = path.join(root, 'shell-was-used');
    const literal = `$(touch ${marker})`;
    const result = spawnSync(process.execPath, [path.join(mainRoot, 'bin', 'kujo.js'), literal, 'two words'], {
      encoding: 'utf8',
      shell: false
    });
    assert.equal(result.status, 23);
    assert.deepEqual(JSON.parse(result.stdout), [literal, 'two words']);
    assert.equal(fs.existsSync(marker), false);
  } finally {
    fs.rmSync(root, { recursive: true, force: true });
  }
});
