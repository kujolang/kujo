# Installation Guide

Return to [README](README.md) for full language overview.

> Looking for how to get Kujo installed on your system? You're in the right place.

Kujo is the Kujo core language/runtime built with Rust. You can install Kujo either from prebuilt release artifacts (recommended for users) or by building from source with Cargo.

If another `kujo` command is already on your machine, make sure you are using the binary from this repository so you do not confuse it with unrelated tools that share the same name.

---

## Prerequisites

Kujo requires **Rust 1.86+** to build from source.

Minimum release validation assumptions for current supported install flow:

- Rust stable `1.86+`
- Linux (`ubuntu-latest` baseline)
- macOS (`macos-latest` baseline)

See `docs/RELEASE_ARTIFACT_VALIDATION.md` for cross-platform clean-environment validation and checksum verification flow.

### Install Rust

**macOS / Linux:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**Windows:**  
Download and run the installer from [rustup.rs](https://rustup.rs/)

Verify Rust installation:
```bash
rustc --version
cargo --version
```

---

## Install From Prebuilt Release Artifacts (Recommended)

Use this path when consuming a tagged Kujo release.

Set the release tag and detect platform:

```bash
KUJO_VERSION="v1.0.0"

if [[ "$(uname -s)" == "Darwin" ]]; then
   KUJO_OS="macos"
else
   KUJO_OS="linux"
fi

KUJO_ARCH="$(uname -m)"
if [[ "${KUJO_ARCH}" == "aarch64" ]]; then
   KUJO_ARCH="arm64"
fi

KUJO_TARGET="${KUJO_OS}-${KUJO_ARCH}"
```

Download binary archive and checksum:

```bash
BASE_URL="https://github.com/kujolang/kujo/releases/download/${KUJO_VERSION}"
ARCHIVE="kujo-${KUJO_VERSION}-${KUJO_TARGET}.tar.gz"

curl -sSfL "${BASE_URL}/${ARCHIVE}" -o "${ARCHIVE}"
curl -sSfL "${BASE_URL}/${ARCHIVE}.sha256" -o "${ARCHIVE}.sha256"
```

Verify checksum:

```bash
if command -v sha256sum >/dev/null 2>&1; then
   sha256sum -c "${ARCHIVE}.sha256"
else
   shasum -a 256 -c "${ARCHIVE}.sha256"
fi
```

Install and verify commands:

```bash
mkdir -p ~/.local/bin
tar -xzf "${ARCHIVE}"
cp kujo ~/.local/bin/kujo
chmod +x ~/.local/bin/kujo

export PATH="$HOME/.local/bin:$PATH"
kujo --version
kujo run examples/hello.kujo
kujo lsp --help
```

## Build from Source

### 1. Clone the Repository

```bash
git clone https://github.com/kujolang/kujo.git
cd kujo
```

### 2. Build the Project

**Development build** (faster compilation, slower runtime):
```bash
cargo build
```

**Release build** (optimized, recommended for daily use):
```bash
cargo build --release
```

### 3. Run Kujo

**Without installing** (from project directory):
```bash
# Development build
cargo run -- run examples/hello.kujo

# Release build
./target/release/kujo run examples/hello.kujo
```

### 4. Install System-Wide (Optional)

**macOS / Linux:**
```bash
cargo install --path .
# Or manually copy the binary
sudo cp target/release/kujo /usr/local/bin/
```

**Windows (PowerShell as Administrator):**
```powershell
cargo install --path .
# Or manually copy the binary to a directory in your PATH
```

Verify installation:
```bash
kujo --version
```

---

## Platform-Specific Notes

### macOS

**Supported versions**: macOS 10.15 (Catalina) or later  
**Architectures**: Intel (x86_64) and Apple Silicon (ARM64)

If you encounter permissions issues:
```bash
sudo chown -R $(whoami) /usr/local/bin
```

### Linux

**Tested distributions**: Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch Linux

**Dependencies**: None required beyond Rust toolchain

If you need to install to a user directory:
```bash
mkdir -p ~/.local/bin
cp target/release/kujo ~/.local/bin/
# Add to PATH in ~/.bashrc or ~/.zshrc:
export PATH="$HOME/.local/bin:$PATH"
```

### Windows

**Supported versions**: Windows 10 or later  
**Architectures**: x64

**Common issues**:
- If you get "VCRUNTIME140.dll missing" errors, install the [Visual C++ Redistributable](https://aka.ms/vs/17/release/vc_redist.x64.exe)
- Ensure your PATH includes the directory where `kujo.exe` is located

---

## Quick Start

Once installed, you can run Kujo programs:

```bash
# Run a script
kujo run examples/hello.kujo

# Run tests
kujo test

# Update test snapshots
kujo test --update
```

---

## Future Installation Methods

The following installation methods are planned for future releases:

### Homebrew (macOS / Linux)
```bash
# Coming soon
brew tap kujolang/tap
brew install kujo
```

### Scoop (Windows)
```powershell
# Coming soon
scoop bucket add kujo https://github.com/kujolang/scoop-bucket
scoop install kujo
```

### Package Managers
- **apt** (Ubuntu/Debian): Planned
- **dnf** (Fedora): Planned
- **pacman** (Arch): Planned
- **winget** (Windows): Planned

---

## Troubleshooting

### Build Failures

**"linker \`cc\` not found"** (Linux)
```bash
# Ubuntu/Debian
sudo apt install build-essential

# Fedora
sudo dnf install gcc

# Arch
sudo pacman -S base-devel
```

**"failed to run custom build command"**
- Ensure you have the latest Rust version: `rustup update`
- Clean and rebuild: `cargo clean && cargo build --release`

### Runtime Issues

**"command not found: kujo"**
- Verify the binary is in your PATH
- Try running with full path: `/usr/local/bin/kujo` or `./target/release/kujo`

**Slow compilation**
- Use `cargo build` for development (faster compile, slower runtime)
- Use `cargo build --release` only when you need performance

### Getting Help

If you encounter issues:
1. Check [GitHub Issues](https://github.com/kujolang/kujo/issues)
2. Read the [Contributing Guide](CONTRIBUTING.md)
3. Open a new issue with:
   - Your OS and version
   - Rust version (`rustc --version`)
   - Full error message
   - Steps to reproduce

---

## Updating Kujo

### Built from Source
```bash
cd kujo
git pull
cargo build --release
# If installed system-wide:
sudo cp target/release/kujo /usr/local/bin/
```

### Via Package Manager (Future)
```bash
# Homebrew
brew upgrade kujo

# Scoop
scoop update kujo
```

---

## 🗑️ Uninstalling

### Cargo Install
```bash
cargo uninstall kujo
```

### Manual Installation
```bash
# macOS/Linux
sudo rm /usr/local/bin/kujo

# Windows - Delete kujo.exe from your installation directory
```

---

## Verification

After installation, verify everything works:

```bash
# Check version
kujo --version

# Run a test script
echo 'print("Hello, Kujo!")' > test.kujo
kujo run test.kujo

# Run test suite
cd /path/to/kujo/repo
kujo test
```

Expected output:
```
Hello, Kujo!
```

---

## Development Setup

For contributors and developers:

```bash
# Clone and setup
git clone https://github.com/kujolang/kujo.git
cd kujo

# Install dev dependencies
rustup component add rustfmt clippy

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy

# Build documentation
cargo doc --open
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development guidelines.

---

**You're ready to start coding in Kujo! 🐾**

*For examples and language features, see the [README](README.md) and [examples/](examples/) directory.*
