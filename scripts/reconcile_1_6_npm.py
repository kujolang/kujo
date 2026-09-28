"""Release-only artifact reconciliation; builds nothing and publishes nothing."""
import hashlib
import json
from pathlib import Path
import shutil
import tarfile
import urllib.request

SOURCE = "44af277848173664f72ca85f2a1b3b98d634ecdd"
TARGETS = {"darwin-x64", "darwin-arm64", "linux-x64", "linux-arm64", "win32-x64"}
PINNED = {
    "kujolang-kujo-darwin-x64-1.6.0.tgz": "2375c8a644047821a5137a03a5a05cadb0f7b56c88001a8c4cd6ce89b232c258",
    "kujolang-kujo-runtime-1.6.0.tgz": "b62ea8a8e0f2e113f721a25eb873cd5f224b4ad86128e25b09d3a1b21fc551bc",
}


def main():
    out = Path("reviewed")
    out.mkdir(exist_ok=False)
    packages = list(Path("incoming").rglob("*.tgz"))
    expected = {f"kujolang-kujo-{t}-1.6.0.tgz" for t in TARGETS} | {
        "kujolang-kujo-runtime-1.6.0.tgz"
    }
    assert len(packages) == 6 and {p.name for p in packages} == expected
    for p in packages:
        shutil.copyfile(p, out / p.name)
    for name, digest in PINNED.items():
        data = urllib.request.urlopen(
            "https://github.com/kujolang/kujo/releases/download/v1.6.0/" + name,
            timeout=120,
        ).read()
        assert hashlib.sha256(data).hexdigest() == digest, name
        (out / name).write_bytes(data)
    records = []
    for p in sorted(out.glob("*.tgz")):
        with tarfile.open(p, "r:gz") as archive:
            package = json.load(archive.extractfile("package/package.json"))
            assert package["version"] == "1.6.0"
            assert not {"preinstall", "install", "postinstall"} & set(package.get("scripts", {}))
            metadata = None
            if package["name"] == "@kujolang/kujo-runtime":
                assert p.name == "kujolang-kujo-runtime-1.6.0.tgz"
                assert set(package["optionalDependencies"]) == {f"@kujolang/kujo-{t}" for t in TARGETS}
                assert set(package["optionalDependencies"].values()) == {"1.6.0"}
            else:
                metadata = json.load(archive.extractfile("package/metadata.json"))
                target = metadata["target"]
                assert target in TARGETS
                assert package["name"] == f"@kujolang/kujo-{target}"
                assert p.name == f"kujolang-kujo-{target}-1.6.0.tgz"
                assert metadata["gitCommit"] == SOURCE
                assert metadata["runtimeVersion"] == "1.6.0"
                binary = archive.extractfile("package/bin/kujo.exe" if target == "win32-x64" else "package/bin/kujo").read()
                assert hashlib.sha256(binary).hexdigest() == metadata["sha256"]
            records.append({"file": p.name, "sha256": hashlib.sha256(p.read_bytes()).hexdigest(), "package": package["name"], "metadata": metadata})
    (out / "manifest.json").write_text(json.dumps({"source_commit": SOURCE, "packages": records, "reviewed_bytes_retained": PINNED}, indent=2) + "\n")


if __name__ == "__main__":
    main()
