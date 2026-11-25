"""Run matched Kujo CLI benchmarks against two already-built executables."""
import argparse
import json
import resource
import statistics
import subprocess
import tempfile
from pathlib import Path

ARRAY = "let input := range(16000)"
DICT_DATA = {f"content-entry-with-a-shared-prefix-{n}": n for n in range(2048)}
DICT_JSON = json.dumps(DICT_DATA)
DICT_PARTS = ", ".join(
    json.dumps(DICT_JSON[start:start + 4096]) for start in range(0, len(DICT_JSON), 4096)
)
DICT = f'let input := parse_json(join([{DICT_PARTS}], ""))'
UNICODE = 'let text := repeat("é🙂ab", 16384)'
ASCII = 'let text := repeat("abcdefgh", 8192)'
SLUG = 'let text := repeat(" Éclair___ !ΟΣ / Word____ ", 2048)'
CASES = [
    ("insert", ARRAY, "insert(input, 1000, 42)[1000]", 220, 42),
    ("remove-miss", ARRAY, "len(remove(input, -1))", 220, 16000),
    ("remove-hit", ARRAY, "remove(input, 42)[42]", 220, 43),
    ("remove_at", ARRAY, "remove_at(input, 1000)[0][1000]", 220, 1001),
    ("values", DICT, "values(input)[2]", 45, 10),
    ("items", DICT, "items(input)[2][1]", 45, 10),
    ("substring", UNICODE, "byte_length(substring(text, 500, 60000))", 220, 119000),
    ("truncate", UNICODE, 'byte_length(truncate(text, 24, "…"))', 600, 50),
    ("truncate-unchanged", UNICODE, 'byte_length(truncate(text, 131072, "…"))', 600, 131072),
    ("char_at", ASCII, 'byte_length(char_at(text, 64000))', 1400, 1),
    ("char_at-short", 'let text := "Kujoé"', "byte_length(char_at(text, 4))", 4000, 2),
    ("slugify", SLUG, "byte_length(slugify(text))", 140, 43005),
]


def cpu_seconds():
    usage = resource.getrusage(resource.RUSAGE_CHILDREN)
    return usage.ru_utime + usage.ru_stime


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binaries", metavar="KUJO", nargs=2, type=Path)
    parser.add_argument("--case", choices=[name for name, *_ in CASES], action="append")
    args = parser.parse_args()
    binaries = [binary.resolve() for binary in args.binaries]
    with tempfile.TemporaryDirectory(prefix="kujo-builtin-bench-") as directory:
        for name, setup, expression, count, expected in CASES:
            if args.case and name not in args.case:
                continue
            path = Path(directory) / f"{name}.kujo"
            path.write_text(f"""{setup}
mut total := 0
for iteration in range({count}) {{
    total += {expression}
}}
print(total)
""", encoding="utf-8")
            samples = [[] for _ in binaries]
            expected_stderr = None
            for trial in range(6):
                order = range(len(binaries))
                for index in order if trial % 2 else reversed(order):
                    start = cpu_seconds()
                    result = subprocess.run(
                        [binaries[index], "run", path], capture_output=True, check=True
                    )
                    elapsed_ms = 1000 * (cpu_seconds() - start)
                    assert result.stdout == f"{count * expected}\n".encode(), (name, result.stdout)
                    if expected_stderr is None:
                        expected_stderr = result.stderr
                    assert result.stderr == expected_stderr, (name, result.stderr)
                    if trial:
                        samples[index].append(round(elapsed_ms, 2))
            print(json.dumps({
                "case": name,
                "samples_cpu_ms": samples,
                "medians_cpu_ms": [statistics.median(run) for run in samples],
            }), flush=True)


if __name__ == "__main__":
    main()
