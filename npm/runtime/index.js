'use strict';

const fs = require('node:fs');
const path = require('node:path');
const RUNTIME_PACKAGE_VERSION = require('./package.json').version;

const TARGETS = Object.freeze({
  'darwin-arm64': Object.freeze({
    packageName: '@kujolang/kujo-darwin-arm64',
    binaryName: 'kujo'
  }),
  'darwin-x64': Object.freeze({
    packageName: '@kujolang/kujo-darwin-x64',
    binaryName: 'kujo'
  }),
  'linux-x64': Object.freeze({
    packageName: '@kujolang/kujo-linux-x64',
    binaryName: 'kujo'
  }),
  'linux-arm64': Object.freeze({
    packageName: '@kujolang/kujo-linux-arm64',
    binaryName: 'kujo'
  }),
  'win32-x64': Object.freeze({
    packageName: '@kujolang/kujo-win32-x64',
    binaryName: 'kujo.exe'
  })
});

function targetFor(platform = process.platform, arch = process.arch) {
  return TARGETS[`${platform}-${arch}`] || null;
}

class KujoRuntimeError extends Error {
  constructor(message, code, platform, arch, options = {}) {
    super(message, options);
    this.name = this.constructor.name;
    this.code = code;
    this.platform = platform;
    this.arch = arch;
  }
}

class UnsupportedPlatformError extends KujoRuntimeError {
  constructor(platform, arch) {
    const supported = Object.keys(TARGETS).sort().join(', ');
    super(
      `Kujo does not provide an npm runtime for ${platform}-${arch}. Supported targets: ${supported}.`,
      'KUJO_UNSUPPORTED_PLATFORM',
      platform,
      arch
    );
  }
}

class MissingPlatformPackageError extends KujoRuntimeError {
  constructor(platform, arch, target, cause) {
    super(
      [
        `Kujo's platform package ${target.packageName} is not installed.`,
        'Reinstall @kujolang/kujo-runtime without --omit=optional or --no-optional,',
        'and ensure your package manager is allowed to install optional dependencies.'
      ].join(' '),
      'KUJO_PLATFORM_PACKAGE_MISSING',
      platform,
      arch,
      { cause }
    );
    this.packageName = target.packageName;
  }
}

class BinaryNotFoundError extends KujoRuntimeError {
  constructor(platform, arch, packageName, binaryPath) {
    super(
      `Kujo's platform package ${packageName} does not contain the expected binary at ${binaryPath}. Reinstall the package and verify your npm cache.`,
      'KUJO_BINARY_MISSING',
      platform,
      arch
    );
    this.packageName = packageName;
    this.binaryPath = binaryPath;
  }
}

function selectedTarget(options = {}) {
  const platform = options.platform || process.platform;
  const arch = options.arch || process.arch;
  const target = targetFor(platform, arch);

  if (!target) {
    throw new UnsupportedPlatformError(platform, arch);
  }

  return { platform, arch, ...target };
}

function resolveRuntime(options = {}) {
  const { platform, arch, packageName, binaryName } = selectedTarget(options);
  const resolvePackage = options.resolvePackage || require.resolve;
  const fileExists = options.fileExists || fs.existsSync;
  const readManifest = options.readManifest || ((manifestPath) => JSON.parse(fs.readFileSync(manifestPath, 'utf8')));

  let manifestPath;
  try {
    manifestPath = resolvePackage(`${packageName}/package.json`);
  } catch (error) {
    throw new MissingPlatformPackageError(platform, arch, { packageName }, error);
  }

  const binaryPath = path.join(path.dirname(manifestPath), 'bin', binaryName);
  if (!fileExists(binaryPath)) {
    throw new BinaryNotFoundError(platform, arch, packageName, binaryPath);
  }

  const platformManifest = readManifest(manifestPath);
  return Object.freeze({
    platform,
    arch,
    packageName,
    packageVersion: RUNTIME_PACKAGE_VERSION,
    runtimeVersion: platformManifest.version,
    binaryName,
    binaryPath,
    source: 'bundled'
  });
}

function getKujoRuntimeInfo(options = {}) {
  return resolveRuntime(options);
}

function resolveKujoBinary(options = {}) {
  return resolveRuntime(options).binaryPath;
}

module.exports = {
  BinaryNotFoundError,
  KujoRuntimeError,
  MissingPlatformPackageError,
  TARGETS,
  UnsupportedPlatformError,
  getKujoRuntimeInfo,
  resolveKujoBinary,
  targetFor
};
