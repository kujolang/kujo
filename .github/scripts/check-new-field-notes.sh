#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

base_sha="${1:-${BASE_SHA:-}}"
head_sha="${2:-${HEAD_SHA:-HEAD}}"

if [[ -z "$base_sha" || "$base_sha" == "0000000000000000000000000000000000000000" ]]; then
	if git rev-parse HEAD^ >/dev/null 2>&1; then
		base_sha="$(git rev-parse HEAD^)"
	else
		echo "[field-notes] No base commit available; skipping new-session-note validation"
		exit 0
	fi
fi

if ! git rev-parse "$base_sha" >/dev/null 2>&1; then
	echo "[field-notes] ERROR: base SHA is not available locally: $base_sha"
	exit 1
fi

if ! git rev-parse "$head_sha" >/dev/null 2>&1; then
	echo "[field-notes] ERROR: head SHA is not available locally: $head_sha"
	exit 1
fi

session_note_pattern='^notes/[0-9]{4}-[0-9]{2}-[0-9]{2}_[0-9]{2}-[0-9]{2}_[a-z0-9][a-z0-9-]*\.md$'

added_notes=()
while IFS= read -r changed_path; do
	if [[ -z "$changed_path" ]]; then
		continue
	fi

	if [[ "$changed_path" == notes/* ]]; then
		added_notes+=("$changed_path")
	fi
done < <(git diff --name-only --diff-filter=A "$base_sha" "$head_sha")

invalid_note_paths=()
for note_path in "${added_notes[@]}"; do
	if [[ "$note_path" == "notes/GOTCHAS.md" || "$note_path" == "notes/README.md" || "$note_path" == "notes/FIELD_NOTES_SYSTEM.md" ]]; then
		continue
	fi

	if [[ ! "$note_path" =~ $session_note_pattern ]]; then
		invalid_note_paths+=("$note_path")
	fi
done

if (( ${#invalid_note_paths[@]} > 0 )); then
	echo "[field-notes] ERROR: new notes files must use session-note filename format YYYY-MM-DD_HH-mm_short-kebab-summary.md"
	for invalid_path in "${invalid_note_paths[@]}"; do
		echo "[field-notes] Invalid notes path: $invalid_path"
	done
	exit 1
fi

session_notes=()
for note_path in "${added_notes[@]}"; do
	if [[ "$note_path" =~ $session_note_pattern ]]; then
		session_notes+=("$note_path")
	fi
done

if (( ${#session_notes[@]} == 0 )); then
	echo "[field-notes] No new session-note files in range ${base_sha}..${head_sha}"
	exit 0
fi

require_regex() {
	local file_path="$1"
	local pattern="$2"
	local label="$3"
	if ! grep -Eq "$pattern" "$file_path"; then
		echo "[field-notes] ERROR: ${file_path} missing required ${label}"
		exit 1
	fi
}

line_of_literal() {
	local file_path="$1"
	local literal="$2"
	grep -nF -- "$literal" "$file_path" | head -n1 | cut -d: -f1
}

require_literal() {
	local file_path="$1"
	local literal="$2"
	local label="$3"
	if ! grep -Fq "$literal" "$file_path"; then
		echo "[field-notes] ERROR: ${file_path} missing required section ${label}: ${literal}"
		exit 1
	fi
}

for session_note in "${session_notes[@]}"; do
	require_regex "$session_note" '^# Kujo Field Notes (—|-) .+$' "title line"
	require_regex "$session_note" '^\*\*Date:\*\* [0-9]{4}-[0-9]{2}-[0-9]{2}$' "date metadata"
	require_regex "$session_note" '^\*\*Session:\*\* [0-9]{2}:[0-9]{2}( local)?$' "session metadata"
	require_regex "$session_note" '^\*\*Branch/Commit:\*\* .+ / .+$' "branch/commit metadata"
	require_regex "$session_note" '^\*\*Scope:\*\* .+$' "scope metadata"

	require_literal "$session_note" '## What I Changed' 'What I Changed'
	require_literal "$session_note" '## Gotchas (Read This Next Time)' 'Gotchas'
	require_literal "$session_note" '## Things I Learned' 'Things I Learned'
	require_literal "$session_note" '## Debug Notes (Only if applicable)' 'Debug Notes'
	require_literal "$session_note" '## Follow-ups / TODO (For Future Agents)' 'Follow-ups / TODO'
	require_literal "$session_note" '## Links / References' 'Links / References'

	title_line="$(grep -nE '^# Kujo Field Notes (—|-) .+$' "$session_note" | head -n1 | cut -d: -f1)"
	date_line="$(grep -nE '^\*\*Date:\*\* [0-9]{4}-[0-9]{2}-[0-9]{2}$' "$session_note" | head -n1 | cut -d: -f1)"
	session_line="$(grep -nE '^\*\*Session:\*\* [0-9]{2}:[0-9]{2}( local)?$' "$session_note" | head -n1 | cut -d: -f1)"
	branch_line="$(grep -nE '^\*\*Branch/Commit:\*\* .+ / .+$' "$session_note" | head -n1 | cut -d: -f1)"
	scope_line="$(grep -nE '^\*\*Scope:\*\* .+$' "$session_note" | head -n1 | cut -d: -f1)"
	separator_line="$(line_of_literal "$session_note" '---')"
	changed_line="$(line_of_literal "$session_note" '## What I Changed')"
	gotchas_line="$(line_of_literal "$session_note" '## Gotchas (Read This Next Time)')"
	learned_line="$(line_of_literal "$session_note" '## Things I Learned')"
	debug_line="$(line_of_literal "$session_note" '## Debug Notes (Only if applicable)')"
	todo_line="$(line_of_literal "$session_note" '## Follow-ups / TODO (For Future Agents)')"
	links_line="$(line_of_literal "$session_note" '## Links / References')"

	ordered_lines=(
		"$title_line"
		"$date_line"
		"$session_line"
		"$branch_line"
		"$scope_line"
		"$separator_line"
		"$changed_line"
		"$gotchas_line"
		"$learned_line"
		"$debug_line"
		"$todo_line"
		"$links_line"
	)

	for idx in "${!ordered_lines[@]}"; do
		line_value="${ordered_lines[$idx]}"
		if [[ -z "$line_value" ]]; then
			echo "[field-notes] ERROR: could not compute required section ordering in ${session_note}"
			exit 1
		fi

		if (( idx > 0 )); then
			prev_line="${ordered_lines[$((idx - 1))]}"
			if (( line_value <= prev_line )); then
				echo "[field-notes] ERROR: required sections are out of order in ${session_note}"
				exit 1
			fi
		fi
	done

	echo "[field-notes] OK: ${session_note} follows required template contract"
done

echo "[field-notes] OK: validated ${#session_notes[@]} new session-note file(s)"
