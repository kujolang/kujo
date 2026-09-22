# Install Kujo

Kujo 1.5.0 is the current stable native release. Choose the install that matches what
you need.

## Kujo and the core toolset

On macOS or Linux, this installs the current Kujo runtime, Kennel, and the core
Kujo tools without `sudo`:

```bash
curl -fsSL https://kujolang.ai/install.sh | bash
export PATH="$HOME/.local/bin:$PATH"
kujo --version
```

The installer downloads a prebuilt Kujo release, verifies its SHA-256 checksum,
and stores source snapshots and tools under `~/.kujo`. Commands go in
`~/.local/bin`. Change those locations with `--prefix` and `--bin-dir`.

Inspect the available profiles before installing more:

```bash
curl -fsSL https://kujolang.ai/install.sh -o install.sh
bash install.sh --list
bash install.sh --group ai
```

`--all` installs every profile. `--source` builds Kujo from source instead of
using a release binary. See [ecosystem installation](docs/ECOSYSTEM_INSTALL.md)
for profile contents and pinned multi-repository installs.

## Runtime only

Download the checksum file and matching archive from the
[v1.5.0 GitHub release](https://github.com/kujolang/kujo/releases/tag/v1.5.0).
Release assets use these names:

```text
kujo-v1.5.0-linux-x64.tar.gz
kujo-v1.5.0-linux-arm64.tar.gz
kujo-v1.5.0-macos-x64.tar.gz
kujo-v1.5.0-macos-arm64.tar.gz
kujo-v1.5.0-windows-x64.zip
```

Each archive has a matching `.sha256` file, and `checksums.txt` lists them all.
The signed release tag also binds these archive digests.
Read [release binaries](docs/RELEASE_BINARIES.md) for manual verification steps.

The independent npm channel currently remains at **1.4.0**. Publishing 1.5.0
was rejected by the registry and requires maintainer authorization review; use
the native archives when you need 1.5.0 APIs. Node.js 18 or newer can still
install the previous lifecycle-script-free runtime package:

```bash
npm install --global @kujolang/kujo-runtime@1.4.0
kujo --version
```

## Install Kennel

[Kennel](https://kennel.kujolang.ai/) manages Kujo packages and global Kujo
tools. It requires Kujo 1.4.0 or newer and currently supports macOS and Linux.

```bash
curl -fsSLO https://kennel.kujolang.ai/install.sh
sh install.sh
. "$HOME/.kennel/env"
kennel --version
```

The shell script starts a Kujo installer, which downloads and verifies the
Kennel release. It installs under `~/.kennel` and adds `~/.kennel/bin` to Bash,
Zsh, and POSIX shell profiles. Use `sh install.sh --no-modify-path` if you want
to manage `PATH` yourself.

Install a project dependency:

```bash
kennel init --name my-project
kennel add ability
kennel install
```

Install a command globally:

```bash
kennel tool install shipcheck
kennel tool list
```

Browse the official packages at
[kennel.kujolang.ai](https://kennel.kujolang.ai/). Public reads need no account.

## Build from source

Source builds need Git, Rust 1.86 or newer, and the platform's C build tools.

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
cargo build --release --locked
cargo install --path . --locked
kujo --version
```

For a faster contributor build:

```bash
cargo run -- run examples/hello.kujo
```

Platform notes:

- macOS: install Xcode Command Line Tools with `xcode-select --install`.
- Ubuntu or Debian: install `build-essential` if the linker is missing.
- Fedora: install `gcc` and `make` if the linker is missing.
- Arch Linux: install `base-devel` if the linker is missing.
- Windows: use the Rust MSVC toolchain. The supported release target is x64.

## Verify the install

```bash
kujo --version
kujo doctor --json
```

Then create a file named `hello.kujo`:

```kujo
print("Hello, Kujo!")
```

Run it:

```bash
kujo check hello.kujo
kujo run hello.kujo
```

## Update

Standalone Kujo installs from v1.3.0 onward can update the runtime from official
GitHub releases:

```bash
kujo upgrade --check
kujo upgrade
```

Select an exact version with `kujo upgrade 1.5.0`. Downgrades require
`--allow-downgrade`. The command verifies the release checksum and keeps a backup
of the previous executable.

Use the original manager for managed installs. The npm command below deliberately
selects the currently published 1.4.0 package; it does not install 1.5.0:

```bash
npm install --global @kujolang/kujo-runtime@1.4.0
cargo install --path . --locked --force
```

Update Kennel separately:

```bash
kennel self update
```

`kujo upgrade` does not update Kennel, ecosystem tools, or project dependencies.
Read [runtime upgrade and recovery](docs/RUNTIME_UPGRADE.md) for ownership checks,
backups, and Windows recovery.

## Troubleshooting

If `kujo` is not found, inspect the command path and add the right directory:

```bash
command -v kujo
export PATH="$HOME/.local/bin:$PATH"
```

For Kennel:

```bash
. "$HOME/.kennel/env"
command -v kennel
```

If a source build fails, first check the toolchain:

```bash
rustc --version
cargo --version
rustup update
```

Open a [GitHub issue](https://github.com/kujolang/kujo/issues) with your OS,
Rust version, full error, and steps to reproduce if the problem remains.

## Uninstall

Remove only the path used by your installer.

- npm: `npm uninstall --global @kujolang/kujo-runtime`
- Cargo: `cargo uninstall kujolang`
- ecosystem installer: remove `~/.kujo` and any Kujo command shims it placed in
  `~/.local/bin`
- Kennel: remove `~/.kennel` and the PATH line its installer added to your shell
  profile

Before deleting a command, use `command -v kujo` or `command -v kennel` so you do
not remove a different installation.

For contributor setup and test commands, continue with
[CONTRIBUTING.md](CONTRIBUTING.md).
