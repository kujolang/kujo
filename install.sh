#!/usr/bin/env bash

set -Eeuo pipefail

readonly KUJO_INSTALL_VERSION="0.1.0"
readonly GITHUB_OWNER="${KUJO_GITHUB_OWNER:-kujolang}"
readonly DEFAULT_REF="${KUJO_ECOSYSTEM_REF:-main}"
readonly DEFAULT_RELEASE_VERSION="${KUJO_RELEASE_VERSION:-v1.0.1}"
readonly DEFAULT_PREFIX="${KUJO_INSTALL_ROOT:-${HOME}/.kujo}"
readonly DEFAULT_BIN_DIR="${KUJO_BIN_DIR:-${HOME}/.local/bin}"

REF="$DEFAULT_REF"
PREFIX="$DEFAULT_PREFIX"
BIN_DIR="$DEFAULT_BIN_DIR"
DRY_RUN=0
WITH_DEPS=0
INSTALL_SOURCE=0
SHOW_LIST=0
SELECTED_GROUPS="core operating"

usage() {
	cat <<'EOF'
Kujo ecosystem installer

Usage:
  bash install.sh [options]

The default profile installs Kujo, the local package foundation, the validated
agent context/proof tools, and the Kujo skills, agents, and workflow packs.

Profiles:
  --core                 Foundation and everyday agent-development tools (default)
  --group <name>         Add one profile: ai, quality, showcases, or operating
  --all                  Install every profile (the kitchen sink)
  --list                 Show the profile catalog and exit

Install options:
  --ref <ref>            Git ref for source repos (default: main)
  --prefix <dir>         Install root (default: ~/.kujo)
  --bin-dir <dir>        User command directory (default: ~/.local/bin)
  --source               Build Kujo from source instead of a release artifact
  --with-deps            Install optional Node dependencies for app/tool repos
  --dry-run              Print actions without downloading or writing files
  -h, --help             Show this help

Examples:
  curl -fsSL https://kujolang.ai/install.sh | bash
  curl -fsSL https://kujolang.ai/install.sh | bash -s -- --all
  bash install.sh --group ai --group quality
  bash install.sh --ref v1.0.1 --with-deps

Environment overrides:
  KUJO_ECOSYSTEM_REF, KUJO_RELEASE_VERSION, KUJO_INSTALL_ROOT, KUJO_BIN_DIR, KUJO_GITHUB_OWNER,
  KUJO_GITHUB_TOKEN (only needed while repositories are private)
EOF
}

profile_catalog() {
	cat <<'EOF'
core       kujo kennel spec eval scout scent packwrite runledger casefile patchbrief changebucket muzzle kujo-skills kujo-agents kujo-workflows
ai         ai-sdk agents-sdk dispatch watchdog mcp rag relay
quality    concord shipcheck fence redact lens tribunal workcell howl
showcases  ssg cms crud-api ai-chat intake site-kit
operating  kujo-skills kujo-agents kujo-workflows
EOF
}

list_catalog() {
	cat <<'EOF'
Kujo ecosystem profiles

core       Everyday local-first development: runtime, package management,
           task contracts, context, agent packs, proof, and run receipts.
ai         Provider and agent infrastructure: AI SDK, Agents SDK, Dispatch,
           Watchdog, MCP, RAG, and Relay.
quality    Architecture, release, privacy, browser, sandbox, governance,
           and evidence rendering tools.
showcases  Copyable applications and design/publishing surfaces.
operating  The skills, agent contracts, and runnable workflow collection.

The default is core plus operating. --all selects every profile.
Source repos are installed under the prefix; only repos with a stable launcher
receive a command shim in the bin directory.

Catalog entries:
EOF
	profile_catalog | sed 's/^/  /'
}

die() {
	echo "install.sh: $*" >&2
	exit 1
}

log() {
	echo "[kujo-install] $*"
}

require_command() {
	command -v "$1" >/dev/null 2>&1 || die "required command not found: $1"
}

github_curl() {
	if [[ -n "${KUJO_GITHUB_TOKEN:-}" ]]; then
		curl -fsSL --retry 3 -H "Authorization: Bearer $KUJO_GITHUB_TOKEN" "$@"
	else
		curl -fsSL --retry 3 "$@"
	fi
}

has_group() {
	case " $SELECTED_GROUPS " in
		*" $1 "*) return 0 ;;
		*) return 1 ;;
	esac
}

select_all() {
	SELECTED_GROUPS="core ai quality showcases operating"
}

repo_dir() {
	echo "$PREFIX/sources/$1"
}

fetch_repo() {
	local repo="$1"
	local destination
	local temp_dir
	local archive
	local extracted
	local marker
	local archive_url

	destination="$(repo_dir "$repo")"
	marker="$destination/.kujo-install-ref"
	if [[ -f "$marker" ]] && [[ "$(<"$marker")" == "$REF" ]]; then
		log "$repo already installed at ref $REF"
		return 0
	fi

	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "would download $GITHUB_OWNER/$repo at $REF"
		return 0
	fi

	temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/kujo-install.XXXXXX")"
	trap 'rm -rf "$temp_dir"' RETURN
	archive="$temp_dir/source.tar.gz"

	log "downloading $GITHUB_OWNER/$repo at $REF"
	case "$REF" in
		refs/*) archive_url="https://codeload.github.com/$GITHUB_OWNER/$repo/tar.gz/$REF" ;;
		v[0-9]*|[0-9]*.[0-9]*) archive_url="https://codeload.github.com/$GITHUB_OWNER/$repo/tar.gz/refs/tags/$REF" ;;
		[0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F][0-9a-fA-F]*) archive_url="https://codeload.github.com/$GITHUB_OWNER/$repo/tar.gz/$REF" ;;
		*) archive_url="https://codeload.github.com/$GITHUB_OWNER/$repo/tar.gz/refs/heads/$REF" ;;
	esac
	github_curl "$archive_url" -o "$archive"
	tar -xzf "$archive" -C "$temp_dir"
	extracted="$(find "$temp_dir" -mindepth 1 -maxdepth 1 -type d -print -quit)"
	[[ -n "$extracted" ]] || die "GitHub archive for $repo did not contain a source directory"

	mkdir -p "$(dirname "$destination")"
	if [[ -e "$destination" || -L "$destination" ]]; then
		mv "$destination" "${destination}.previous.$(date +%Y%m%d%H%M%S)"
	fi
	mv "$extracted" "$destination"
	printf '%s\n' "$REF" > "$marker"
	rm -rf "$temp_dir"
	trap - RETURN
}

verify_sha256() {
	local archive="$1"
	local checksum_file="$2"
	local expected
	local actual

	expected="$(awk '{print $1}' "$checksum_file")"
	[[ "$expected" =~ ^[0-9a-fA-F]{64}$ ]] || die "invalid SHA-256 checksum file: $checksum_file"
	if command -v sha256sum >/dev/null 2>&1; then
		actual="$(sha256sum "$archive" | awk '{print $1}')"
	else
		actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
	fi
	[[ "$actual" == "$expected" ]] || die "checksum mismatch for Kujo release artifact"
}

install_kujo_from_release() {
	local version="$1"
	local os
	local arch
	local target
	local archive_name
	local base_url
	local temp_dir
	local archive
	local checksum_file
	local extracted

	case "$(uname -s)" in
		Darwin) os="macos" ;;
		Linux) os="linux" ;;
		*) return 1 ;;
	esac
	case "$(uname -m)" in
		x86_64|amd64) arch="x64" ;;
		arm64|aarch64) arch="arm64" ;;
		*) return 1 ;;
	esac
	target="$os-$arch"
	archive_name="kujo-$version-$target.tar.gz"
	base_url="https://github.com/$GITHUB_OWNER/kujo/releases/download/$version"
	temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/kujo-release.XXXXXX")"
	trap 'rm -rf "$temp_dir"' RETURN
	archive="$temp_dir/$archive_name"
	checksum_file="$archive.sha256"

	if ! github_curl "$base_url/$archive_name" -o "$archive"; then
		rm -rf "$temp_dir"
		trap - RETURN
		return 1
	fi
	github_curl "$base_url/$archive_name.sha256" -o "$checksum_file" || die "release artifact downloaded but checksum is unavailable"
	verify_sha256 "$archive" "$checksum_file"
	tar -xzf "$archive" -C "$temp_dir"
	extracted="$temp_dir/kujo"
	# The archive is only staged here. Do not require execution permission on the
	# temporary directory: hardened sandbox runners commonly mount /tmp with
	# noexec, even though the final user bin directory is executable.
	[[ -f "$extracted" ]] || die "Kujo release artifact did not contain a file named kujo"
	mkdir -p "$BIN_DIR"
	cp "$extracted" "$BIN_DIR/kujo"
	chmod 755 "$BIN_DIR/kujo"
	rm -rf "$temp_dir"
	trap - RETURN
	log "installed Kujo $version ($target) to $BIN_DIR/kujo"
}

install_kujo_from_source() {
	local source_dir
	source_dir="$(repo_dir kujo)"
	fetch_repo kujo
	require_command cargo
	log "building Kujo from source at ref $REF"
	if [[ "$DRY_RUN" -eq 1 ]]; then
		return 0
	fi
	(
		cd "$source_dir"
		cargo build --release --locked
	)
	mkdir -p "$BIN_DIR"
	cp "$source_dir/target/release/kujo" "$BIN_DIR/kujo"
	chmod 755 "$BIN_DIR/kujo"
	log "installed source-built Kujo to $BIN_DIR/kujo"
}

install_kujo() {
	if [[ "$DRY_RUN" -eq 1 ]]; then
		if [[ "$INSTALL_SOURCE" -eq 1 ]]; then
			log "would build Kujo from source at ref $REF"
		elif [[ "$REF" == "main" ]]; then
			log "would download and verify Kujo release $DEFAULT_RELEASE_VERSION"
		else
			log "would download and verify Kujo release $REF"
		fi
		return 0
	fi
	if [[ "$INSTALL_SOURCE" -eq 1 ]]; then
		install_kujo_from_source
	elif [[ "$REF" == "main" ]]; then
		install_kujo_from_release "$DEFAULT_RELEASE_VERSION"
	elif ! install_kujo_from_release "$REF"; then
		log "release artifact unavailable for $REF; falling back to a source build"
		install_kujo_from_source
	fi
}

make_kujo_shim() {
	local command_name="$1"
	local repo="$2"
	local entrypoint="$3"
	local mode="$4"
	local shim="$BIN_DIR/$command_name"
	local kujo_bin_literal
	local source_literal
	local entrypoint_literal

	kujo_bin_literal="$(printf '%q' "$BIN_DIR/kujo")"
	source_literal="$(printf '%q' "$(repo_dir "$repo")")"
	entrypoint_literal="$(printf '%q' "$entrypoint")"
	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "would install command shim $shim"
		return 0
	fi
	mkdir -p "$BIN_DIR"
	{
		printf '%s\n' '#!/usr/bin/env bash' 'set -Eeuo pipefail'
		printf 'exec %s run %s' "$kujo_bin_literal" "$source_literal/$entrypoint_literal"
		if [[ "$mode" == "interpreter" ]]; then
			printf '%s' ' --interpreter'
		fi
		printf '%s\n' ' -- "$@"'
	} > "$shim"
	chmod 755 "$shim"
}

make_shell_shim() {
	local command_name="$1"
	local repo="$2"
	local entrypoint="$3"
	local shim="$BIN_DIR/$command_name"
	local source_literal
	local entrypoint_literal

	source_literal="$(printf '%q' "$(repo_dir "$repo")")"
	entrypoint_literal="$(printf '%q' "$entrypoint")"
	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "would install command shim $shim"
		return 0
	fi
	mkdir -p "$BIN_DIR"
	{
		printf '%s\n' '#!/usr/bin/env bash' 'set -Eeuo pipefail'
		printf 'cd %s\nexec %s "$@"\n' "$source_literal" "$entrypoint_literal"
	} > "$shim"
	chmod 755 "$shim"
}

install_entry() {
	local group="$1"
	local repo="$2"
	local kind="$3"
	local entrypoint="$4"
	local command_name="$5"
	local mode="$6"

	has_group "$group" || return 0
	if [[ "$repo" == "kujo" ]]; then
		return 0
	fi
	fetch_repo "$repo"
	case "$kind" in
		kujo) make_kujo_shim "$command_name" "$repo" "$entrypoint" "$mode" ;;
		shell) make_shell_shim "$command_name" "$repo" "$entrypoint" ;;
		none) log "$repo source installed (library/content repo; no command shim)" ;;
		*) die "unknown catalog entry kind: $kind" ;;
	 esac
}

install_optional_dependencies() {
	local repo="$1"
	local source_dir
	source_dir="$(repo_dir "$repo")"
	[[ "$WITH_DEPS" -eq 1 ]] || return 0
	[[ "$DRY_RUN" -eq 0 ]] || { log "would install optional dependencies for $repo"; return 0; }
	case "$repo" in
		ai-chat|intake|site-kit)
			require_command npm
			if [[ -f "$source_dir/package-lock.json" ]]; then
				(cd "$source_dir" && npm ci)
			else
				(cd "$source_dir" && npm install)
			fi
			;;
		crud-api)
			require_command npm
			if [[ -f "$source_dir/frontend/package-lock.json" ]]; then
				(cd "$source_dir/frontend" && npm ci)
			else
				(cd "$source_dir/frontend" && npm install)
			fi
			;;
		*) return 0 ;;
	 esac
}

write_install_record() {
	local record="$PREFIX/install.json"
	if [[ "$DRY_RUN" -eq 1 ]]; then
		log "would write $record"
		return 0
	fi
	mkdir -p "$PREFIX"
	{
		printf '{\n'
		printf '  "installer_version": "%s",\n' "$KUJO_INSTALL_VERSION"
		printf '  "ref": "%s",\n' "$REF"
		printf '  "prefix": "%s",\n' "$PREFIX"
		printf '  "bin_dir": "%s",\n' "$BIN_DIR"
		printf '  "groups": ['
		printf '%s' "$SELECTED_GROUPS" | awk '{for (i = 1; i <= NF; i++) { if (i > 1) printf ", "; printf "\"%s\"", $i }}'
		printf '],\n'
		printf '  "with_deps": %s\n' "$([[ "$WITH_DEPS" -eq 1 ]] && echo true || echo false)"
		printf '}\n'
	} > "$record"
}

parse_args() {
	while [[ "$#" -gt 0 ]]; do
		case "$1" in
			--core) SELECTED_GROUPS="core operating" ;;
			--group)
				[[ "$#" -ge 2 ]] || die "--group requires a value"
				case "$2" in
					core|ai|quality|showcases|operating) ;;
					*) die "unknown group: $2" ;;
				 esac
				[[ " $SELECTED_GROUPS " == *" $2 "* ]] || SELECTED_GROUPS="$SELECTED_GROUPS $2"
				shift
				;;
			--all|--kitchen-sink) select_all ;;
			--ref)
				[[ "$#" -ge 2 ]] || die "--ref requires a value"
				REF="$2"
				shift
				;;
			--prefix)
				[[ "$#" -ge 2 ]] || die "--prefix requires a value"
				PREFIX="$2"
				shift
				;;
			--bin-dir)
				[[ "$#" -ge 2 ]] || die "--bin-dir requires a value"
				BIN_DIR="$2"
				shift
				;;
			--source) INSTALL_SOURCE=1 ;;
			--with-deps) WITH_DEPS=1 ;;
			--dry-run) DRY_RUN=1 ;;
			--list) SHOW_LIST=1 ;;
			-h|--help) usage; exit 0 ;;
			*) die "unknown option: $1" ;;
		 esac
		shift
	done
}

main() {
	local group
	local repo
	local kind
	local entrypoint
	local command_name
	local mode

	parse_args "$@"
	if [[ "$SHOW_LIST" -eq 1 ]]; then
		list_catalog
		return 0
	fi
	require_command curl
	require_command tar
	require_command awk
	install_kujo

	while IFS='|' read -r group repo kind entrypoint command_name mode; do
		[[ -n "$group" ]] || continue
		install_entry "$group" "$repo" "$kind" "$entrypoint" "$command_name" "$mode"
		if has_group "$group"; then
			install_optional_dependencies "$repo"
		fi
	done <<'EOF'
core|kujo|none|||
core|kennel|kujo|kennel.kujo|kennel|interpreter
core|spec|shell|scripts/spec|spec|
core|eval|kujo|main.kujo|eval|
core|scout|kujo|scout.kujo|scout|
core|scent|kujo|scent.kujo|scent|
core|packwrite|shell|bin/packwrite|packwrite|
core|runledger|shell|bin/runledger|runledger|
core|casefile|kujo|casefile.kujo|casefile|
core|patchbrief|kujo|patchbrief.kujo|patchbrief|
core|changebucket|shell|bin/changebucket|changebucket|
core|muzzle|kujo|muzzle.kujo|muzzle|
operating|kujo-skills|none|||
operating|kujo-agents|none|||
operating|kujo-workflows|none|||
ai|ai-sdk|none|||
ai|agents-sdk|none|||
ai|dispatch|kujo|dispatch.kujo|dispatch|
ai|watchdog|kujo|watchdog.kujo|watchdog|
ai|mcp|kujo|mcp.kujo|mcp|
ai|rag|kujo|main.kujo|rag|
ai|relay|shell|bin/relay|relay|
quality|concord|kujo|concord.kujo|concord|
quality|shipcheck|kujo|shipcheck.kujo|shipcheck|
quality|fence|shell|fence.sh|fence|
quality|redact|kujo|redact.kujo|redact|
quality|lens|shell|lens|lens|
quality|tribunal|shell|bin/tribunal|tribunal|
quality|workcell|shell|bin/workcell|workcell|
quality|howl|shell|bin/howl|howl|
showcases|ssg|none|||
showcases|cms|none|||
showcases|crud-api|none|||
showcases|ai-chat|none|||
showcases|intake|shell|bin/intake.js|intake|
showcases|site-kit|none|||
EOF

	write_install_record
	cat <<EOF

Kujo ecosystem installation complete.
  Root:     $PREFIX
  Commands: $BIN_DIR
  Profiles: $SELECTED_GROUPS
  Ref:      $REF

If $BIN_DIR is not already on PATH, run:
  export PATH="$BIN_DIR:\$PATH"

Point an agent at the ecosystem guide:
  https://github.com/$GITHUB_OWNER/kujo/blob/main/docs/ECOSYSTEM_INSTALL.md
EOF
}

main "$@"
