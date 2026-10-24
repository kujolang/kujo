#!/usr/bin/env bash
# Bootstrap tooling is shell because Kujo is not installed yet. Benchmark
# orchestration stays in the existing Kujo runners. Run as an ordinary user.
set -euo pipefail

if [[ ${1:-} == --help ]]; then
    printf '%s\n' 'Usage: bash prepare-linux.sh /absolute/new/workspace [build-jobs]' \
        'Requires Linux x86_64, git, rustup, clang/clang++, cmake, make, pkg-config, sha256sum.' \
        'Builds original and candidate Kujo, Goose and Rust with pinned sources and records provenance.' \
        'Does not provision cloud resources, install OS packages, or run benchmarks.'
    exit 0
fi
[[ $# -ge 1 && $# -le 2 ]] || { printf '%s\n' 'Expected workspace and optional build-jobs.' >&2; exit 2; }
workspace=$1
jobs=${2:-2}
[[ $workspace == /* && ! -e $workspace ]] || { printf '%s\n' 'Workspace must be an absolute, nonexistent path.' >&2; exit 2; }
[[ $jobs =~ ^[1-9][0-9]*$ ]] || { printf '%s\n' 'Build jobs must be a positive integer.' >&2; exit 2; }
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || { printf '%s\n' 'This setup targets Linux x86_64 only.' >&2; exit 2; }
[[ -z ${RUSTFLAGS:-} && -z ${CARGO_ENCODED_RUSTFLAGS:-} ]] || { printf '%s\n' 'Unset ambient Rust optimization flags before preparing benchmarks.' >&2; exit 2; }
for tool in git rustup clang clang++ cmake make pkg-config sha256sum; do
    command -v "$tool" >/dev/null || { printf 'Missing prerequisite: %s\n' "$tool" >&2; exit 2; }
done

kujo_baseline_revision=f037705190bb6ef765238f9c091086eae0e97485
kujo_revision=aa3007194ebb446ddbe6a91a90d7a8f55e8ee6b9
arena_revision=3c9f84ddaeec834c0631747cb789e8cfd602f993
goose_revision=ea1f368c109c083a2161580ff1d74c70200b6cc8
toolchain=1.96.0
mkdir -p "$workspace/logs" "$workspace/results"
# Preserve stdout/stderr on the server, including failed build diagnostics.
exec > >(tee "$workspace/logs/prepare.log") 2>&1
trap 'printf "Preparation failed. Retain logs; no ready marker was written.\n" >&2' ERR
rustup toolchain install "$toolchain" --profile minimal
export RUSTUP_TOOLCHAIN=$toolchain
export CC=clang CXX=clang++

checkout() {
    local url=$1 destination=$2 revision=$3
    git clone "$url" "$destination"
    git -C "$destination" checkout --detach "$revision"
    [[ $(git -C "$destination" rev-parse HEAD) == "$revision" ]]
}
checkout https://github.com/kujolang/kujo.git "$workspace/kujo" "$kujo_revision"
git -C "$workspace/kujo" worktree add --detach "$workspace/kujo-baseline" "$kujo_baseline_revision"
[[ $(git -C "$workspace/kujo-baseline" rev-parse HEAD) == "$kujo_baseline_revision" ]]
checkout https://github.com/kostya/LangArena.git "$workspace/arena" "$arena_revision"
checkout https://github.com/aardappel/goose.git "$workspace/goose" "$goose_revision"

for source in kujo-baseline kujo; do
    (
        cd "$workspace/$source"
        cargo build --locked --release --bin kujo --target-dir "$workspace/$source/target" -j "$jobs"
    )
done
cmake -S "$workspace/goose" -B "$workspace/goose/build" \
    -DCMAKE_BUILD_TYPE=Release -DGOOSE_LIBTCC=OFF -DGOOSE_GFX=OFF \
    -DGOOSE_AUDIO=OFF -DGOOSE_PHYSICS=OFF -DGOOSE_UI=OFF
cmake --build "$workspace/goose/build" --target goose -j "$jobs"
export PATH="$workspace/goose/build:$PATH"
(
    cd "$workspace/arena/goose"
    # The pinned Makefile spells `cc` literally and ignores a CC override.
    # Execute its prod recipe explicitly so the recorded compiler is the one used.
    mkdir -p target
    goose -O2 --standalone -o target/prod.c src/main.goose
    clang -O2 -w target/prod.c -o target/benchmark -lm -pthread
)
(
    cd "$workspace/arena/rust"
    cargo build --locked --release --target-dir "$workspace/arena/rust/target" -j "$jobs"
)

{
    date -u
    uname -a
    cat /etc/os-release
    lscpu
    free -h
    df -h "$workspace"
    rustc -Vv
    cargo -V
    clang --version
    cmake --version
    printf 'Kujo original %s\nKujo candidate %s\nLangArena %s\nGoose %s\nBuild jobs %s\n' "$kujo_baseline_revision" "$kujo_revision" "$arena_revision" "$goose_revision" "$jobs"
} > "$workspace/results/host-and-build.txt"
sha256sum "$workspace/kujo-baseline/target/release/kujo" \
    "$workspace/kujo/target/release/kujo" \
    "$workspace/arena/goose/target/benchmark" \
    "$workspace/arena/rust/target/release/benchmarks" \
    "$workspace/kujo/benchmarks/langarena/port.kujo" \
    "$workspace/kujo/benchmarks/langarena/test.json" \
    "$workspace/kujo/benchmarks/langarena/run.json" > "$workspace/results/SHA256SUMS"
printf '%s\n' 'Builds complete. Correctness, memory, timing and production gates are still pending.' > "$workspace/BUILT_NOT_VERIFIED"
printf 'Build artifacts ready in %s; no benchmark acceptance claimed.\n' "$workspace"
