# Prompt: Verify Kujo Agent Installation in a Clean Container

You are validating the current Kujo Agent Project experience outside the developer's machine. Perform an evidence-producing clean-room test in Docker. Podman is an acceptable fallback. If neither is available, stop and report that the test is blocked; do not substitute the host environment.

## Rules

- Start from a fresh Ubuntu 22.04 x86_64 image.
- Do not mount the host home directory, Cargo caches, source checkout, SSH agent, credential stores, Docker socket, or existing Kujo installation.
- Do not copy binaries built on the host into the container.
- A dedicated empty evidence directory may be mounted read/write at `/evidence`.
- Keep secrets out of Dockerfiles, image layers, command arguments, shell history, logs, and evidence.
- Never use a Docker `ARG` or persistent `ENV` for an API key or GitHub token.
- Use BuildKit secrets or runtime-only secret files when private repository or live-provider access is required.
- Do not weaken tests or modify Kujo merely to obtain a pass.
- Remove test containers and secret mounts when finished. Preserve only redacted evidence.

## Phase 1: Prove the environment is isolated

Record the following in the report:

```bash
cat /etc/os-release
uname -a
id
mount
git --version
```

Confirm that no host home, tool cache, source directory, credential store, or Docker socket is mounted.

Install only the prerequisites needed to build the source installer:

```bash
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y \
  build-essential ca-certificates clang cmake curl git perl pkg-config
rm -rf /var/lib/apt/lists/*
```

## Phase 2: Install Kujo and the Agent toolchain

Install from the public default branch using the documented one-liner:

```bash
curl -fsSL https://raw.githubusercontent.com/kujolang/kujo/main/install.sh \
  | bash -s -- --source --ref main --group agent
export PATH="$HOME/.local/bin:$PATH"
```

If the repositories are private, provide `KUJO_GITHUB_TOKEN` only through a
BuildKit secret or a runtime-mounted secret file. Do not print it. Record the
exact resolved Kujo commit and confirm it contains the maintenance-warning
cleanup and compatibility commits `32e4ceb` and `4b49d52`:

```bash
git -C "$HOME/.local/share/kujo/source/kujo" merge-base --is-ancestor 32e4ceb HEAD
git -C "$HOME/.local/share/kujo/source/kujo" merge-base --is-ancestor 4b49d52 HEAD
```

If the installer uses a different source location, locate its recorded checkout
without consulting or mounting the host filesystem.

Verify the installed commands:

```bash
kujo --version
kennel --help
kujo agent --help
kujo doctor agent --help
```

A missing command or non-zero exit is a failure.

## Phase 3: Run the offline basic profile

This phase must not use an API key.

```bash
mkdir -p /workspace
cd /workspace
kujo agent new fixture-agent --profile basic --install --no-git
cd fixture-agent
kujo doctor agent --json
kujo agent inspect --json
kujo agent run --json "Reply with exactly: install verified"
kujo agent eval --json
```

Capture stdout, stderr, and exit status separately for every command. Validate that successful machine-readable output is JSON, no credential is requested, and all commands exit zero.

Build a second test image after installation, then repeat the fixture run in a new container started with `--network none`. This proves the normal offline execution path does not depend on hidden network access.

## Phase 4: Run the observable profile

From `/workspace`:

```bash
kujo agent new observable-agent --profile observable --install --no-git
cd observable-agent
kujo doctor agent --json
kujo agent inspect --json
kujo agent run --json "Reply with exactly: observable install verified"
kujo agent eval --json
```

Confirm that Watchdog and RunLedger are installed or resolved by `--install`, and that Agent Doctor reports them ready. If either is deliberately removed for a negative test, confirm Doctor fails clearly and gives the focused repair command:

```bash
bash install.sh --group agent
```

Restore the dependency and rerun Doctor before judging the observable profile.

## Phase 5: Prove Cargo audit is clean

Use a fresh source checkout inside the container, not a host mount:

```bash
git clone https://github.com/kujolang/kujo.git /src/kujo
cd /src/kujo
git rev-parse HEAD
cargo audit --deny warnings
cargo tree -i mach
cargo tree -i paste
```

Install `cargo-audit` inside the container if the source installer did not provide it. `cargo audit --deny warnings` must exit zero with no vulnerabilities and no maintenance warnings. The two inverse tree commands must report that no matching package exists. Also run:

```bash
cargo test --test release_dependency_advisory_contract
cargo check --all-features
```

Do not use advisory ignore flags or an audit configuration that suppresses warnings.

## Phase 6: Optional live-provider proof

Run this phase only when a real OpenAI key is explicitly supplied as a runtime secret. The offline and observable phases must pass independently first.

Create a new container from the installed test image. Mount the key as a read-only runtime secret at `/run/secrets/openai_api_key`; do not put it in the image or command line.

```bash
cd /workspace
kujo agent new live-agent --provider openai --install --no-git --no-credential
cd live-agent
kujo agent auth set openai --project --from-stdin < /run/secrets/openai_api_key
kujo agent auth status openai --json
kujo doctor agent --json
kujo agent run --json "Reply with exactly: clean container verified"
```

Confirm the status and command output never reveal the key. The project-scoped credential is acceptable only inside this disposable container; verify `.env.local` is ignored by Git and owner-readable only. Destroy the container after recording redacted results.

## Required report

Return a concise PASS/FAIL report containing:

- container engine/version, base-image digest, architecture, and isolation checks;
- exact Kujo commit tested;
- each command, exit status, and redacted evidence path;
- offline run result with network disabled;
- observable profile result, including Watchdog and RunLedger readiness;
- `cargo audit --deny warnings` result;
- optional live request result, or “not run—no secret supplied”;
- any failure with the first relevant error and a reproducible command;
- confirmation that no host binaries, caches, credentials, or source mounts were used.

Mark the overall result PASS only if installation, the offline basic profile, the observable profile, the audit gate, and the isolation checks all pass. The live-provider phase is optional and must be reported separately.
