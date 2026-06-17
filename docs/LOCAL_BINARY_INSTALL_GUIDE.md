# Kujo Local Binary Install Guide

This guide is for unsigned local/test Kujo binaries shared inside the team. These artifacts are useful for smoke testing before the official GitHub release pipeline is available.

## Artifact Files

Each platform artifact should be shared with its matching `.sha256` file:

| Platform | Artifact name pattern | Binary inside |
|---|---|---|
| macOS Intel | `kujo-v<version>-<date>-<commit>-macos-x64.tar.gz` | `kujo` |
| macOS Apple Silicon | `kujo-v<version>-<date>-<commit>-macos-arm64.tar.gz` | `kujo` |
| Linux x64 | `kujo-v<version>-<date>-<commit>-linux-x64.tar.gz` | `kujo` |
| Windows x64 | `kujo-v<version>-<date>-<commit>-windows-x64.zip` | `kujo.exe` |

## Verify The Download

Keep the archive and checksum file in the same directory.

macOS or Linux:

```bash
shasum -a 256 -c kujo-v<version>-<date>-<commit>-<platform>.tar.gz.sha256
```

Windows PowerShell:

```powershell
$archive = "kujo-v<version>-<date>-<commit>-windows-x64.zip"
$expected = (Get-Content "$archive.sha256").Split(" ")[0].ToUpper()
$actual = (Get-FileHash -Algorithm SHA256 $archive).Hash
if ($actual -eq $expected) { "OK" } else { "Checksum mismatch" }
```

## Install On macOS

Choose the right artifact:

- Intel Mac: `macos-x64`
- Apple Silicon Mac: `macos-arm64`

Extract:

```bash
tar -xzf kujo-v<version>-<date>-<commit>-macos-arm64.tar.gz
chmod +x kujo
```

Run from the current directory:

```bash
./kujo --version
./kujo run hello.kujo
```

Optional install into your user path:

```bash
mkdir -p "$HOME/.local/bin"
mv kujo "$HOME/.local/bin/kujo"
kujo --version
```

If macOS blocks the unsigned local binary, allow it in System Settings, or remove quarantine metadata:

```bash
xattr -dr com.apple.quarantine ./kujo
```

## Install On Linux

Extract:

```bash
tar -xzf kujo-v<version>-<date>-<commit>-linux-x64.tar.gz
chmod +x kujo
```

Run from the current directory:

```bash
./kujo --version
./kujo run hello.kujo
```

Optional install into your user path:

```bash
mkdir -p "$HOME/.local/bin"
mv kujo "$HOME/.local/bin/kujo"
kujo --version
```

## Install On Windows

Extract the `.zip` file in File Explorer, or with PowerShell:

```powershell
Expand-Archive .\kujo-v<version>-<date>-<commit>-windows-x64.zip -DestinationPath .\kujo-local -Force
```

Run from PowerShell:

```powershell
.\kujo-local\kujo.exe --version
.\kujo-local\kujo.exe run .\hello.kujo
```

Optional install into a user tools directory:

```powershell
New-Item -ItemType Directory -Force "$HOME\.local\bin"
Move-Item .\kujo-local\kujo.exe "$HOME\.local\bin\kujo.exe" -Force
```

Add `$HOME\.local\bin` to your user `PATH` if it is not already there.

## Smoke Test Script

Create `hello.kujo`:

```kujo
print("Kujo Kujo!")
```

Expected output:

```text
Kujo Kujo!
```

## Build Notes

For official release assets, prefer the GitHub Actions release matrix when it is available. It builds on native runners for:

- Linux x64
- macOS x64
- macOS ARM64
- Windows x64

Local cross-builds can be blocked by platform-native dependencies such as OpenSSL. If cross-building fails, build on the target platform or use the GitHub Actions matrix.

