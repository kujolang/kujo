#!/usr/bin/env bash
# Local, unpublished release artifact rehearsal. No registry/tag/release writes.
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$#" == 1 ]] || { echo 'Usage: bash scripts/build_local_rc.sh NEW_OUTPUT_DIRECTORY' >&2; exit 2; }
[[ -z "$(git status --porcelain)" ]] || { echo 'RC source must be clean' >&2; exit 1; }
[[ ! -e "$1" ]] || { echo 'Output must be a new directory' >&2; exit 1; }
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Invalid Cargo package version: $version" >&2; exit 1; }
case "$(uname -s)-$(uname -m)" in
  Darwin-x86_64) platform=macos-x64 ;;
  *) echo 'This local rehearsal currently supports macos-x64 only; use the platform release matrix elsewhere.' >&2; exit 1 ;;
esac
cargo build --release --locked
[[ "$(target/release/kujo --version)" == "kujo $version" ]]
mkdir -p "$1"
out="$(cd "$1" && pwd)"
mkdir "$out/native"
cp target/release/kujo "$out/native/kujo"
tar -czf "$out/kujo-v$version-$platform.tar.gz" -C "$out/native" kujo
git archive --format=tar --prefix="kujo-v$version/" HEAD | gzip -n > "$out/kujo-v$version-source.tar.gz"
for archive in "$out"/*.tar.gz; do
  (cd "$out" && shasum -a 256 "$(basename "$archive")" > "$(basename "$archive").sha256")
done
node - "$out" "$version" "$platform" <<'JS'
const fs=require('node:fs'),crypto=require('node:crypto'),cp=require('node:child_process'),path=require('node:path');
const [out,version,platform]=process.argv.slice(2);
const sha=f=>crypto.createHash('sha256').update(fs.readFileSync(f)).digest('hex');
const run=(c,args)=>cp.execFileSync(c,args,{encoding:'utf8'}).trim();
const value={schema:'kujo.local-rc-artifacts/v1',version,source_commit:run('git',['rev-parse','HEAD']),platform,rust:run('rustc',['--version']),cargo:run('cargo',['--version']),cargo_lock_sha256:sha('Cargo.lock'),binary_sha256:sha(path.join(out,'native/kujo')),archives:{},publication_authorized:false};
for(const name of fs.readdirSync(out).filter(n=>n.endsWith('.tar.gz')))value.archives[name]=sha(path.join(out,name));
fs.writeFileSync(path.join(out,'artifacts.json'),JSON.stringify(value,null,2)+'\n');
JS
