# Release Binaries

This document explains how Kujo release binaries are built, named, published,
and installed.

## Supported Release Assets

For tag `<TAG>` such as `v1.0.0`, the release workflow publishes these assets:

| Platform | Runner | Archive | Binary inside |
| --- | --- | --- | --- |
| Linux x64 | `ubuntu-latest` | `kujo-<TAG>-linux-x64.tar.gz` | `kujo` |
| macOS Intel | `macos-15-intel` | `kujo-<TAG>-macos-x64.tar.gz` | `kujo` |
| macOS Apple Silicon | `macos-15` | `kujo-<TAG>-macos-arm64.tar.gz` | `kujo` |
| Windows x64 | `windows-latest` | `kujo-<TAG>-windows-x64.zip` | `kujo.exe` |

Each archive has a matching SHA-256 file:

```text
<archive>.sha256
```

The release also includes a consolidated `checksums.txt`.

## How The Workflow Runs

The release workflow lives at:

```text
.github/workflows/release-binaries.yml
```

It runs in two modes:

- automatically when a `v*` tag is pushed
- manually from GitHub Actions with `workflow_dispatch`

For a tag release, the workflow:

1. runs the release gate on Ubuntu
2. builds optimized binaries with `cargo build --release --locked`
3. smoke-tests each binary with:
   - `kujo --version`
   - `kujo run examples/hello.kujo`
   - `kujo lsp --help`
4. packages each platform archive
5. writes per-archive SHA-256 files
6. publishes the GitHub release, generated release notes, and release assets when the ref is a tag
7. dispatches the published-artifact smoke workflow for that tag

Manual runs upload artifacts to the workflow run, but they do not publish GitHub
release assets unless the run is for a `v*` tag.

## Manual Dry Run

Use this before a release tag when you want to verify the artifact matrix.

1. Open the Kujo repo in GitHub.
2. Go to **Actions**.
3. Select **release-binaries**.
4. Choose **Run workflow**.
5. Optionally set `version`, for example `v1.0.0-rc-dry-run`.
6. Download the uploaded artifacts from the completed workflow run.

The manual artifact names use the supplied version value, or `manual-<run number>`
when no value is supplied.

## Installing A Binary

Download the asset for your platform from the GitHub release page.

macOS Apple Silicon:

```bash
KUJO_VERSION="v1.0.0"
ARCHIVE="kujo-${KUJO_VERSION}-macos-arm64.tar.gz"
BASE_URL="https://github.com/kujolang/kujo/releases/download/${KUJO_VERSION}"

curl -sSfL "${BASE_URL}/${ARCHIVE}" -o "${ARCHIVE}"
curl -sSfL "${BASE_URL}/${ARCHIVE}.sha256" -o "${ARCHIVE}.sha256"
shasum -a 256 -c "${ARCHIVE}.sha256"

mkdir -p ~/.local/bin
tar -xzf "${ARCHIVE}"
cp kujo ~/.local/bin/kujo
chmod +x ~/.local/bin/kujo
~/.local/bin/kujo --version
```

Linux x64:

```bash
KUJO_VERSION="v1.0.0"
ARCHIVE="kujo-${KUJO_VERSION}-linux-x64.tar.gz"
BASE_URL="https://github.com/kujolang/kujo/releases/download/${KUJO_VERSION}"

curl -sSfL "${BASE_URL}/${ARCHIVE}" -o "${ARCHIVE}"
curl -sSfL "${BASE_URL}/${ARCHIVE}.sha256" -o "${ARCHIVE}.sha256"
sha256sum -c "${ARCHIVE}.sha256"

mkdir -p ~/.local/bin
tar -xzf "${ARCHIVE}"
cp kujo ~/.local/bin/kujo
chmod +x ~/.local/bin/kujo
~/.local/bin/kujo --version
```

Windows x64 PowerShell:

```powershell
$KujoVersion = "v1.0.0"
$Archive = "kujo-$KujoVersion-windows-x64.zip"
$BaseUrl = "https://github.com/kujolang/kujo/releases/download/$KujoVersion"

Invoke-WebRequest "$BaseUrl/$Archive" -OutFile $Archive
Invoke-WebRequest "$BaseUrl/$Archive.sha256" -OutFile "$Archive.sha256"

$Expected = (Get-Content "$Archive.sha256").Split(" ")[0].ToLower()
$Actual = (Get-FileHash -Algorithm SHA256 $Archive).Hash.ToLower()
if ($Expected -ne $Actual) { throw "Checksum mismatch" }

New-Item -ItemType Directory -Force "$HOME\.kujo\bin" | Out-Null
Expand-Archive $Archive -DestinationPath "$HOME\.kujo\bin" -Force
& "$HOME\.kujo\bin\kujo.exe" --version
```

You can run Kujo by full path, from the extracted directory, or by placing the
binary directory on your `PATH`.

## Post-Publish Smoke Test

The published release assets are verified by:

```text
.github/workflows/release-published-artifact-smoke.yml
```

That workflow downloads each published archive/checksum pair from the release,
verifies the checksum, extracts the binary, and runs the same command smoke
checks used during packaging.

## Notes For Maintainers

- Keep asset names stable. Onboarding bundles, install docs, and downstream
  automation should be able to derive the archive name from `<TAG>` and platform.
- Use `.tar.gz` for Unix-like platforms and `.zip` for Windows.
- If a new platform is added, update this document, `INSTALLATION.md`,
  `docs/INSTALL_MATRIX.md`, `docs/RELEASE_ARTIFACT_VALIDATION.md`, and
  `docs/RELEASE_ARTIFACT_CHECKLIST_V1_0_0.md` in the same change.
