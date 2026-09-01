export type KujoRuntimeSource = 'bundled';
export type KujoRuntimePlatform = 'darwin' | 'linux' | 'win32';
export type KujoRuntimeArch = 'arm64' | 'x64';

export interface KujoRuntimeInfo {
  platform: KujoRuntimePlatform;
  arch: KujoRuntimeArch;
  packageName: string;
  packageVersion: string;
  runtimeVersion: string;
  binaryName: 'kujo' | 'kujo.exe';
  binaryPath: string;
  source: KujoRuntimeSource;
}

export interface KujoRuntimeOptions {
  platform?: KujoRuntimePlatform;
  arch?: KujoRuntimeArch;
}

export class KujoRuntimeError extends Error {
  readonly code: string;
  readonly platform: string;
  readonly arch: string;
}

export class UnsupportedPlatformError extends KujoRuntimeError {}

export class MissingPlatformPackageError extends KujoRuntimeError {
  readonly packageName: string;
}

export class BinaryNotFoundError extends KujoRuntimeError {
  readonly packageName: string;
  readonly binaryPath: string;
}

export function getKujoRuntimeInfo(options?: KujoRuntimeOptions): KujoRuntimeInfo;
export function resolveKujoBinary(options?: KujoRuntimeOptions): string;
